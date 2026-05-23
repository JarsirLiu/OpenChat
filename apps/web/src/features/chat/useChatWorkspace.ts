import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import {
  applyThreadStreamEvent,
  appendOptimisticTurn,
  createInitialChatRuntimeV2State,
  hydrateChatRuntimeV2State,
} from '@openchat/chat-core'
import { type ChatStreamEvent, type ThreadTurn } from '@openchat/protocol'
import type { AuthUser } from '../../lib/auth'
import { authenticatedFetch } from '../../lib/auth'
import { writeLocalApiCache } from '../../lib/localApiCache'
import { ApiError, API_ERROR_CODES, CLIENT_ERROR_CODES, ensureOk, toApiError } from '../../lib/apiError'
import { buildChatRequest } from './request'
import {
  createSessionId,
  ensureSessionPreference,
  updateSessionPreference,
} from './sessionPreferences'
import { sessionDetailCacheKey, useChatStream } from './useChatStream'
import {
  filterSupportedAttachmentFiles,
  filterSupportedImageFiles,
  getUnsupportedImageMessage,
} from './imageUpload'
import { compressImageFiles } from './imageCompression'
import { useModelCatalog } from './useModelCatalog'
import { useSessions } from './useSessions'
import { createShanghaiTimestamp } from './timestamps'
import type { UploadedImageAttachment } from './types'

interface UseChatWorkspaceParams {
  currentUser: AuthUser
  onUnauthorized: () => void
  activeSessionId: string | null
  onOpenSession: (sessionId: string) => void
  onOpenNewSession: () => void
}

interface OptimisticUserPreview {
  sessionId: string
  text: string
  createdAt: string
}

const DRAFT_SESSION_ID = '__draft__'

const sortTurnsByStartedAt = (turns: ThreadTurn[]) =>
  [...turns].sort((left, right) => {
    const leftAt = Date.parse(left.startedAt ?? '') || 0
    const rightAt = Date.parse(right.startedAt ?? '') || 0
    return leftAt - rightAt
  })

const mergeHydratedTurnsWithLocal = (
  hydratedTurns: ThreadTurn[],
  localTurns: ThreadTurn[],
  activeSessionId: string,
) => {
  const mergedById = new Map(hydratedTurns.map((turn) => [turn.id, turn] as const))

  for (const localTurn of localTurns) {
    if (localTurn.sessionId !== activeSessionId) {
      continue
    }

    const hydratedTurn = mergedById.get(localTurn.id)
    if (!hydratedTurn) {
      const shouldPreserveLocalOnlyTurn =
        localTurn.status === 'running' ||
        localTurn.items.some((item) => item.status === 'in_progress')
      if (shouldPreserveLocalOnlyTurn) {
        mergedById.set(localTurn.id, localTurn)
      }
      continue
    }

    mergedById.set(localTurn.id, {
      ...hydratedTurn,
      status:
        hydratedTurn.status === 'running' || localTurn.status === 'running'
          ? 'running'
          : hydratedTurn.status,
      startedAt: hydratedTurn.startedAt ?? localTurn.startedAt,
      completedAt:
        hydratedTurn.status === 'running' ? null : hydratedTurn.completedAt ?? localTurn.completedAt,
    })
  }

  return sortTurnsByStartedAt([...mergedById.values()])
}

export const shouldPreserveDraftHandoffState = ({
  previousSessionId,
  nextSessionId,
  pendingSessionHandoffId,
  runtimeState,
}: {
  previousSessionId: string
  nextSessionId: string
  pendingSessionHandoffId: string | null
  runtimeState: {
    isStreaming: boolean
    turns: Array<{ sessionId: string }>
  }
}) =>
  previousSessionId === DRAFT_SESSION_ID &&
  nextSessionId !== DRAFT_SESSION_ID &&
  (pendingSessionHandoffId === nextSessionId ||
    (runtimeState.isStreaming &&
      runtimeState.turns.some((turn) => turn.sessionId === nextSessionId)))

export const toRuntimeRequestError = (
  error:
    | {
        code?: string | null
        message: string
      }
    | null
    | undefined,
) => {
  if (!error?.message) {
    return null
  }

  return new ApiError(error.message, {
    status: 502,
    code: error.code ?? null,
    category:
      Object.values(API_ERROR_CODES).find((candidate) => candidate.code === error.code)?.category ??
      null,
    retryable:
      Object.values(API_ERROR_CODES).find((candidate) => candidate.code === error.code)?.retryable ??
      false,
  })
}

