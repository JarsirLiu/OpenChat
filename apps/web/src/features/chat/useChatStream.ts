import { useEffect, useEffectEvent, useRef, useState } from 'react'
import { fetchEventSource } from '@microsoft/fetch-event-source'
import type { ChatStreamEvent, ThreadTurn } from '@openchat/protocol'
import { normalizeStreamEvent } from '@openchat/protocol'
import { authenticatedFetch } from '../../lib/auth'
import { readLocalApiCache, writeLocalApiCache } from '../../lib/localApiCache'
import type { SessionListItem } from './useSessions'
import { normalizeSessionTurns } from './threadItems'

interface UseChatStreamParams {
  currentUserId: string
  sessionId: string
  enabled: boolean
  onHydrateTurns?: (turns: ThreadTurn[]) => void
  onPrependHydrateTurns?: (turns: ThreadTurn[]) => void
  onHydrateSession: (session: SessionListItem | null) => void
  onEvent: (event: ChatStreamEvent) => void
}

const SESSION_DETAIL_CACHE_TTL_MS = 10_000
export const sessionDetailCacheKey = (sessionId: string) => `chat:session:${sessionId}`

interface SessionDetailResponse {
  session?: {
    id?: string
    title?: string | null
    status?: string
    createdAt?: string
    updatedAt?: string
  }
  turns?: unknown[]
  historyPage?: {
    hasMore?: boolean
    nextBeforeTurnId?: string | null
  }
}

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
  currentUserId,
  sessionId,
  enabled,
  onHydrateTurns,
  onPrependHydrateTurns,
  onHydrateSession,
  onEvent,
}: UseChatStreamParams) {
  const [streamState, setStreamState] = useState<'connecting' | 'connected' | 'disconnected'>(
    'connecting',
  )
  const [historyHasMore, setHistoryHasMore] = useState(false)
  const [historyLoading, setHistoryLoading] = useState(false)
  const nextBeforeTurnIdRef = useRef<string | null>(null)
  const handleEvent = useEffectEvent((event: ChatStreamEvent) => {
    onEvent(event)
  })
  const handleHydrateTurns = useEffectEvent((turns: ThreadTurn[]) => {
    onHydrateTurns?.(turns)
  })
  const handlePrependHydrateTurns = useEffectEvent((turns: ThreadTurn[]) => {
    onPrependHydrateTurns?.(turns)
  })
  const handleHydrateSession = useEffectEvent((session: SessionListItem | null) => {
    onHydrateSession(session)
  })

  const loadOlderHistory = async () => {
    if (!enabled || historyLoading || !historyHasMore || !nextBeforeTurnIdRef.current) {
      return false
    }

    setHistoryLoading(true)

    try {
      const response = await authenticatedFetch(
        `/api/sessions/${sessionId}?before_turn_id=${encodeURIComponent(nextBeforeTurnIdRef.current)}`,
      )
      if (!response.ok) {
        return false
      }

      const payload = (await response.json()) as SessionDetailResponse
      nextBeforeTurnIdRef.current =
        typeof payload.historyPage?.nextBeforeTurnId === 'string'
          ? payload.historyPage.nextBeforeTurnId
          : null
      setHistoryHasMore(Boolean(payload.historyPage?.hasMore))
      handlePrependHydrateTurns(normalizeSessionTurns(payload.turns as never))
      return true
    } catch {
      return false
    } finally {
      setHistoryLoading(false)
    }
  }

  useEffect(() => {
    if (!enabled) {
      setStreamState('disconnected')
      setHistoryHasMore(false)
      setHistoryLoading(false)
      nextBeforeTurnIdRef.current = null
      return
    }

    setStreamState('connecting')
    let active = true
    let bootstrapping = true
    const queuedEvents: ChatStreamEvent[] = []
    const abortController = new AbortController()

    void (async () => {
      try {
        const cached = readLocalApiCache<SessionDetailResponse>(
          currentUserId,
          sessionDetailCacheKey(sessionId),
          SESSION_DETAIL_CACHE_TTL_MS,
        )
        if (cached && active) {
          handleHydrateSession(normalizeSession(cached.data.session))
          handleHydrateTurns(normalizeSessionTurns(cached.data.turns as never))
          nextBeforeTurnIdRef.current =
            typeof cached.data.historyPage?.nextBeforeTurnId === 'string'
              ? cached.data.historyPage.nextBeforeTurnId
              : null
          setHistoryHasMore(Boolean(cached.data.historyPage?.hasMore))
        }

        const response = await authenticatedFetch(`/api/sessions/${sessionId}`)
        if (!response.ok) {
          return
        }

        const payload = (await response.json()) as SessionDetailResponse
        if (!active) {
          return
        }

        handleHydrateSession(normalizeSession(payload.session))
        handleHydrateTurns(normalizeSessionTurns(payload.turns as never))
        writeLocalApiCache(currentUserId, sessionDetailCacheKey(sessionId), payload)
        nextBeforeTurnIdRef.current =
          typeof payload.historyPage?.nextBeforeTurnId === 'string'
            ? payload.historyPage.nextBeforeTurnId
            : null
        setHistoryHasMore(Boolean(payload.historyPage?.hasMore))
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
  }, [currentUserId, enabled, sessionId])

  return {
    historyHasMore,
    historyLoading,
    loadOlderHistory,
    streamState,
  }
}
