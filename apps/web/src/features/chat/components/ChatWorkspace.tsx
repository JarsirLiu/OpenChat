import { useEffect, useRef, useState } from 'react'
import { Conversation } from '@openchat/ui'
import type { AuthUser } from '../../../lib/auth'
import { isProviderConfigurationError } from '../../../lib/apiError'
import { ChatComposer } from './ChatComposer'
import { ChatHeader } from './ChatHeader'
import { ChatLanding } from './ChatLanding'
import { ProviderSettingsDialog } from './ProviderSettingsDialog'
import { ChatSidebar } from './ChatSidebar'
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
      isProviderConfigurationError(requestErrorCode ? { code: requestErrorCode } : null) ||
      isProviderConfigurationError(catalogErrorCode ? { code: catalogErrorCode } : null)
    ) {
      setSettingsOpen(true)
    }
  }, [catalogErrorCode, requestErrorCode])

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

  return (
    <div className="flex h-screen overflow-hidden bg-white font-sans dark:bg-[#121212]">
      <ChatSidebar
        sessions={sessions}
        currentSessionId={currentSession?.id ?? ''}
        loading={sessionsLoading}
        error={sessionsError}
        currentUserInitial={(
          currentUser.username?.slice(0, 1) || currentUser.email.slice(0, 1)
        ).toUpperCase()}
        onSelect={handleSelectSession}
        onNewSession={startNewSession}
        onDeleteSession={handleDeleteSession}
        onLogout={onLogout}
      />

      <div className="flex min-w-0 flex-1 flex-col bg-white dark:bg-[#121212]">
        <ChatHeader 
          title={currentSession?.title?.trim() || '新对话'} 
          onDelete={() => currentSession && handleDeleteSession(currentSession.id)}
          settingsOpen={settingsOpen}
          onOpenSettings={() => setSettingsOpen(true)}
          onRename={openRenameDialog}
        />

        <div ref={scrollRef} className="flex-1 overflow-y-auto">
          <div className="mx-auto w-full max-w-[800px] py-6">
            {runtimeState.messages.length === 0 ? (
              <ChatLanding
                username={currentUser.username}
                providerInitial={selectedTextModel?.display_name?.slice(0, 1).toUpperCase() ?? 'O'}
              />
            ) : (
              <Conversation state={runtimeState} requestPending={requestPending} />
            )}
          </div>
        </div>

        <div className="p-4">
          <div className="mx-auto max-w-[800px]">
            <ChatComposer
              value={input}
              pending={pending}
              disabled={!selectedTextModel || catalogLoading}
              imageTools={imageMenuItems}
              selectedImageToolKey={selectedImageToolKey}
              selectedImageToolLabel={selectedImageTool?.display_name ?? '不使用图片工具'}
              selectedImageToolAvailable={selectedImageTool?.available ?? true}
              models={textMenuItems}
              selectedModelKey={selectedTextModelId}
              selectedModelLabel={selectedTextModel?.display_name ?? '选择模型'}
              selectedModelProvider={selectedTextModel?.display_provider ?? 'M'}
              modelLoading={catalogLoading}
              imageToolLoading={catalogLoading}
              attachments={attachments}
              placeholder={
                !selectedTextModel
                  ? '先选择一个对话模型'
                  : selectedTextModel.available !== false
                    ? '提问或继续追问，按 Enter 发送，Shift + Enter 换行'
                    : '提问或继续追问。若要对话，请先在右侧边栏配置 API Key'
              }
              canUploadImages={selectedModelSupportsImageInputs}
              onChange={setInput}
              onClearImageTool={() => setSelectedImageToolKey(null)}
              onRemoveAttachment={removeAttachment}
              onSelectImageTool={setSelectedImageToolKey}
              onSelectModel={setSelectedTextModelId}
              onInterrupt={handleInterruptTurn}
              onSubmit={handleSubmit}
              onUploadImages={uploadImages}
            />
          </div>
        </div>
      </div>

      <ProviderSettingsDialog
        isOpen={settingsOpen}
        onClose={() => setSettingsOpen(false)}
        onSaved={refreshCatalog}
        onUnauthorized={onUnauthorized}
      />

      {renameDialogOpen ? (
        <>
          <div
            className="fixed inset-0 z-40 bg-black/25"
            onClick={() => !renamePending && setRenameDialogOpen(false)}
            aria-hidden="true"
          />
          <div className="fixed left-1/2 top-1/2 z-50 w-[min(92vw,460px)] -translate-x-1/2 -translate-y-1/2 rounded-2xl border border-gray-100 bg-white p-5 shadow-[0_24px_80px_rgba(0,0,0,0.18)] dark:border-gray-800 dark:bg-[#171717]">
            <div className="space-y-4">
              <div>
                <h3 className="text-[16px] font-semibold text-gray-900 dark:text-white">
                  重命名会话
                </h3>
                <p className="mt-1 text-[13px] text-gray-500 dark:text-gray-400">
                  给这个对话换一个更清晰的名字。
                </p>
              </div>

              <input
                type="text"
                value={renameValue}
                onChange={(event) => setRenameValue(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === 'Enter') {
                    event.preventDefault()
                    void submitRename()
                  }
                }}
                disabled={renamePending}
                autoFocus
                maxLength={40}
                placeholder="输入会话名称"
                className="h-11 w-full rounded-xl border border-gray-200 bg-white px-4 text-[14px] text-gray-900 outline-none transition focus:border-gray-300 dark:border-gray-700 dark:bg-[#1a1a1a] dark:text-gray-100 dark:focus:border-gray-500"
              />

              <div className="flex items-center justify-end gap-2">
                <button
                  type="button"
                  onClick={() => setRenameDialogOpen(false)}
                  disabled={renamePending}
                  className="inline-flex h-10 items-center rounded-lg border border-gray-200 bg-white px-4 text-[14px] text-gray-700 transition hover:bg-gray-50 disabled:cursor-not-allowed disabled:opacity-60 dark:border-gray-700 dark:bg-[#1a1a1a] dark:text-gray-200 dark:hover:bg-gray-800"
                >
                  取消
                </button>
                <button
                  type="button"
                  onClick={() => void submitRename()}
                  disabled={renamePending || !renameValue.trim()}
                  className="inline-flex h-10 items-center rounded-lg bg-black px-4 text-[14px] text-white transition hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60 dark:bg-white dark:text-black"
                >
                  {renamePending ? '保存中' : '确定'}
                </button>
              </div>
            </div>
          </div>
        </>
      ) : null}
    </div>
  )
}
