import { useEffect, useEffectEvent, useState } from 'react'
import { fetchEventSource } from '@microsoft/fetch-event-source'
import type { ChatMessage, ChatStreamEvent, ItemStatus, MessageContentPart } from '@openchat/protocol'
import { normalizeStreamEvent } from '@openchat/protocol'
import { authenticatedFetch } from '../../lib/auth'
import type { SessionListItem } from './useSessions'

interface UseChatStreamParams {
  sessionId: string
  enabled: boolean
  onHydrate: (messages: ChatMessage[]) => void
  onHydrateSession: (session: SessionListItem | null) => void
  onEvent: (event: ChatStreamEvent) => void
}

interface SessionDetailResponse {
  session?: {
    id?: string
    title?: string | null
    status?: string
    createdAt?: string
    updatedAt?: string
  }
  messages?: Array<{
    id?: string
    role?: 'user' | 'assistant' | 'reasoning'
    turnId?: string
    status?: ItemStatus
    createdAt?: string
    updatedAt?: string
    content?: unknown
    toolCalls?: unknown
  }>
}

const normalizeSessionMessages = (value: SessionDetailResponse['messages']): ChatMessage[] =>
  Array.isArray(value)
    ? (() => {
        const messages: ChatMessage[] = []

        const appendMessage = (nextMessage: ChatMessage) => {
          if (nextMessage.role !== 'reasoning') {
            messages.push(nextMessage)
            return
          }

          const assistantIndex = messages.findIndex(
            (message) => message.role === 'assistant' && message.turnId === nextMessage.turnId,
          )

          if (assistantIndex >= 0) {
            messages.splice(assistantIndex, 0, nextMessage)
            return
          }

          messages.push(nextMessage)
        }

        for (const message of value) {
          const { id, role, turnId, status, createdAt, updatedAt } = message

          if (!id || !turnId || !status) {
            continue
          }

          if (role !== 'user' && role !== 'assistant' && role !== 'reasoning') {
            continue
          }

          const content = Array.isArray(message.content) ? message.content : []

          const normalizedContent: MessageContentPart[] = content.flatMap((part): MessageContentPart[] => {
            if (!part || typeof part !== 'object') {
              return []
            }

            const item = part as Record<string, unknown>
            if (item.type === 'text' && typeof item.text === 'string') {
              return [{ type: 'text', text: item.text }]
            }

            if (item.type === 'image' && typeof item.url === 'string') {
              return [{ type: 'image', url: item.url, alt: typeof item.alt === 'string' ? item.alt : 'Image' }]
            }

            return []
          })

          const normalizedToolCalls = Array.isArray(message.toolCalls)
            ? message.toolCalls.flatMap((toolCall) => {
                if (!toolCall || typeof toolCall !== 'object') {
                  return []
                }
                const item = toolCall as Record<string, unknown>
                const toolCallId = typeof item.id === 'string' ? item.id : null
                const name = typeof item.name === 'string' ? item.name : null
                if (!toolCallId || !name) {
                  return []
                }
                return [
                  {
                    id: toolCallId,
                    name,
                    displayName: typeof item.displayName === 'string' ? item.displayName : undefined,
                    parentItemId:
                      typeof item.parentItemId === 'string' ? item.parentItemId : undefined,
                    argumentsText:
                      typeof item.argumentsText === 'string' ? item.argumentsText : undefined,
                    result:
                      item.result && typeof item.result === 'object'
                        ? (item.result as Record<string, unknown>)
                        : typeof item.result === 'string'
                          ? item.result
                          : undefined,
                    status:
                      item.status === 'in_progress' ||
                      item.status === 'interrupted' ||
                      item.status === 'completed' ||
                      item.status === 'failed'
                        ? (item.status as ItemStatus)
                        : undefined,
                    media: Array.isArray(item.media)
                      ? item.media
                          .filter((entry): entry is Record<string, unknown> => Boolean(entry && typeof entry === 'object'))
                          .flatMap((entry) =>
                            typeof entry.kind === 'string' &&
                            typeof entry.url === 'string' &&
                            typeof entry.mimeType === 'string' &&
                            typeof entry.sizeBytes === 'number'
                              ? [
                                  {
                                    kind: entry.kind,
                                    url: entry.url,
                                    mimeType: entry.mimeType,
                                    sizeBytes: entry.sizeBytes,
                                  },
                                ]
                              : [],
                          )
                      : undefined,
                  },
                ]
              })
            : undefined

          const nextMessage: ChatMessage = {
            id,
            role,
            turnId,
            status,
            createdAt,
            updatedAt,
            content: normalizedContent,
            toolCalls: normalizedToolCalls,
          }

          appendMessage(nextMessage)
        }

        return messages
      })()
    : []

