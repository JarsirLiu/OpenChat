import { useNavigate, useParams } from 'react-router-dom'
import type { AuthUser } from '../../../lib/auth'
import {
  buildChatSessionPath,
  CHAT_HOME_PATH,
  normalizeChatSessionId,
} from '../chatNavigation'
import { ChatWorkspace } from './ChatWorkspace'

interface ChatWorkspaceRouteProps {
  currentUser: AuthUser
  onLogout: () => Promise<void>
  onUnauthorized: () => void
}

export function ChatWorkspaceRoute({
  currentUser,
  onLogout,
  onUnauthorized,
}: ChatWorkspaceRouteProps) {
  const navigate = useNavigate()
  const { sessionId } = useParams<{ sessionId: string }>()
  const activeSessionId = normalizeChatSessionId(sessionId)

  return (
    <ChatWorkspace
      currentUser={currentUser}
      onLogout={onLogout}
      onUnauthorized={onUnauthorized}
      activeSessionId={activeSessionId}
      onOpenSession={(nextSessionId) => navigate(buildChatSessionPath(nextSessionId))}
      onOpenNewSession={() => navigate(CHAT_HOME_PATH)}
    />
  )
}
