import { useState } from 'react'
import { ChatHeader } from './ChatHeader'
import { ChatSidebar } from './ChatSidebar'
import { ChatWorkspaceMainPane, type ChatWorkspaceMainPaneProps } from './ChatWorkspaceShared'

interface ChatWorkspaceMobileProps {
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
}

export function ChatWorkspaceMobile({
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
  title,
}: ChatWorkspaceMobileProps) {
  const [sidebarOpen, setSidebarOpen] = useState(false)

  return (
    <div className="flex h-[100dvh] overflow-hidden overscroll-none bg-white font-sans dark:bg-[#121212]">
      {sidebarOpen ? (
        <>
          <div
            className="fixed inset-0 z-30 bg-black/30"
            onClick={() => setSidebarOpen(false)}
            aria-hidden="true"
          />
          <div className="fixed inset-y-0 left-0 z-40">
            <ChatSidebar
              sessions={sessions}
              currentSessionId={currentSessionId}
              loading={sessionsLoading}
              error={sessionsError}
              currentUserInitial={currentUserInitial}
              mobile
              onClose={() => setSidebarOpen(false)}
              onSelect={(sessionId) => {
                setSidebarOpen(false)
                onSelectSession(sessionId)
              }}
              onNewSession={() => {
                setSidebarOpen(false)
                onNewSession()
              }}
              onDeleteSession={onDeleteSession}
              onLogout={onLogout}
            />
          </div>
        </>
      ) : null}

      <div className="flex min-w-0 flex-1 flex-col overflow-hidden overscroll-none bg-white dark:bg-[#121212]">
        <ChatHeader
          title={title}
          onDelete={onDeleteCurrentSession}
          settingsOpen={settingsOpen}
          onOpenSidebar={() => {
            setSidebarOpen(true)
          }}
          onOpenSettings={onOpenSettings}
          onRename={onOpenRename}
        />
        <ChatWorkspaceMainPane {...mainPane} />
      </div>
    </div>
  )
}
