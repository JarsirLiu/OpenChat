import type { CatalogTool, SessionPreference, SessionPreferenceMap } from './types'

export const SESSION_STORAGE_KEY = 'openchat:session-id'
const SESSION_PREFERENCES_STORAGE_KEY = 'openchat:session-preferences'

export const createSessionId = () => `sess_${crypto.randomUUID()}`

export const imageToolKeyOf = (tool: Pick<CatalogTool, 'id' | 'model_config_id'>) =>
  `${tool.id}::${tool.model_config_id}`

export const readStoredSessionId = () => window.localStorage.getItem(SESSION_STORAGE_KEY)

export const writeStoredSessionId = (sessionId: string) => {
  window.localStorage.setItem(SESSION_STORAGE_KEY, sessionId)
}

export const readSessionPreferences = (): SessionPreferenceMap => {
  const raw = window.localStorage.getItem(SESSION_PREFERENCES_STORAGE_KEY)
  if (!raw) {
    return {}
  }

  try {
    return JSON.parse(raw) as SessionPreferenceMap
  } catch {
    return {}
  }
}

export const writeSessionPreferences = (preferences: SessionPreferenceMap) => {
  window.localStorage.setItem(SESSION_PREFERENCES_STORAGE_KEY, JSON.stringify(preferences))
}

export const getInitialSessionPreference = (sessionId: string): SessionPreference =>
  readSessionPreferences()[sessionId] ?? {
    textModelId: null,
    imageToolKey: null,
  }

export const ensureSessionPreference = (sessionId: string) => {
  const preferences = readSessionPreferences()
  if (preferences[sessionId]) {
    return preferences[sessionId]
  }

  const nextPreference = getInitialSessionPreference(sessionId)
  writeSessionPreferences({
    ...preferences,
    [sessionId]: nextPreference,
  })
  return nextPreference
}

export const updateSessionPreference = (sessionId: string, nextPreference: SessionPreference) => {
  const preferences = readSessionPreferences()
  writeSessionPreferences({
    ...preferences,
    [sessionId]: nextPreference,
  })
}
