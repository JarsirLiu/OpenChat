import { useCallback, useEffect, useMemo, useReducer, useState } from 'react'
import {
  applyStreamEvent,
  createInitialChatRuntimeState,
  hydrateChatRuntimeState,
} from '@openchat/chat-core'
import { type ChatMessage, type ChatStreamEvent } from '@openchat/protocol'
import type { AuthUser } from '../../lib/auth'
import { authenticatedFetch } from '../../lib/auth'
import { buildChatRequest } from './request'
import {
  createSessionId,
  ensureSessionPreference,
  readStoredSessionId,
  updateSessionPreference,
  writeStoredSessionId,
} from './sessionPreferences'
import { useChatStream } from './useChatStream'
import {
  filterSupportedImageFiles,
  getUnsupportedImageMessage,
} from './imageUpload'
import { compressImageFiles } from './imageCompression'
import { useModelCatalog } from './useModelCatalog'
import { useSessions } from './useSessions'
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
}

export function useChatWorkspace({ currentUser, onUnauthorized }: UseChatWorkspaceParams) {
  const [sessionId, setSessionId] = useState(
    () => readStoredSessionId(currentUser.id) ?? createSessionId(),
  )
  const [runtimeState, dispatch] = useReducer(
    runtimeReducer,
    undefined,
    createInitialChatRuntimeState,
  )
  const [input, setInput] = useState('')
  const [attachments, setAttachments] = useState<UploadedImageAttachment[]>([])
  const [requestError, setRequestError] = useState<string | null>(null)
  const [requestPending, setRequestPending] = useState(false)

  useEffect(() => {
    ensureSessionPreference(currentUser.id, sessionId)
    writeStoredSessionId(currentUser.id, sessionId)
    dispatch({ type: 'reset' })
    setInput('')
    setAttachments([])
    setRequestError(null)
    setRequestPending(false)
  }, [currentUser.id, sessionId])

  useEffect(() => {
    setSessionId(readStoredSessionId(currentUser.id) ?? createSessionId())
  }, [currentUser.id])

  useEffect(() => {
    if (!requestError) {
      return
    }

    const timer = window.setTimeout(() => {
      setRequestError((current) => (current === requestError ? null : current))
    }, 5000)

    return () => {
      window.clearTimeout(timer)
    }
  }, [requestError])

  const {
    loading: catalogLoading,
    error: catalogError,
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
    upsertSession,
    renameSession,
    deleteSession,
  } = useSessions(currentUser.id, onUnauthorized)

  const currentSession = useMemo(
    () => sessions.find((session) => session.id === sessionId) ?? null,
    [sessionId, sessions],
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
    enabled: Boolean(currentSession),
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
    const nextSessionId = createSessionId()
    ensureSessionPreference(currentUser.id, nextSessionId)
    setSessionId(nextSessionId)
  }, [currentUser.id])

  const runChatTurn = useCallback(
    async (prompt: string) => {
      if (!selectedTextModel) {
        throw new Error('Select a text model before starting a conversation')
      }
      if (selectedTextModel.available === false) {
        throw new Error(
          selectedTextModel.unavailable_reason ??
            '请先在右侧参数中配置当前模型的 API Key，然后再发送消息',
        )
      }
      if (selectedImageTool && !selectedImageTool.available) {
        throw new Error(
          selectedImageTool.unavailable_reason ??
            'The selected image tool is not available for this account',
        )
      }
      setRequestError(null)
      setRequestPending(true)

      try {
        const response = await authenticatedFetch('/api/chat', {
          method: 'POST',
          body: JSON.stringify(
            buildChatRequest(
              sessionId,
              prompt,
              selectedTextModel,
              selectedImageTool,
              attachments,
            ),
          ),
        })

        if (!response.ok) {
          const payload = (await response.json().catch(() => null)) as { message?: string } | null
          throw new Error(payload?.message ?? 'Failed to start OpenChat chat turn')
        }

        const payload = (await response.json().catch(() => ({}))) as Record<string, unknown>
        if (payload.status !== 'done') {
          throw new Error('OpenChat server did not accept the chat request')
        }

        upsertSession({
          id:
            typeof payload.session_id === 'string' && payload.session_id.trim()
              ? payload.session_id
              : sessionId,
          title: null,
          status: 'running',
          createdAt: Date.now().toString(),
          updatedAt: Date.now().toString(),
        })
      } finally {
        setRequestPending(false)
      }
    },
    [attachments, selectedImageTool, selectedTextModel, sessionId, upsertSession],
  )

  const handleSubmit = useCallback(async () => {
    const next = input.trim()
    if ((!next && attachments.length === 0) || pending || !selectedTextModel) {
      return
    }

    setInput('')
    setAttachments([])
    setRequestError(null)

    try {
      await runChatTurn(next)
    } catch (error) {
      setInput((current) => (current ? current : next))
      setAttachments((current) => (current.length === 0 ? attachments : current))
      setRequestError(error instanceof Error ? error.message : 'Unknown OpenChat request failure')
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
            throw new Error(getUnsupportedImageMessage())
          }
          return
        }

        if (!selectedModelSupportsImageInputs) {
          throw new Error('当前模型不支持图像输入，请切换到多模态模型后再上传图片')
        }

        setRequestError(null)
        const preparedFiles = await compressImageFiles(imageFiles)

        const formData = new FormData()
        for (const file of preparedFiles) {
          formData.append('files', file, file.name)
        }

        const response = await authenticatedFetch('/api/uploads/images', {
          method: 'POST',
          body: formData,
        })

        if (!response.ok) {
          const payload = (await response.json().catch(() => null)) as { message?: string } | null
          throw new Error(payload?.message ?? '图片上传失败')
        }

        const uploaded = (await response.json()) as UploadedImageAttachment[]
        setAttachments((current) => {
          const seen = new Set(current.map((attachment) => attachment.id))
          return [
            ...current,
            ...uploaded.filter((attachment) => !seen.has(attachment.id)),
          ]
        })
      } catch (error) {
        setRequestError(error instanceof Error ? error.message : '图片上传失败')
      }
    },
    [selectedModelSupportsImageInputs],
  )

  const handleSelectSession = useCallback((nextSessionId: string) => {
    if (!nextSessionId || nextSessionId === sessionId) {
      return
    }
    ensureSessionPreference(currentUser.id, nextSessionId)
    setSessionId(nextSessionId)
  }, [currentUser.id, sessionId])

  const handleDeleteSession = useCallback(
    async (targetSessionId: string) => {
      try {
        setRequestError(null)
        await deleteSession(targetSessionId)

        if (targetSessionId === sessionId) {
          createAndSelectSession()
        }
      } catch (error) {
        setRequestError(error instanceof Error ? error.message : 'Failed to delete session')
      }
    },
    [createAndSelectSession, deleteSession, sessionId],
  )

  const handleRenameSession = useCallback(
    async (targetSessionId: string, title: string) => {
      const normalizedTitle = title.trim()
      if (!normalizedTitle) {
        throw new Error('会话标题不能为空')
      }

      try {
        setRequestError(null)
        return await renameSession(targetSessionId, normalizedTitle)
      } catch (error) {
        setRequestError(error instanceof Error ? error.message : 'Failed to rename session')
        throw error
      }
    },
    [renameSession],
  )

  const handleInterruptTurn = useCallback(async () => {
    if (!runtimeState.isStreaming || !runtimeState.activeTurnId) {
      return
    }

    try {
      setRequestError(null)
      const response = await authenticatedFetch(
        `/api/sessions/${sessionId}/turns/${runtimeState.activeTurnId}/interrupt`,
        {
          method: 'POST',
        },
      )

      if (!response.ok) {
        const payload = (await response.json().catch(() => null)) as { message?: string } | null
        throw new Error(payload?.message ?? 'Failed to stop generation')
      }
    } catch (error) {
      setRequestError(error instanceof Error ? error.message : 'Failed to stop generation')
    }
  }, [runtimeState.activeTurnId, runtimeState.isStreaming, sessionId])

  return {
    catalogError,
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
