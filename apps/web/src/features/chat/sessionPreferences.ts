import type { CatalogTool, SessionPreference, SessionPreferenceMap } from './types'

const buildSessionStorageKey = (userId: string | null) =>
  `openchat:session-id:${userId ?? 'anonymous'}`

const buildSessionPreferencesStorageKey = (userId: string | null) =>
  `openchat:session-preferences:${userId ?? 'anonymous'}`

export const createSessionId = () => `sess_${crypto.randomUUID()}`

export const imageToolKeyOf = (tool: Pick<CatalogTool, 'id' | 'model_config_id'>) =>
  `${tool.id}::${tool.model_config_id}`

export const readStoredSessionId = (userId: string | null) =>
  window.localStorage.getItem(buildSessionStorageKey(userId))

export const writeStoredSessionId = (userId: string | null, sessionId: string) => {
  window.localStorage.setItem(buildSessionStorageKey(userId), sessionId)
}

export const readSessionPreferences = (userId: string | null): SessionPreferenceMap => {
  const raw = window.localStorage.getItem(buildSessionPreferencesStorageKey(userId))
  if (!raw) {
    return {}
  }

  try {
    return JSON.parse(raw) as SessionPreferenceMap
  } catch {
    return {}
  }
}

export const writeSessionPreferences = (
  userId: string | null,
  preferences: SessionPreferenceMap,
) => {
  window.localStorage.setItem(
    buildSessionPreferencesStorageKey(userId),
    JSON.stringify(preferences),
  )
}

export const getInitialSessionPreference = (
  userId: string | null,
  sessionId: string,
): SessionPreference =>
  readSessionPreferences(userId)[sessionId] ?? {
    textModelId: null,
    imageToolKey: null,
  }

export const ensureSessionPreference = (userId: string | null, sessionId: string) => {
  const preferences = readSessionPreferences(userId)
  if (preferences[sessionId]) {
    return preferences[sessionId]
  }

  const nextPreference = getInitialSessionPreference(userId, sessionId)
  writeSessionPreferences(userId, {
    ...preferences,
    [sessionId]: nextPreference,
  })
  return nextPreference
}

export const updateSessionPreference = (
  userId: string | null,
  sessionId: string,
  nextPreference: SessionPreference,
) => {
  const preferences = readSessionPreferences(userId)
  writeSessionPreferences(userId, {
    ...preferences,
    [sessionId]: nextPreference,
  })
}
