const ACCESS_TOKEN_KEY = 'openchat_access_token'
const REFRESH_TOKEN_KEY = 'openchat_refresh_token'
const USER_INFO_KEY = 'openchat_user_info'

export interface AuthUser {
  id: string
  username: string
  email: string
  is_admin?: boolean
}

interface AuthResponse {
  status: string
  token: string
  refresh_token: string
  user_info: AuthUser
}

export class AuthError extends Error {
  status: number

  constructor(message: string, status: number) {
    super(message)
    this.status = status
  }
}

let refreshInFlight: Promise<string> | null = null

export const getAccessToken = () => window.localStorage.getItem(ACCESS_TOKEN_KEY)
export const getRefreshToken = () => window.localStorage.getItem(REFRESH_TOKEN_KEY)

export const getStoredUser = (): AuthUser | null => {
  const raw = window.localStorage.getItem(USER_INFO_KEY)
  return raw ? (JSON.parse(raw) as AuthUser) : null
}

export const saveAuthData = (payload: AuthResponse) => {
  window.localStorage.setItem(ACCESS_TOKEN_KEY, payload.token)
  window.localStorage.setItem(REFRESH_TOKEN_KEY, payload.refresh_token)
  window.localStorage.setItem(USER_INFO_KEY, JSON.stringify(payload.user_info))
}

export const clearAuthData = () => {
  window.localStorage.removeItem(ACCESS_TOKEN_KEY)
  window.localStorage.removeItem(REFRESH_TOKEN_KEY)
  window.localStorage.removeItem(USER_INFO_KEY)
}

export async function login(account: string, password: string) {
  let response: Response
  try {
    response = await fetch('/api/auth/login', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ account, password }),
    })
  } catch {
    throw new AuthError('OpenChat backend is not ready yet. Please wait a moment and try again.', 503)
  }

  if (!response.ok) {
    const payload = (await response.json().catch(() => null)) as { message?: string } | null
    throw new AuthError(payload?.message ?? 'Login failed', response.status)
  }

  const payload = (await response.json()) as AuthResponse
  saveAuthData(payload)
  return payload.user_info
}

export async function register(email: string, password: string, username?: string) {
  let response: Response
  try {
    response = await fetch('/api/auth/register', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ email, password, username }),
    })
  } catch {
    throw new AuthError('OpenChat backend is not ready yet. Please wait a moment and try again.', 503)
  }

  if (!response.ok) {
    const payload = (await response.json().catch(() => null)) as { message?: string } | null
    throw new AuthError(payload?.message ?? 'Registration failed', response.status)
  }

  const payload = (await response.json()) as AuthResponse
  saveAuthData(payload)
  return payload.user_info
}

async function refreshAccessToken() {
  const refreshToken = getRefreshToken()
  if (!refreshToken) {
    throw new AuthError('Authentication required', 401)
  }

  const response = await fetch('/api/auth/refresh', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ refresh_token: refreshToken }),
  })

  if (!response.ok) {
    clearAuthData()
    throw new AuthError('Authentication required', response.status)
  }

  const payload = (await response.json()) as AuthResponse
  saveAuthData(payload)
  return payload.token
}

export async function authenticatedFetch(url: string, options: RequestInit = {}) {
  const doFetch = async (token: string | null) => {
    const headers = new Headers(options.headers ?? {})
    if (options.body && !(options.body instanceof FormData) && !headers.has('Content-Type')) {
      headers.set('Content-Type', 'application/json')
    }
    if (token) {
      headers.set('Authorization', `Bearer ${token}`)
    }
    return fetch(url, {
      ...options,
      headers,
    })
  }

  let response = await doFetch(getAccessToken())
  if (response.status !== 401) {
    return response
  }

  if (!refreshInFlight) {
    refreshInFlight = refreshAccessToken().finally(() => {
      refreshInFlight = null
    })
  }

  const nextToken = await refreshInFlight
  response = await doFetch(nextToken)
  if (response.status === 401) {
    clearAuthData()
    throw new AuthError('Authentication required', 401)
  }
  return response
}

export async function fetchCurrentUser() {
  const token = getAccessToken()
  if (!token) {
    return null
  }

  const response = await authenticatedFetch('/api/auth/me')
  if (!response.ok) {
    clearAuthData()
    return null
  }

  const payload = (await response.json()) as { user_info: AuthUser }
  window.localStorage.setItem(USER_INFO_KEY, JSON.stringify(payload.user_info))
  return payload.user_info
}

export async function logout() {
  const refreshToken = getRefreshToken()
  if (refreshToken) {
    await fetch('/api/auth/logout', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ refresh_token: refreshToken }),
    }).catch(() => undefined)
  }
  clearAuthData()
}