export function useChatWorkspace({
  currentUser,
  onUnauthorized,
  activeSessionId,
  onOpenSession,
  onOpenNewSession,
}: UseChatWorkspaceParams) {
  const sessionId = activeSessionId ?? DRAFT_SESSION_ID
  const [runtimeV2State, setRuntimeV2State] = useState(createInitialChatRuntimeV2State)
  const [optimisticUserPreviews, setOptimisticUserPreviews] = useState<
    Record<string, OptimisticUserPreview>
  >({})
  const [input, setInput] = useState('')
  const [attachments, setAttachments] = useState<UploadedImageAttachment[]>([])
  const [requestErrorState, setRequestErrorState] = useState<ApiError | null>(null)

  const previousSessionIdRef = useRef(sessionId)
  const runtimeV2StateRef = useRef(runtimeV2State)
  const pendingSessionHandoffRef = useRef<string | null>(null)

  useEffect(() => {
    runtimeV2StateRef.current = runtimeV2State
  }, [runtimeV2State])

  useEffect(() => {
    const nextError = toRuntimeRequestError(runtimeV2State.error)
    if (!nextError) {
      return
    }

    setRequestErrorState(nextError)
  }, [runtimeV2State.error])

  useEffect(() => {
    const previousSessionId = previousSessionIdRef.current
    previousSessionIdRef.current = sessionId

    ensureSessionPreference(currentUser.id, sessionId)
    setInput('')
    setAttachments([])
    setRequestErrorState(null)

    const runtimeState = runtimeV2StateRef.current
    const shouldPreserveDraftSessionHandoff = shouldPreserveDraftHandoffState({
      previousSessionId,
      nextSessionId: sessionId,
      pendingSessionHandoffId: pendingSessionHandoffRef.current,
      runtimeState,
    })

    if (shouldPreserveDraftSessionHandoff) {
      pendingSessionHandoffRef.current = null
      return
    }

    setOptimisticUserPreviews({})
    setRuntimeV2State(createInitialChatRuntimeV2State())
  }, [currentUser.id, sessionId])

  const requestError = requestErrorState?.message ?? null
  const requestErrorCode = requestErrorState?.code ?? null

  useEffect(() => {
    if (!requestError) {
      return
    }

    const timer = window.setTimeout(() => {
      setRequestErrorState((current) => (current?.message === requestError ? null : current))
    }, 5000)

    return () => {
      window.clearTimeout(timer)
    }
  }, [requestError])

  const {
    loading: catalogLoading,
    error: catalogError,
    errorCode: catalogErrorCode,
    imageMenuItems,
    imageTools,
    selectedTextModelId,
    selectedImageToolKey,
    setSelectedTextModelId,
    setSelectedImageToolKey,
    selectedTextModel,
    selectedImageTool,
    textMenuItems,
    refreshCatalog,
  } = useModelCatalog({
    currentUserId: currentUser.id,
    sessionId,
    onUnauthorized,
  })

  const {
    sessions,
    loading: sessionsLoading,
    error: sessionsError,
    errorCode: sessionsErrorCode,
    upsertSession,
    renameSession,
    deleteSession,
  } = useSessions(currentUser.id, onUnauthorized)

  const currentSession = useMemo(
    () =>
      activeSessionId
        ? sessions.find((session) => session.id === activeSessionId) ?? null
        : null,
    [activeSessionId, sessions],
  )

  const handleHydrateTurns = useCallback((turns: ThreadTurn[]) => {
    setRuntimeV2State((current) => {
      const shouldPreserveRunningLocalState =
        current.isStreaming &&
        current.turns.some((turn) => turn.sessionId === sessionId)

      if (!shouldPreserveRunningLocalState) {
        return hydrateChatRuntimeV2State(sortTurnsByStartedAt(turns))
      }

      return hydrateChatRuntimeV2State(
        mergeHydratedTurnsWithLocal(turns, current.turns, sessionId),
      )
    })
    setOptimisticUserPreviews((current) => {
      const next = { ...current }
      for (const turn of turns) {
        const hasRealUserMessage = turn.items.some((item) => item.type === 'userMessage')
        if (hasRealUserMessage || turn.status !== 'running') {
          delete next[turn.id]
        }
      }
      return next
    })
  }, [sessionId])
  const handlePrependHydrateTurns = useCallback((turns: ThreadTurn[]) => {
    setRuntimeV2State((current) => {
      const existingById = new Map(current.turns.map((turn) => [turn.id, turn] as const))
      const merged = [...turns]
      for (const turn of current.turns) {
        if (!existingById.has(turn.id) || !merged.some((candidate) => candidate.id === turn.id)) {
          merged.push(turn)
        }
      }
      return hydrateChatRuntimeV2State(sortTurnsByStartedAt(merged))
    })
  }, [])

  const handleHydrateSession = useCallback(
    (session: (typeof sessions)[number] | null) => {
      if (!session) {
        return
      }
      upsertSession(session)
    },
    [upsertSession],
  )

  const handleStreamEvent = useCallback((event: ChatStreamEvent) => {
    if (event.type === 'session.updated') {
      upsertSession({
        id: event.session.id,
        title: event.session.title,
        status: event.session.status,
        createdAt: event.session.createdAt,
        updatedAt: event.session.updatedAt,
      })
    }
    if (event.type === 'item.started' && event.item.role === 'user') {
      setOptimisticUserPreviews((current) => {
        if (!current[event.turnId]) {
          return current
        }
        const next = { ...current }
        delete next[event.turnId]
        return next
      })
    }
    if (event.type === 'turn.completed' || event.type === 'turn.failed') {
      setOptimisticUserPreviews((current) => {
        if (!current[event.turnId]) {
          return current
        }
        const next = { ...current }
        delete next[event.turnId]
        return next
      })
    }
    setRuntimeV2State((current) => applyThreadStreamEvent(current, event))
  }, [upsertSession])

  const { historyHasMore, historyLoading, loadOlderHistory } = useChatStream({
    currentUserId: currentUser.id,
    sessionId,
    enabled: Boolean(activeSessionId && currentSession),
    onHydrateTurns: handleHydrateTurns,
    onPrependHydrateTurns: handlePrependHydrateTurns,
    onHydrateSession: handleHydrateSession,
    onEvent: handleStreamEvent,
  })

  useEffect(() => {
    if (!activeSessionId || !currentSession) {
      return
    }

    const activeTurns = runtimeV2State.turns.filter((turn) => turn.sessionId === activeSessionId)
    if (activeTurns.length === 0) {
      return
    }

    writeLocalApiCache(currentUser.id, sessionDetailCacheKey(activeSessionId), {
      session: {
        id: currentSession.id,
        title: currentSession.title,
        status: currentSession.status,
        createdAt: currentSession.createdAt,
        updatedAt: currentSession.updatedAt,
      },
      turns: activeTurns,
      historyPage: {
        hasMore: historyHasMore,
        nextBeforeTurnId: null,
      },
    })
  }, [activeSessionId, currentSession, currentUser.id, historyHasMore, runtimeV2State.turns])

  useEffect(() => {
    const storedPreferences = ensureSessionPreference(currentUser.id, sessionId)
    updateSessionPreference(currentUser.id, sessionId, {
      textModelId: selectedTextModelId ?? storedPreferences.textModelId,
      imageToolKey: selectedImageToolKey ?? storedPreferences.imageToolKey,
    })
  }, [currentUser.id, selectedImageToolKey, selectedTextModelId, sessionId])

  const pending = runtimeV2State.isStreaming
  const selectedModelSupportsImageInputs =
    selectedTextModel?.input_modalities?.some((modality) => {
      const normalized = modality.toLowerCase()
      return normalized === 'image' || normalized === 'vision'
    }) ?? false

  const createAndSelectSession = useCallback(() => {
    onOpenNewSession()
  }, [onOpenNewSession])

  const runChatTurn = useCallback(
    async (prompt: string) => {
      if (!selectedTextModel) {
        throw new ApiError('Select a text model before starting a conversation', {
          status: 400,
          code: CLIENT_ERROR_CODES.modelSelectionRequired.code,
          category: CLIENT_ERROR_CODES.modelSelectionRequired.category,
          retryable: CLIENT_ERROR_CODES.modelSelectionRequired.retryable,
        })
      }
      if (selectedTextModel.available === false) {
        throw new ApiError(
          selectedTextModel.unavailable_reason ??
            '请先在右侧参数中配置当前模型的 API Key，然后再发送消息',
          {
            status: 400,
            code: API_ERROR_CODES.providerApiKeyRequired.code,
            category: API_ERROR_CODES.providerApiKeyRequired.category,
            retryable: API_ERROR_CODES.providerApiKeyRequired.retryable,
          },
        )
      }
      if (selectedImageTool && !selectedImageTool.available) {
        throw new ApiError(
          selectedImageTool.unavailable_reason ??
            'The selected image tool is not available for this account',
          {
            status: 400,
            code: API_ERROR_CODES.toolUnavailable.code,
            category: API_ERROR_CODES.toolUnavailable.category,
            retryable: API_ERROR_CODES.toolUnavailable.retryable,
          },
        )
      }
      setRequestErrorState(null)
      const targetSessionId = activeSessionId ?? createSessionId()
      ensureSessionPreference(currentUser.id, targetSessionId)

      const response = await ensureOk(await authenticatedFetch('/api/chat', {
        method: 'POST',
        body: JSON.stringify(
          buildChatRequest(
            targetSessionId,
            prompt,
            selectedTextModel,
            selectedImageTool,
            attachments,
          ),
        ),
      }), 'Failed to start OpenChat chat turn')

      const payload = (await response.json().catch(() => ({}))) as Record<string, unknown>
      if (payload.status !== 'done') {
        throw new ApiError('OpenChat server did not accept the chat request', {
          status: 502,
          code: CLIENT_ERROR_CODES.invalidChatAcceptance.code,
          category: CLIENT_ERROR_CODES.invalidChatAcceptance.category,
          retryable: CLIENT_ERROR_CODES.invalidChatAcceptance.retryable,
        })
      }

      const acceptedSessionId =
        typeof payload.session_id === 'string' && payload.session_id.trim()
          ? payload.session_id
          : targetSessionId
      const acceptedTurnId =
        typeof payload.turn_id === 'string' && payload.turn_id.trim()
          ? payload.turn_id
          : null

      const acceptedAt = createShanghaiTimestamp()
      upsertSession({
        id: acceptedSessionId,
        title: null,
        status: 'running',
        createdAt: acceptedAt,
        updatedAt: acceptedAt,
      })
      if (acceptedTurnId) {
        setRuntimeV2State((current) => {
          const next = appendOptimisticTurn(current, {
            id: acceptedTurnId,
            sessionId: acceptedSessionId,
            startedAt: acceptedAt,
          })
          // Keep ref in sync immediately so session handoff effect does not
          // observe stale state and reset the just-created optimistic turn.
          runtimeV2StateRef.current = next
          return next
        })
        setOptimisticUserPreviews((current) => ({
          ...current,
          [acceptedTurnId]: {
            sessionId: acceptedSessionId,
            text: prompt,
            createdAt: acceptedAt,
          },
        }))
      }
      if (!activeSessionId) {
        pendingSessionHandoffRef.current = acceptedSessionId
        onOpenSession(acceptedSessionId)
      }
    },
    [
      activeSessionId,
      attachments,
      currentUser.id,
      onOpenSession,
      selectedImageTool,
      selectedTextModel,
      upsertSession,
    ],
  )

  const handleSubmit = useCallback(async () => {
    const next = input.trim()
    if ((!next && attachments.length === 0) || pending || !selectedTextModel) {
      return
    }

    setInput('')
    setAttachments([])
    setRequestErrorState(null)

    try {
      await runChatTurn(next)
    } catch (error) {
      setInput((current) => (current ? current : next))
      setAttachments((current) => (current.length === 0 ? attachments : current))
      setRequestErrorState(toApiError(error, 'Unknown OpenChat request failure'))
    }
  }, [attachments, input, pending, runChatTurn, selectedTextModel])

  const handleRemoveAttachment = useCallback((attachmentId: string) => {
    setAttachments((current) => current.filter((attachment) => attachment.id !== attachmentId))
  }, [])

  const handleUploadImages = useCallback(
    async (files: File[]) => {
      try {
        const supportedFiles = filterSupportedAttachmentFiles(files)
        if (supportedFiles.length === 0) {
          if (files.length > 0) {
            throw new ApiError(getUnsupportedImageMessage(), {
              status: 400,
              code: API_ERROR_CODES.unsupportedUploadType.code,
              category: API_ERROR_CODES.unsupportedUploadType.category,
              retryable: API_ERROR_CODES.unsupportedUploadType.retryable,
            })
          }
          return
        }

        const imageFiles = filterSupportedImageFiles(files)
        if (imageFiles.length > 0 && !selectedModelSupportsImageInputs) {
          throw new ApiError('当前模型不支持图像输入，请切换到多模态模型后再上传图片', {
            status: 400,
            code: CLIENT_ERROR_CODES.imageInputNotSupported.code,
            category: CLIENT_ERROR_CODES.imageInputNotSupported.category,
            retryable: CLIENT_ERROR_CODES.imageInputNotSupported.retryable,
          })
        }

        setRequestErrorState(null)
        const preparedFiles = await compressImageFiles(imageFiles)
        const imageFileSet = new Set<File>(imageFiles)
        const documentFiles = supportedFiles.filter((file) => !imageFileSet.has(file))

        const formData = new FormData()
        for (const file of preparedFiles) {
          formData.append('files', file, file.name)
        }
        for (const file of documentFiles) {
          formData.append('files', file, file.name)
        }

        const response = await ensureOk(await authenticatedFetch('/api/uploads/files', {
          method: 'POST',
          body: formData,
        }), '文件上传失败')

        const uploaded = (await response.json()) as UploadedImageAttachment[]
        setAttachments((current) => {
          const seen = new Set(current.map((attachment) => attachment.id))
          return [
            ...current,
            ...uploaded.filter((attachment) => !seen.has(attachment.id)),
          ]
        })
      } catch (error) {
        setRequestErrorState(toApiError(error, '文件上传失败'))
      }
    },
    [selectedModelSupportsImageInputs],
  )

  const handleSelectSession = useCallback((nextSessionId: string) => {
    if (!nextSessionId || nextSessionId === activeSessionId) {
      return
    }
    ensureSessionPreference(currentUser.id, nextSessionId)
    onOpenSession(nextSessionId)
  }, [activeSessionId, currentUser.id, onOpenSession])

  const handleDeleteSession = useCallback(
    async (targetSessionId: string) => {
      try {
        setRequestErrorState(null)
        await deleteSession(targetSessionId)

        if (targetSessionId === activeSessionId) {
          createAndSelectSession()
        }
      } catch (error) {
        setRequestErrorState(toApiError(error, 'Failed to delete session'))
      }
    },
    [activeSessionId, createAndSelectSession, deleteSession],
  )

  const handleRenameSession = useCallback(
    async (targetSessionId: string, title: string) => {
      const normalizedTitle = title.trim()
      if (!normalizedTitle) {
        throw new ApiError('会话标题不能为空', {
          status: 400,
          code: API_ERROR_CODES.validationError.code,
          category: API_ERROR_CODES.validationError.category,
          retryable: API_ERROR_CODES.validationError.retryable,
        })
      }

      try {
        setRequestErrorState(null)
        return await renameSession(targetSessionId, normalizedTitle)
      } catch (error) {
        setRequestErrorState(toApiError(error, 'Failed to rename session'))
        throw error
      }
    },
    [renameSession],
  )

  const handleInterruptTurn = useCallback(async () => {
    const activeTurnId = runtimeV2State.activeTurnId
    const isStreaming = runtimeV2State.isStreaming
    if (!activeSessionId || !isStreaming || !activeTurnId) {
      return
    }

    try {
      setRequestErrorState(null)
      await ensureOk(await authenticatedFetch(
        `/api/sessions/${activeSessionId}/turns/${activeTurnId}/interrupt`,
        {
          method: 'POST',
        },
      ), 'Failed to stop generation')
    } catch (error) {
      setRequestErrorState(toApiError(error, 'Failed to stop generation'))
    }
  }, [
    activeSessionId,
    runtimeV2State.activeTurnId,
    runtimeV2State.isStreaming,
  ])

  return {
    catalogError,
    catalogErrorCode,
    catalogLoading,
    currentSession,
    handleDeleteSession,
    handleRenameSession,
    handleSelectSession,
    handleSubmit,
    handleInterruptTurn,
    imageMenuItems,
    imageTools,
    input,
    pending,
    requestError,
    requestErrorCode,
    runtimeV2State,
    optimisticUserPreviews,
    historyHasMore,
    historyLoading,
    loadOlderHistory,
    attachments,
    selectedImageTool,
    selectedImageToolKey,
    selectedTextModel,
    selectedTextModelId,
    sessions,
    sessionsError,
    sessionsErrorCode,
    sessionsLoading,
    setInput,
    setSelectedImageToolKey,
    setSelectedTextModelId,
    uploadImages: handleUploadImages,
    removeAttachment: handleRemoveAttachment,
    startNewSession: createAndSelectSession,
    textMenuItems,
    selectedModelSupportsImageInputs,
    refreshCatalog,
  }
}
