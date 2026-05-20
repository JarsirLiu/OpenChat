import { useEffect, useRef, useState } from 'react'
import type { AuthUser } from '../../../lib/auth'
import { isProviderConfigurationError } from '../../../lib/apiError'
import { ProviderSettingsDialog } from './ProviderSettingsDialog'
import { ChatWorkspaceDesktop } from './ChatWorkspaceDesktop'
import { ChatWorkspaceMobile } from './ChatWorkspaceMobile'
import { RenameSessionDialog } from './ChatWorkspaceShared'
import { useChatWorkspace } from '../useChatWorkspace'

interface ChatWorkspaceProps {
  currentUser: AuthUser
  onLogout: () => Promise<void>
  onUnauthorized: () => void
  activeSessionId: string | null
  onOpenSession: (sessionId: string) => void
  onOpenNewSession: () => void
}

export function ChatWorkspace({
  currentUser,
  onLogout,
  onUnauthorized,
  activeSessionId,
  onOpenSession,
  onOpenNewSession,
}: ChatWorkspaceProps) {
  const [settingsOpen, setSettingsOpen] = useState(false)
  const [renameDialogOpen, setRenameDialogOpen] = useState(false)
  const [renameValue, setRenameValue] = useState('')
  const [renamePending, setRenamePending] = useState(false)
  const [isMobile, setIsMobile] = useState(() =>
    typeof window !== 'undefined' ? window.innerWidth < 1024 : false,
  )
  const hasAutoOpenedSettingsRef = useRef(false)
  const {
    catalogErrorCode,
    catalogLoading,
    currentSession,
    handleDeleteSession,
    handleInterruptTurn,
    handleRenameSession,
    handleSelectSession,
    handleSubmit,
    imageMenuItems,
    input,
    imageTools,
    pending,
    requestPending,
    requestErrorCode,
    historyHasMore,
    historyLoading,
    loadOlderHistory,
    attachments,
    removeAttachment,
    runtimeState,
    selectedImageTool,
    selectedImageToolKey,
    selectedModelSupportsImageInputs,
    selectedTextModel,
    selectedTextModelId,
    sessions,
    sessionsError,
    sessionsLoading,
    setInput,
    setSelectedImageToolKey,
    setSelectedTextModelId,
    startNewSession,
    textMenuItems,
    uploadImages,
    refreshCatalog,
  } = useChatWorkspace({
    currentUser,
    onUnauthorized,
    activeSessionId,
    onOpenSession,
    onOpenNewSession,
  })

  const scrollRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (typeof window === 'undefined') {
      return
    }

    const mediaQuery = window.matchMedia('(max-width: 1023px)')
    const update = () => setIsMobile(mediaQuery.matches)

    update()
    mediaQuery.addEventListener('change', update)
    return () => mediaQuery.removeEventListener('change', update)
  }, [])

  useEffect(() => {
    if (scrollRef.current && pending) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight
    }
  }, [runtimeState.messages, pending])

  useEffect(() => {
    const container = scrollRef.current
    if (!container) {
      return
    }

    let loading = false

    const handleScroll = () => {
      if (loading || historyLoading || !historyHasMore || container.scrollTop > 80) {
        return
      }

      loading = true
      const previousScrollHeight = container.scrollHeight
      const previousScrollTop = container.scrollTop

      void loadOlderHistory().then((loaded) => {
        requestAnimationFrame(() => {
          if (loaded) {
            const nextScrollHeight = container.scrollHeight
            container.scrollTop =
              previousScrollTop + (nextScrollHeight - previousScrollHeight)
          }
          loading = false
        })
      })
    }

    container.addEventListener('scroll', handleScroll)
    return () => {
      container.removeEventListener('scroll', handleScroll)
    }
  }, [historyHasMore, historyLoading, loadOlderHistory])

  useEffect(() => {
    if (
      runtimeState.error?.code === 'provider_authentication_failed' ||
      isProviderConfigurationError(requestErrorCode ? { code: requestErrorCode } : null) ||
      isProviderConfigurationError(catalogErrorCode ? { code: catalogErrorCode } : null)
    ) {
      setSettingsOpen(true)
    }
  }, [catalogErrorCode, requestErrorCode, runtimeState.error?.code])

  useEffect(() => {
    if (hasAutoOpenedSettingsRef.current || catalogLoading || settingsOpen) {
      return
    }

    if (selectedTextModel?.available === false) {
      hasAutoOpenedSettingsRef.current = true
      setSettingsOpen(true)
    }
  }, [catalogLoading, selectedTextModel?.available, settingsOpen])

  useEffect(() => {
    if (!renameDialogOpen) {
      return
    }
    setRenameValue(currentSession?.title?.trim() || '')
  }, [currentSession?.title, renameDialogOpen])

  const openRenameDialog = () => {
    if (!currentSession) {
      return
    }
    setRenameValue(currentSession.title?.trim() || '')
    setRenameDialogOpen(true)
  }

  const submitRename = async () => {
    if (!currentSession) {
      return
    }

    const normalizedTitle = renameValue.trim()
    if (!normalizedTitle) {
      return
    }

    setRenamePending(true)
    try {
      await handleRenameSession(currentSession.id, normalizedTitle)
      setRenameDialogOpen(false)
    } finally {
      setRenamePending(false)
    }
  }

  const isEmptyState = runtimeState.messages.length === 0
  const currentUserInitial = (
    currentUser.username?.slice(0, 1) || currentUser.email.slice(0, 1)
  ).toUpperCase()
  const currentTitle = currentSession?.title?.trim() || '新对话'
  const mainPane = {
    catalogLoading,
    currentUsername: currentUser.username,
    input,
    pending,
    requestPending,
    runtimeMessagesEmpty: isEmptyState,
    runtimeState,
    selectedImageToolAvailable: selectedImageTool?.available ?? true,
    selectedImageToolKey,
    selectedImageToolLabel: selectedImageTool?.display_name ?? '不使用图片工具',
    selectedModelDisplayName: selectedTextModel?.display_name ?? '选择模型',
    selectedModelProvider: selectedTextModel?.display_provider ?? 'M',
    selectedModelSupportsImageInputs,
    selectedProviderInitial: selectedTextModel?.display_name?.slice(0, 1).toUpperCase() ?? 'O',
    selectedTextModelAvailable: selectedTextModel?.available !== false,
    selectedTextModelId,
    imageMenuItems,
    textMenuItems,
    attachments,
    onChangeInput: setInput,
    onClearImageTool: () => setSelectedImageToolKey(null),
    onInterrupt: handleInterruptTurn,
    onLoadOlderHistory: loadOlderHistory,
    onRemoveAttachment: removeAttachment,
    onScrollContainerReady: (node: HTMLDivElement | null) => {
      scrollRef.current = node
    },
    onSelectImageTool: setSelectedImageToolKey,
    onSelectModel: setSelectedTextModelId,
    onSubmit: handleSubmit,
    onUploadImages: uploadImages,
  } as const
  const currentSessionDeleteHandler = currentSession
    ? () => handleDeleteSession(currentSession.id)
    : undefined

  return (
    <>
      {isMobile ? (
        <>
          <ChatWorkspaceMobile
            sessions={sessions}
            currentSessionId={currentSession?.id ?? ''}
            currentUserInitial={currentUserInitial}
            sessionsLoading={sessionsLoading}
            sessionsError={sessionsError}
            title={currentTitle}
            settingsOpen={settingsOpen}
            onDeleteCurrentSession={currentSessionDeleteHandler}
            onLogout={onLogout}
            onNewSession={startNewSession}
            onOpenRename={openRenameDialog}
            onOpenSettings={() => setSettingsOpen(true)}
            onSelectSession={handleSelectSession}
            onDeleteSession={handleDeleteSession}
            mainPane={mainPane}
          />
          <ProviderSettingsDialog
            isOpen={settingsOpen}
            onClose={() => setSettingsOpen(false)}
            onSaved={refreshCatalog}
            onUnauthorized={onUnauthorized}
            autoFocusApiKey
          />
        </>
      ) : (
        <ChatWorkspaceDesktop
          sessions={sessions}
          currentSessionId={currentSession?.id ?? ''}
          currentUserInitial={currentUserInitial}
          sessionsLoading={sessionsLoading}
          sessionsError={sessionsError}
          title={currentTitle}
          settingsOpen={settingsOpen}
          onDeleteCurrentSession={currentSessionDeleteHandler}
          onLogout={onLogout}
          onNewSession={startNewSession}
          onOpenRename={openRenameDialog}
          onOpenSettings={() => setSettingsOpen(true)}
          onSelectSession={handleSelectSession}
          onDeleteSession={handleDeleteSession}
          mainPane={mainPane}
          settingsPanel={
            <ProviderSettingsDialog
              isOpen={settingsOpen}
              onClose={() => setSettingsOpen(false)}
              onSaved={refreshCatalog}
              onUnauthorized={onUnauthorized}
              autoFocusApiKey
            />
          }
        />
      )}

      <RenameSessionDialog
        isOpen={renameDialogOpen}
        pending={renamePending}
        value={renameValue}
        onChange={setRenameValue}
        onClose={() => setRenameDialogOpen(false)}
        onSubmit={() => void submitRename()}
      />
    </>
  )
}
