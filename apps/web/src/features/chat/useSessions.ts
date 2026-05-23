import { useCallback, useEffect, useState } from 'react'
import { authenticatedFetch, AuthError } from '../../lib/auth'
import { ApiError, ensureOk, toApiError } from '../../lib/apiError'
import { readLocalApiCache, removeLocalApiCache, writeLocalApiCache } from '../../lib/localApiCache'

export interface SessionListItem {
  id: string
  title: string | null
  status: string
  createdAt: string
  updatedAt: string
}

const SESSIONS_CACHE_KEY = 'chat:sessions'
const SESSIONS_CACHE_TTL_MS = 15_000

export function useSessions(currentUserId: string | null, onUnauthorized: () => void) {
  const [sessions, setSessions] = useState<SessionListItem[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<ApiError | null>(null)

  useEffect(() => {
    let active = true

    const load = async () => {
      if (!currentUserId) {
        setSessions([])
        setLoading(false)
        setError(null)
        return
      }

      const cached = readLocalApiCache<SessionListItem[]>(
        currentUserId,
        SESSIONS_CACHE_KEY,
        SESSIONS_CACHE_TTL_MS,
      )
      if (cached) {
        setSessions(cached.data)
        if (cached.fresh) {
          setLoading(false)
          return
        }
      }

      setLoading(true)
      setError(null)
      try {
        const response = await ensureOk(
          await authenticatedFetch('/api/sessions'),
          'Failed to load sessions',
        )
        const payload = (await response.json()) as SessionListItem[]
        if (!active) {
          return
        }
        setSessions(payload)
        writeLocalApiCache(currentUserId, SESSIONS_CACHE_KEY, payload)
      } catch (error) {
        if (!active) {
          return
        }
        if (error instanceof AuthError && error.status === 401) {
          onUnauthorized()
        }
        setError(toApiError(error, 'Failed to load sessions'))
      } finally {
        if (active) {
          setLoading(false)
        }
      }
    }

    void load()

    return () => {
      active = false
    }
  }, [currentUserId, onUnauthorized])

  const upsertSession = useCallback((nextSession: SessionListItem) => {
    setSessions((current) => {
      const existingIndex = current.findIndex((session) => session.id === nextSession.id)
      if (existingIndex < 0) {
        const next = [nextSession, ...current]
        writeLocalApiCache(currentUserId, SESSIONS_CACHE_KEY, next)
        return next
      }

      const merged = current.map((session, index) =>
        index === existingIndex ? nextSession : session,
      )

      writeLocalApiCache(currentUserId, SESSIONS_CACHE_KEY, merged)
      return merged
    })
  }, [currentUserId])

  const refresh = useCallback(async () => {
    if (!currentUserId) {
      setSessions([])
      setLoading(false)
      setError(null)
      return
    }

    setLoading(true)
    setError(null)
    try {
      const response = await ensureOk(
        await authenticatedFetch('/api/sessions'),
        'Failed to load sessions',
      )
      const payload = (await response.json()) as SessionListItem[]
      setSessions(payload)
      writeLocalApiCache(currentUserId, SESSIONS_CACHE_KEY, payload)
    } catch (error) {
      if (error instanceof AuthError && error.status === 401) {
        onUnauthorized()
      }
      setError(toApiError(error, 'Failed to load sessions'))
      throw error
    } finally {
      setLoading(false)
    }
  }, [currentUserId, onUnauthorized])

  const deleteSession = useCallback(async (sessionId: string) => {
    await ensureOk(await authenticatedFetch(`/api/sessions/${sessionId}`, {
      method: 'DELETE',
    }), 'Failed to delete session')

    removeLocalApiCache(currentUserId, `chat:session:${sessionId}`)
    setSessions((current) => {
      const next = current.filter((session) => session.id !== sessionId)
      writeLocalApiCache(currentUserId, SESSIONS_CACHE_KEY, next)
      return next
    })
  }, [currentUserId])

  const renameSession = useCallback(async (sessionId: string, title: string) => {
    const response = await ensureOk(await authenticatedFetch(`/api/sessions/${sessionId}`, {
      method: 'PUT',
      body: JSON.stringify({ title }),
    }), 'Failed to rename session')

    const payload = (await response.json()) as SessionListItem
    upsertSession(payload)
    return payload
  }, [upsertSession])

  return {
    sessions,
    loading,
    error: error?.message ?? null,
    errorCode: error?.code ?? null,
    refresh,
    deleteSession,
    upsertSession,
    renameSession,
  }
}
