import { Bot } from 'lucide-react'
import type { ReactNode } from 'react'

interface AssistantFrameProps {
  children: ReactNode
  footer?: ReactNode
  meta?: ReactNode
}

export function AssistantFrame({ children, footer, meta }: AssistantFrameProps) {
  return (
    <article className="group flex w-full px-2 py-3 sm:px-4 sm:py-4">
      <div className="mr-2.5 flex-shrink-0 sm:mr-3">
        <div className="flex h-8 w-8 items-center justify-center rounded-full border border-gray-100 dark:border-gray-800 bg-white dark:bg-gray-900 shadow-sm overflow-hidden">
          <img
            src="https://unpkg.com/@lobehub/assets-logo@1.2.0/assets/logo-3d.webp"
            alt="OpenChat"
            className="h-6 w-6 object-contain"
            onError={(event) => {
              event.currentTarget.style.display = 'none'
              event.currentTarget.nextElementSibling?.classList.remove('hidden')
            }}
          />
          <Bot className="h-4 w-4 text-gray-700 dark:text-gray-300 hidden" />
        </div>
      </div>
      <div className="flex min-w-0 flex-1 flex-col">
        <div className="mb-1 flex items-center gap-2">
          <span className="text-[13px] font-semibold text-gray-800 dark:text-gray-200">OpenChat</span>
          {meta}
        </div>
        {children}
        {footer}
      </div>
    </article>
  )
}
