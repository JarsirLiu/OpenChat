import { useState } from 'react'
import { Menu, MoreHorizontal, Pencil, Trash, PanelRightOpen, PanelRightClose } from 'lucide-react'

interface ChatHeaderProps {
  title: string
  onRename?: () => void
  onDelete?: () => void
  onOpenSettings?: () => void
  onOpenSidebar?: () => void
  settingsOpen?: boolean
}

export function ChatHeader({
  title,
  onRename,
  onDelete,
  onOpenSettings,
  onOpenSidebar,
  settingsOpen = false,
}: ChatHeaderProps) {
  const [isMenuOpen, setIsMenuOpen] = useState(false)

  return (
    <div className="sticky top-0 z-10 border-b border-gray-100 bg-white/80 px-3 py-3 backdrop-blur-sm dark:border-gray-800 dark:bg-gray-900/80 sm:p-4">
      <div className="flex items-center justify-between">
        <div className="flex min-w-0 items-center gap-2">
          <button
            type="button"
            className="rounded-md p-1 text-gray-500 transition-colors hover:bg-gray-100 hover:text-gray-700 dark:text-gray-400 dark:hover:bg-gray-800 dark:hover:text-white lg:hidden"
            aria-label="Open sidebar"
            onClick={onOpenSidebar}
          >
            <Menu className="h-4 w-4" />
          </button>
          <h1 className="truncate text-[18px] font-semibold text-gray-900 dark:text-white sm:text-base">
            {title}
          </h1>
          <div className="relative">
            <button
              type="button"
              className="rounded-md p-1 text-gray-400 transition-colors hover:bg-gray-100 hover:text-gray-600 dark:hover:bg-gray-800"
              aria-label="More options"
              onClick={() => setIsMenuOpen(!isMenuOpen)}
            >
              <MoreHorizontal className="h-4 w-4" />
            </button>

            {isMenuOpen && (
              <>
                <div 
                  className="fixed inset-0 z-10" 
                  onClick={() => setIsMenuOpen(false)}
                />
                <div className="absolute left-0 mt-1 w-40 rounded-lg border border-gray-100 bg-white p-1 shadow-lg dark:border-gray-800 dark:bg-gray-900 z-20">
                  <button
                    type="button"
                    className="flex w-full items-center gap-2 rounded-md px-3 py-2 text-sm text-gray-700 transition-colors hover:bg-gray-50 dark:text-gray-300 dark:hover:bg-gray-800"
                    onClick={() => {
                      setIsMenuOpen(false)
                      onRename?.()
                    }}
                  >
                    <Pencil className="h-4 w-4" />
                    <span>重命名</span>
                  </button>
                  <div className="my-1 border-t border-gray-100 dark:border-gray-800" />
                  <button
                    type="button"
                    className="flex w-full items-center gap-2 rounded-md px-3 py-2 text-sm text-red-600 transition-colors hover:bg-red-50 dark:text-red-400 dark:hover:bg-red-900/20"
                    onClick={() => {
                      setIsMenuOpen(false)
                      onDelete?.()
                    }}
                  >
                    <Trash className="h-4 w-4" />
                    <span>删除</span>
                  </button>
                </div>
              </>
            )}
          </div>
        </div>

        <div className="flex items-center gap-0.5 sm:gap-1">
          <button
            type="button"
            className={`rounded-lg p-2 transition-colors ${
              settingsOpen
                ? 'bg-gray-100 text-gray-900 dark:bg-gray-800 dark:text-white'
                : 'text-gray-500 hover:bg-gray-100 hover:text-gray-700 dark:text-gray-400 dark:hover:bg-gray-800 dark:hover:text-white'
            }`}
            aria-label="Conversation settings"
            onClick={onOpenSettings}
          >
            {settingsOpen ? (
              <PanelRightClose className="h-4.5 w-4.5" strokeWidth={1.8} />
            ) : (
              <PanelRightOpen className="h-4.5 w-4.5" strokeWidth={1.8} />
            )}
          </button>
        </div>
      </div>
    </div>
  )
}
