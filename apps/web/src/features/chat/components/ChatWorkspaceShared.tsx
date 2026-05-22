import { ThreadConversation } from '@openchat/ui'
import type { ChatRuntimeV2State } from '@openchat/chat-core'
import { ChatComposer } from './ChatComposer'
import { ChatLanding } from './ChatLanding'

export interface ChatWorkspaceMainPaneProps {
  catalogLoading: boolean
  currentUsername?: string | null
  input: string
  pending: boolean
  runtimeMessagesEmpty: boolean
  runtimeV2State: ChatRuntimeV2State
  optimisticUserPreviews: Record<
    string,
    {
      sessionId: string
      text: string
      createdAt: string
    }
  >
  selectedImageToolAvailable: boolean
  selectedImageToolKey: string | null
  selectedImageToolLabel: string
  selectedModelDisplayName?: string | null
  selectedModelProvider?: string | null
  selectedModelSupportsImageInputs: boolean
  selectedTextModelAvailable: boolean
  selectedTextModelId: string | null
  imageMenuItems: Parameters<typeof ChatComposer>[0]['imageTools']
  textMenuItems: Parameters<typeof ChatComposer>[0]['models']
  attachments: Parameters<typeof ChatComposer>[0]['attachments']
  selectedProviderInitial: string
  onChangeInput: (value: string) => void
  onClearImageTool: () => void
  onInterrupt: () => void
  onLoadOlderHistory: () => Promise<boolean>
  onRemoveAttachment: (attachmentId: string) => void
  onScrollContainerReady: (node: HTMLDivElement | null) => void
  onSelectImageTool: (toolKey: string | null) => void
  onSelectModel: (modelKey: string) => void
  onSubmit: () => void
  onUploadImages: (files: File[]) => Promise<void>
  desktop?: boolean
}

export function ChatWorkspaceMainPane({
  attachments,
  catalogLoading,
  currentUsername,
  imageMenuItems,
  input,
  onChangeInput,
  onClearImageTool,
  onInterrupt,
  onRemoveAttachment,
  onScrollContainerReady,
  onSelectImageTool,
  onSelectModel,
  onSubmit,
  onUploadImages,
  pending,
  runtimeMessagesEmpty,
  runtimeV2State,
  optimisticUserPreviews,
  desktop = false,
  selectedImageToolAvailable,
  selectedImageToolKey,
  selectedImageToolLabel,
  selectedModelDisplayName,
  selectedModelProvider,
  selectedModelSupportsImageInputs,
  selectedProviderInitial,
  selectedTextModelAvailable,
  selectedTextModelId,
  textMenuItems,
}: ChatWorkspaceMainPaneProps) {
  return (
    <>
      <div
        ref={onScrollContainerReady}
        className="relative z-0 min-h-0 flex-1 overflow-y-auto overscroll-y-contain px-3 sm:px-4"
      >
        <div
          className={`mx-auto flex min-h-full w-full flex-col py-4 sm:py-6 ${
            desktop ? 'lg:max-w-[800px]' : 'max-w-[800px]'
          }`}
        >
          <div className="relative min-h-full flex-1">
            <div
              className={
                runtimeMessagesEmpty
                  ? 'pointer-events-auto absolute inset-0 opacity-100 transition-opacity duration-150'
                  : 'pointer-events-none absolute inset-0 opacity-0 transition-opacity duration-150'
              }
            >
              <ChatLanding
                username={currentUsername}
                providerInitial={selectedProviderInitial}
                onSelectPrompt={onChangeInput}
              />
            </div>
            <div className="relative z-10">
              <ThreadConversation
                state={runtimeV2State}
                optimisticUserPreviews={optimisticUserPreviews}
              />
            </div>
          </div>
        </div>
      </div>

      <div className="relative z-20 shrink-0 overscroll-none border-t border-gray-100 bg-white/95 px-3 pb-[calc(env(safe-area-inset-bottom)+12px)] pt-3 backdrop-blur-sm dark:border-gray-800 dark:bg-[#121212]/95 sm:px-4 sm:pt-4">
        <div className={`mx-auto ${desktop ? 'lg:max-w-[800px]' : 'max-w-[800px]'}`}>
          <ChatComposer
            value={input}
            pending={pending}
            disabled={!selectedTextModelId || catalogLoading}
            imageTools={imageMenuItems}
            selectedImageToolKey={selectedImageToolKey}
            selectedImageToolLabel={selectedImageToolLabel}
            selectedImageToolAvailable={selectedImageToolAvailable}
            models={textMenuItems}
            selectedModelKey={selectedTextModelId}
            selectedModelLabel={selectedModelDisplayName ?? '选择模型'}
            selectedModelProvider={selectedModelProvider ?? 'M'}
            modelLoading={catalogLoading}
            imageToolLoading={catalogLoading}
            attachments={attachments}
            placeholder={
              !selectedTextModelId
                ? '先选择一个对话模型'
                : selectedTextModelAvailable
                  ? '提问或继续追问，按 Enter 发送，Shift + Enter 换行'
                  : '提问或继续追问。若要对话，请先在右侧边栏配置 API Key'
            }
            canUploadImages={selectedModelSupportsImageInputs}
            onChange={onChangeInput}
            onClearImageTool={onClearImageTool}
            onRemoveAttachment={onRemoveAttachment}
            onSelectImageTool={onSelectImageTool}
            onSelectModel={onSelectModel}
            onInterrupt={onInterrupt}
            onSubmit={onSubmit}
            onUploadImages={onUploadImages}
          />
        </div>
      </div>
    </>
  )
}

interface RenameSessionDialogProps {
  isOpen: boolean
  pending: boolean
  value: string
  onChange: (value: string) => void
  onClose: () => void
  onSubmit: () => void
}

export function RenameSessionDialog({
  isOpen,
  pending,
  value,
  onChange,
  onClose,
  onSubmit,
}: RenameSessionDialogProps) {
  if (!isOpen) {
    return null
  }

  return (
    <>
      <div
        className="fixed inset-0 z-40 bg-black/25"
        onClick={() => !pending && onClose()}
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
            value={value}
            onChange={(event) => onChange(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === 'Enter') {
                event.preventDefault()
                onSubmit()
              }
            }}
            disabled={pending}
            autoFocus
            maxLength={40}
            placeholder="输入会话名称"
            className="h-11 w-full rounded-xl border border-gray-200 bg-white px-4 text-[14px] text-gray-900 outline-none transition focus:border-gray-300 dark:border-gray-700 dark:bg-[#1a1a1a] dark:text-gray-100 dark:focus:border-gray-500"
          />

          <div className="flex items-center justify-end gap-2">
            <button
              type="button"
              onClick={onClose}
              disabled={pending}
              className="inline-flex h-10 items-center rounded-lg border border-gray-200 bg-white px-4 text-[14px] text-gray-700 transition hover:bg-gray-50 disabled:cursor-not-allowed disabled:opacity-60 dark:border-gray-700 dark:bg-[#1a1a1a] dark:text-gray-200 dark:hover:bg-gray-800"
            >
              取消
            </button>
            <button
              type="button"
              onClick={onSubmit}
              disabled={pending || !value.trim()}
              className="inline-flex h-10 items-center rounded-lg bg-black px-4 text-[14px] text-white transition hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60 dark:bg-white dark:text-black"
            >
              {pending ? '保存中' : '确定'}
            </button>
          </div>
        </div>
      </div>
    </>
  )
}
