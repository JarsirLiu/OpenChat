export const CHAT_HOME_PATH = '/'
export const CHAT_INDEX_PATH = '/chat'
export const CHAT_SESSION_PATH = '/chat/:sessionId'

export const buildChatSessionPath = (sessionId: string) => `/chat/${sessionId}`

export const normalizeChatSessionId = (sessionId: string | undefined) =>
  sessionId && sessionId.trim() ? sessionId : null
