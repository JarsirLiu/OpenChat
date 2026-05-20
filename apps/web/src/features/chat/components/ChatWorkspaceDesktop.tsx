import type { ReactNode } from 'react'
import { ChatHeader } from './ChatHeader'
import { ChatSidebar } from './ChatSidebar'
import { ChatWorkspaceMainPane, type ChatWorkspaceMainPaneProps } from './ChatWorkspaceShared'

interface ChatWorkspaceDesktopProps {
  currentSessionId: string
  currentUserInitial: string
  sessions: Parameters<typeof ChatSidebar>[0]['sessions']
  sessionsLoading: boolean
  sessionsError: string | null
  title: string
  settingsOpen: boolean
  onDeleteCurrentSession?: () => void
  onLogout: () => void
  onNewSession: () => void
  onOpenRename: () => void
  onOpenSettings: () => void
  onSelectSession: (sessionId: string) => void
  onDeleteSession: (sessionId: string) => void
  mainPane: ChatWorkspaceMainPaneProps
  settingsPanel?: ReactNode
}

export function ChatWorkspaceDesktop({
  currentSessionId,
  currentUserInitial,
  mainPane,
  onDeleteCurrentSession,
  onDeleteSession,
  onLogout,
  onNewSession,
  onOpenRename,
  onOpenSettings,
  onSelectSession,
  sessions,
  sessionsError,
  sessionsLoading,
  settingsOpen,
  settingsPanel,
  title,
}: ChatWorkspaceDesktopProps) {
  return (
    <div className="flex h-[100dvh] overflow-hidden bg-white font-sans dark:bg-[#121212]">
      <ChatSidebar
        sessions={sessions}
        currentSessionId={currentSessionId}
        loading={sessionsLoading}
        error={sessionsError}
        currentUserInitial={currentUserInitial}
        onSelect={onSelectSession}
        onNewSession={onNewSession}
        onDeleteSession={onDeleteSession}
        onLogout={onLogout}
      />

      <div className="flex min-w-0 flex-1 flex-col bg-white dark:bg-[#121212]">
        <ChatHeader
          title={title}
          onDelete={onDeleteCurrentSession}
          settingsOpen={settingsOpen}
          onOpenSettings={onOpenSettings}
          onRename={onOpenRename}
        />
        <ChatWorkspaceMainPane {...mainPane} desktop />
      </div>
      {settingsPanel}
    </div>
  )
}