const normalizeSession = (value: SessionDetailResponse['session']): SessionListItem | null => {
  if (!value || typeof value !== 'object') {
    return null
  }

  const { id, title, status, createdAt, updatedAt } = value
  if (
    typeof id !== 'string' ||
    typeof status !== 'string' ||
    typeof createdAt !== 'string' ||
    typeof updatedAt !== 'string'
  ) {
    return null
  }

  return {
    id,
    title: typeof title === 'string' ? title : null,
    status,
    createdAt,
    updatedAt,
  }
}

export function useChatStream({
  sessionId,
  enabled,
  onHydrate,
  onHydrateSession,
  onEvent,
}: UseChatStreamParams) {
  const [streamState, setStreamState] = useState<'connecting' | 'connected' | 'disconnected'>(
    'connecting',
  )
  const handleEvent = useEffectEvent((event: ChatStreamEvent) => {
    onEvent(event)
  })
  const handleHydrateMessages = useEffectEvent((messages: ChatMessage[]) => {
    onHydrate(messages)
  })
  const handleHydrateSession = useEffectEvent((session: SessionListItem | null) => {
    onHydrateSession(session)
  })

  useEffect(() => {
    if (!enabled) {
      setStreamState('disconnected')
      return
    }

    setStreamState('connecting')
    let active = true
    let bootstrapping = true
    const queuedEvents: ChatStreamEvent[] = []
    const abortController = new AbortController()

    void (async () => {
      try {
        const response = await authenticatedFetch(`/api/sessions/${sessionId}`)
        if (!response.ok) {
          return
        }

        const payload = (await response.json()) as SessionDetailResponse
        if (!active) {
          return
        }

        handleHydrateSession(normalizeSession(payload.session))
        handleHydrateMessages(normalizeSessionMessages(payload.messages))
      } catch {
        // Ignore hydration failures and fall back to live stream events.
      } finally {
        if (!active) {
          return
        }

        bootstrapping = false
        queuedEvents.forEach((event) => {
          handleEvent(event)
        })
      }
    })()

    void fetchEventSource(`/api/stream/${sessionId}`, {
      signal: abortController.signal,
      openWhenHidden: true,
      fetch: (input, init) =>
        authenticatedFetch(
          input instanceof Request ? input.url : typeof input === 'string' ? input : input.toString(),
          init,
        ),
      async onopen(response) {
        if (!response.ok) {
          throw new Error(`Unexpected stream response: ${response.status}`)
        }

        setStreamState('connected')
      },
      onmessage(message) {
        if (message.event !== 'stream_event') {
          return
        }

        const payload = JSON.parse(message.data) as unknown
        const event = normalizeStreamEvent(payload)
        if (!event || event.sessionId !== sessionId) {
          return
        }

        if (bootstrapping) {
          queuedEvents.push(event)
          return
        }

        handleEvent(event)
      },
      onclose() {
        setStreamState('disconnected')
      },
      onerror(error) {
        if (abortController.signal.aborted) {
          throw error
        }

        setStreamState('disconnected')
        return 1000
      },
    }).catch(() => {
      if (!abortController.signal.aborted) {
        setStreamState('disconnected')
      }
    })

    return () => {
      active = false
      bootstrapping = false
      abortController.abort()
    }
  }, [enabled, sessionId])

  return {
    streamState,
  }
}
