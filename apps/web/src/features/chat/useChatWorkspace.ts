import { useCallback, useEffect, useMemo, useReducer, useState } from 'react'
import {
  applyStreamEvent,
  createInitialChatRuntimeState,
  hydrateChatRuntimeState,
} from '@openchat/chat-core'
import { type ChatMessage, type ChatStreamEvent } from '@openchat/protocol'
import type { AuthUser } from '../../lib/auth'
import { authenticatedFetch } from '../../lib/auth'
import { ApiError, API_ERROR_CODES, CLIENT_ERROR_CODES, ensureOk, toApiError } from '../../lib/apiError'
import { buildChatRequest } from './request'
import {
  createSessionId,
  ensureSessionPreference,
  updateSessionPreference,
} from './sessionPreferences'
import { useChatStream } from './useChatStream'
import {
  filterSupportedImageFiles,
  getUnsupportedImageMessage,
} from './imageUpload'
import { compressImageFiles } from './imageCompression'
import { useModelCatalog } from './useModelCatalog'
import { useSessions } from './useSessions'
import { createShanghaiTimestamp } from './timestamps'
import type { UploadedImageAttachment } from './types'

type RuntimeAction =
  | { type: 'hydrate'; messages: ChatMessage[] }
  | { type: 'prepend'; messages: ChatMessage[] }
  | { type: 'stream'; event: ChatStreamEvent }
  | { type: 'reset' }

const runtimeReducer = (
  state: ReturnType<typeof createInitialChatRuntimeState>,
  action: RuntimeAction,
) => {
  switch (action.type) {
    case 'hydrate':
      return hydrateChatRuntimeState(action.messages)
    case 'prepend': {
      const existingIds = new Set(state.messages.map((message) => message.id))
      const nextMessages = [
        ...action.messages.filter((message) => !existingIds.has(message.id)),
        ...state.messages,
      ]
      return hydrateChatRuntimeState(nextMessages)
    }
    case 'stream':
      return applyStreamEvent(state, action.event)
    case 'reset':
      return createInitialChatRuntimeState()
    default:
      return state
  }
}

interface UseChatWorkspaceParams {
  currentUser: AuthUser
  onUnauthorized: () => void
  activeSessionId: string | null
  onOpenSession: (sessionId: string) => void
  onOpenNewSession: () => void
}

const DRAFT_SESSION_ID = '__draft__'

export function useChatWorkspace({
  currentUser,
  onUnauthorized,
  activeSessionId,
  onOpenSession,
  onOpenNewSession,
}: UseChatWorkspaceParams) {
  const sessionId = activeSessionId ?? DRAFT_SESSION_ID
  const [runtimeState, dispatch] = useReducer(
    runtimeReducer,
    undefined,
    createInitialChatRuntimeState,
  )
  const [input, setInput] = useState('')
  const [attachments, setAttachments] = useState<UploadedImageAttachment[]>([])
  const [requestErrorState, setRequestErrorState] = useState<ApiError | null>(null)
  const [requestPending, setRequestPending] = useState(false)

  useEffect(() => {
    ensureSessionPreference(currentUser.id, sessionId)
    dispatch({ type: 'reset' })
    setInput('')
    setAttachments([])
    setRequestErrorState(null)
    setRequestPending(false)
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

  const handleHydrate = useCallback((messages: ChatMessage[]) => {
    dispatch({ type: 'hydrate', messages })
  }, [])
  const handlePrependHydrate = useCallback((messages: ChatMessage[]) => {
    dispatch({ type: 'prepend', messages })
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
    dispatch({ type: 'stream', event })
  }, [upsertSession])

  const { historyHasMore, historyLoading, loadOlderHistory } = useChatStream({
    sessionId,
    enabled: Boolean(activeSessionId && currentSession),
    onHydrate: handleHydrate,
    onPrependHydrate: handlePrependHydrate,
    onHydrateSession: handleHydrateSession,
    onEvent: handleStreamEvent,
  })

  useEffect(() => {
    const storedPreferences = ensureSessionPreference(currentUser.id, sessionId)
    updateSessionPreference(currentUser.id, sessionId, {
      textModelId: selectedTextModelId ?? storedPreferences.textModelId,
      imageToolKey: selectedImageToolKey ?? storedPreferences.imageToolKey,
    })
  }, [currentUser.id, selectedImageToolKey, selectedTextModelId, sessionId])

  const pending = runtimeState.isStreaming
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
      setRequestPending(true)
      const targetSessionId = activeSessionId ?? createSessionId()
      ensureSessionPreference(currentUser.id, targetSessionId)

      try {
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

        const acceptedAt = createShanghaiTimestamp()
        upsertSession({
          id: acceptedSessionId,
          title: null,
          status: 'running',
          createdAt: acceptedAt,
          updatedAt: acceptedAt,
        })

        if (!activeSessionId) {
          onOpenSession(acceptedSessionId)
        }
      } finally {
        setRequestPending(false)
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
        const imageFiles = filterSupportedImageFiles(files)
        if (imageFiles.length === 0) {
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

        if (!selectedModelSupportsImageInputs) {
          throw new ApiError('当前模型不支持图像输入，请切换到多模态模型后再上传图片', {
            status: 400,
            code: CLIENT_ERROR_CODES.imageInputNotSupported.code,
            category: CLIENT_ERROR_CODES.imageInputNotSupported.category,
            retryable: CLIENT_ERROR_CODES.imageInputNotSupported.retryable,
          })
        }

        setRequestErrorState(null)
        const preparedFiles = await compressImageFiles(imageFiles)

        const formData = new FormData()
        for (const file of preparedFiles) {
          formData.append('files', file, file.name)
        }

        const response = await ensureOk(await authenticatedFetch('/api/uploads/images', {
          method: 'POST',
          body: formData,
        }), '图片上传失败')

        const uploaded = (await response.json()) as UploadedImageAttachment[]
        setAttachments((current) => {
          const seen = new Set(current.map((attachment) => attachment.id))
          return [
            ...current,
            ...uploaded.filter((attachment) => !seen.has(attachment.id)),
          ]
        })
      } catch (error) {
        setRequestErrorState(toApiError(error, '图片上传失败'))
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
    if (!activeSessionId || !runtimeState.isStreaming || !runtimeState.activeTurnId) {
      return
    }

    try {
      setRequestErrorState(null)
      await ensureOk(await authenticatedFetch(
        `/api/sessions/${activeSessionId}/turns/${runtimeState.activeTurnId}/interrupt`,
        {
          method: 'POST',
        },
      ), 'Failed to stop generation')
    } catch (error) {
      setRequestErrorState(toApiError(error, 'Failed to stop generation'))
    }
  }, [activeSessionId, runtimeState.activeTurnId, runtimeState.isStreaming])

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
    requestPending,
    requestError,
    requestErrorCode,
    runtimeState,
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
