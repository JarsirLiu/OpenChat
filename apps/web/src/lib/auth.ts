import { ApiError, createApiError } from './apiError'
import { clearLocalApiCache, readLocalApiCache, writeLocalApiCache } from './localApiCache'

export interface AuthUser {
  id: string
  username: string
  email: string
  is_admin?: boolean
}

interface UserInfoResponse {
  user_info: AuthUser
}

interface CsrfResponse {
  csrf_token: string
}

export class AuthError extends ApiError {
  constructor(message: string, status: number, code?: string | null) {
    super(message, {
      status,
      code,
      category: status === 401 ? 'authentication' : 'authorization',
      retryable: false,
    })
  }
}

let refreshInFlight: Promise<void> | null = null
let csrfToken: string | null = null

const AUTH_USER_CACHE_KEY = 'auth:me'
const AUTH_USER_CACHE_TTL_MS = 30_000

function isUnsafeMethod(method?: string) {
  const normalized = (method ?? 'GET').toUpperCase()
  return normalized !== 'GET' && normalized !== 'HEAD' && normalized !== 'OPTIONS' && normalized !== 'TRACE'
}

export const clearAuthData = () => {
  csrfToken = null
  clearLocalApiCache()
}

async function ensureCsrfToken(forceRefresh = false) {
  if (!forceRefresh && csrfToken) {
    return csrfToken
  }

  const response = await fetch('/api/auth/csrf', {
    credentials: 'same-origin',
  })

  if (!response.ok) {
    throw new AuthError('Failed to initialize secure session', response.status)
  }

  const payload = (await response.json()) as CsrfResponse
  csrfToken = payload.csrf_token
  return csrfToken
}

async function fetchWithSession(url: string, options: RequestInit = {}) {
  const headers = new Headers(options.headers ?? {})
  if (options.body && !(options.body instanceof FormData) && !headers.has('Content-Type')) {
    headers.set('Content-Type', 'application/json')
  }
  if (isUnsafeMethod(options.method)) {
    headers.set('X-CSRF-Token', await ensureCsrfToken())
  }

  return fetch(url, {
    ...options,
    credentials: 'same-origin',
    headers,
  })
}

async function refreshSession() {
  const response = await fetchWithSession('/api/auth/refresh', {
    method: 'POST',
  })

  if (!response.ok) {
    clearAuthData()
    throw new AuthError('Authentication required', response.status)
  }
}

async function parseUserResponse(response: Response, fallbackMessage: string) {
  if (!response.ok) {
    const apiError = await createApiError(response, fallbackMessage)
    throw new AuthError(apiError.message, response.status, apiError.code)
  }

  const payload = (await response.json()) as UserInfoResponse
  return payload.user_info
}

export async function login(account: string, password: string) {
  try {
    await ensureCsrfToken()
    const response = await fetchWithSession('/api/auth/login', {
      method: 'POST',
      body: JSON.stringify({ account, password }),
    })
    const user = await parseUserResponse(response, 'Login failed')
    writeLocalApiCache(user.id, AUTH_USER_CACHE_KEY, user)
    return user
  } catch (error) {
    if (error instanceof AuthError) {
      throw error
    }
    throw new AuthError('OpenChat backend is not ready yet. Please wait a moment and try again.', 503)
  }
}

export async function register(email: string, password: string, username?: string) {
  try {
    await ensureCsrfToken()
    const response = await fetchWithSession('/api/auth/register', {
      method: 'POST',
      body: JSON.stringify({ email, password, username }),
    })
    const user = await parseUserResponse(response, 'Registration failed')
    writeLocalApiCache(user.id, AUTH_USER_CACHE_KEY, user)
    return user
  } catch (error) {
    if (error instanceof AuthError) {
      throw error
    }
    throw new AuthError('OpenChat backend is not ready yet. Please wait a moment and try again.', 503)
  }
}

export async function authenticatedFetch(url: string, options: RequestInit = {}) {
  const response = await fetchWithSession(url, options)
  if (response.status !== 401 || url === '/api/auth/refresh') {
    return response
  }

  if (!refreshInFlight) {
    refreshInFlight = refreshSession().finally(() => {
      refreshInFlight = null
    })
  }

  await refreshInFlight
  const retried = await fetchWithSession(url, options)
  if (retried.status === 401) {
    clearAuthData()
    throw new AuthError('Authentication required', 401)
  }
  return retried
}

export async function fetchCurrentUser() {
  const response = await authenticatedFetch('/api/auth/me')
  if (!response.ok) {
    if (response.status === 401) {
      clearAuthData()
      return null
    }
    const apiError = await createApiError(response, 'Authentication failed')
    throw new AuthError(apiError.message, response.status, apiError.code)
  }

  const payload = (await response.json()) as UserInfoResponse
  writeLocalApiCache(payload.user_info.id, AUTH_USER_CACHE_KEY, payload.user_info)
  return payload.user_info
}

export function readCachedCurrentUser() {
  if (typeof window === 'undefined') {
    return null
  }

  for (let index = window.localStorage.length - 1; index >= 0; index -= 1) {
    const key = window.localStorage.key(index)
    if (!key || !key.includes(`:${AUTH_USER_CACHE_KEY}`)) {
      continue
    }

    try {
      const parsed = JSON.parse(window.localStorage.getItem(key) ?? '{}') as {
        userId?: string
      }
      const cached = readLocalApiCache<AuthUser>(
        parsed.userId,
        AUTH_USER_CACHE_KEY,
        AUTH_USER_CACHE_TTL_MS,
      )
      if (cached?.fresh) {
        return cached.data
      }
    } catch {
      window.localStorage.removeItem(key)
    }
  }

  return null
}

export async function logout() {
  try {
    await ensureCsrfToken()
    await fetchWithSession('/api/auth/logout', {
      method: 'POST',
    })
  } finally {
    clearAuthData()
  }
}
