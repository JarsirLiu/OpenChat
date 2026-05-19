import { useCallback, useEffect, useState } from 'react'
import { authenticatedFetch, AuthError } from '../../lib/auth'

export interface SessionListItem {
  id: string
  title: string | null
  status: string
  createdAt: string
  updatedAt: string
}

export function useSessions(currentUserId: string | null, onUnauthorized: () => void) {
  const [sessions, setSessions] = useState<SessionListItem[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let active = true

    const load = async () => {
      if (!currentUserId) {
        setSessions([])
        setLoading(false)
        setError(null)
        return
      }

      setLoading(true)
      setError(null)
      try {
        const response = await authenticatedFetch('/api/sessions')
        if (!response.ok) {
          throw new Error('Failed to load sessions')
        }
        const payload = (await response.json()) as SessionListItem[]
        if (!active) {
          return
        }
        setSessions(payload)
      } catch (error) {
        if (!active) {
          return
        }
        if (error instanceof AuthError && error.status === 401) {
          onUnauthorized()
        }
        setError(error instanceof Error ? error.message : 'Failed to load sessions')
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
        return [nextSession, ...current]
      }

      const merged = current.map((session, index) =>
        index === existingIndex ? nextSession : session,
      )

      return merged.sort((left, right) => right.updatedAt.localeCompare(left.updatedAt))
    })
  }, [])

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
      const response = await authenticatedFetch('/api/sessions')
      if (!response.ok) {
        throw new Error('Failed to load sessions')
      }
      const payload = (await response.json()) as SessionListItem[]
      setSessions(payload)
    } catch (error) {
      if (error instanceof AuthError && error.status === 401) {
        onUnauthorized()
      }
      setError(error instanceof Error ? error.message : 'Failed to load sessions')
      throw error
    } finally {
      setLoading(false)
    }
  }, [currentUserId, onUnauthorized])

  const deleteSession = useCallback(async (sessionId: string) => {
    const response = await authenticatedFetch(`/api/sessions/${sessionId}`, {
      method: 'DELETE',
    })

    if (!response.ok) {
      const payload = (await response.json().catch(() => null)) as { message?: string } | null
      throw new Error(payload?.message ?? 'Failed to delete session')
    }

    setSessions((current) => current.filter((session) => session.id !== sessionId))
  }, [])

  const renameSession = useCallback(async (sessionId: string, title: string) => {
    const response = await authenticatedFetch(`/api/sessions/${sessionId}`, {
      method: 'PUT',
      body: JSON.stringify({ title }),
    })

    if (!response.ok) {
      const payload = (await response.json().catch(() => null)) as { message?: string } | null
      throw new Error(payload?.message ?? 'Failed to rename session')
    }

    const payload = (await response.json()) as SessionListItem
    upsertSession(payload)
    return payload
  }, [upsertSession])

  return {
    sessions,
    loading,
    error,
    refresh,
    deleteSession,
    upsertSession,
    renameSession,
  }
}
