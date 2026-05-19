import { useCallback, useEffect, useState } from 'react'
import {
  AuthError,
  fetchCurrentUser,
  login,
  logout,
  register,
  type AuthUser,
} from '../../lib/auth'

export function useAuthSession() {
  const [authLoading, setAuthLoading] = useState(true)
  const [authSubmitting, setAuthSubmitting] = useState(false)
  const [authError, setAuthError] = useState<string | null>(null)
  const [currentUser, setCurrentUser] = useState<AuthUser | null>(null)

  useEffect(() => {
    if (!authError) {
      return
    }

    const timeoutId = window.setTimeout(() => {
      setAuthError((current) => (current === authError ? null : current))
    }, 4000)

    return () => {
      window.clearTimeout(timeoutId)
    }
  }, [authError])

  useEffect(() => {
    let active = true

    const bootstrapAuth = async () => {
      try {
        const user = await fetchCurrentUser()
        if (!active) {
          return
        }
        setCurrentUser(user)
      } catch (error) {
        if (!active) {
          return
        }
        setCurrentUser(null)
        if (error instanceof AuthError && error.status === 401) {
          setAuthError(null)
          return
        }
        setAuthError(error instanceof Error ? error.message : 'Authentication failed')
      } finally {
        if (active) {
          setAuthLoading(false)
        }
      }
    }

    void bootstrapAuth()

    return () => {
      active = false
    }
  }, [])

  const handleUnauthorized = useCallback(() => {
    setCurrentUser(null)
  }, [])

  const handleLogin = useCallback(async (account: string, password: string) => {
    try {
      setAuthSubmitting(true)
      setAuthError(null)
      const user = await login(account.trim(), password)
      setCurrentUser(user)
    } catch (error) {
      setAuthError(error instanceof Error ? error.message : 'Login failed')
    } finally {
      setAuthSubmitting(false)
    }
  }, [])

  const handleRegister = useCallback(async (email: string, password: string, username?: string) => {
    try {
      setAuthSubmitting(true)
      setAuthError(null)
      const user = await register(email.trim(), password, username?.trim() || undefined)
      setCurrentUser(user)
    } catch (error) {
      setAuthError(error instanceof Error ? error.message : 'Registration failed')
    } finally {
      setAuthSubmitting(false)
    }
  }, [])

  const handleLogout = useCallback(async () => {
    await logout()
    setCurrentUser(null)
  }, [])

  return {
    authLoading,
    authSubmitting,
    authError,
    currentUser,
    handleUnauthorized,
    handleLogin,
    handleRegister,
    handleLogout,
  }
}
