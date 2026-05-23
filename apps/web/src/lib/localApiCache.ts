const CACHE_PREFIX = 'openchat:api-cache:v2'

interface CacheEnvelope<T> {
  version: 1
  userId: string
  storedAt: number
  data: T
}

const hasLocalStorage = () => typeof window !== 'undefined' && Boolean(window.localStorage)

const cacheKey = (userId: string, key: string) =>
  `${CACHE_PREFIX}:${encodeURIComponent(userId)}:${key}`

export function readLocalApiCache<T>(
  userId: string | null | undefined,
  key: string,
  ttlMs: number,
): { data: T; fresh: boolean } | null {
  if (!userId || !hasLocalStorage()) {
    return null
  }

  const storageKey = cacheKey(userId, key)
  const raw = window.localStorage.getItem(storageKey)
  if (!raw) {
    return null
  }

  try {
    const parsed = JSON.parse(raw) as CacheEnvelope<T>
    if (parsed.version !== 1 || parsed.userId !== userId || typeof parsed.storedAt !== 'number') {
      window.localStorage.removeItem(storageKey)
      return null
    }

    return {
      data: parsed.data,
      fresh: Date.now() - parsed.storedAt <= ttlMs,
    }
  } catch {
    window.localStorage.removeItem(storageKey)
    return null
  }
}

export function writeLocalApiCache<T>(
  userId: string | null | undefined,
  key: string,
  data: T,
) {
  if (!userId || !hasLocalStorage()) {
    return
  }

  try {
    const envelope: CacheEnvelope<T> = {
      version: 1,
      userId,
      storedAt: Date.now(),
      data,
    }
    window.localStorage.setItem(cacheKey(userId, key), JSON.stringify(envelope))
  } catch {
    // Local storage can be full or unavailable in private contexts. The app should
    // continue with network data if persistence is not available.
  }
}

export function removeLocalApiCache(userId: string | null | undefined, key: string) {
  if (!userId || !hasLocalStorage()) {
    return
  }

  window.localStorage.removeItem(cacheKey(userId, key))
}

export function clearLocalApiCache(userId?: string | null) {
  if (!hasLocalStorage()) {
    return
  }

  const prefix = userId ? `${CACHE_PREFIX}:${encodeURIComponent(userId)}:` : `${CACHE_PREFIX}:`
  for (let index = window.localStorage.length - 1; index >= 0; index -= 1) {
    const key = window.localStorage.key(index)
    if (key?.startsWith(prefix)) {
      window.localStorage.removeItem(key)
    }
  }
}
