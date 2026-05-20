import { useMemo } from 'react'
import { ChatBrandIcon } from './ChatBrandIcon'

function getGreeting(username?: string | null) {
  const hour = new Date().getHours()
  const base = hour < 12 ? '早上好' : hour < 18 ? '下午好' : '晚上好'
  return `${base}, ${username || 'L'}`
}

export function ChatLanding({
  username,
  providerInitial,
}: {
  username?: string | null
  providerInitial: string
}) {
  const greeting = useMemo(() => getGreeting(username), [username])

  return (
    <div className="flex h-full flex-col items-center justify-center bg-gray-50/30 p-6 text-center dark:bg-gray-900/30">
      <div className="max-w-md space-y-6">
        <div className="flex justify-center">
          <div className="rounded-3xl border border-gray-100 bg-white p-4 shadow-sm dark:border-gray-700 dark:bg-gray-800">
            <ChatBrandIcon providerInitial={providerInitial} size={48} />
          </div>
        </div>

        <div className="space-y-2">
          <h1 className="text-3xl font-bold text-gray-900 dark:text-white">{greeting}</h1>
          <p className="mx-auto max-w-sm text-sm text-gray-500 dark:text-gray-400">
            无论是提问、追问，还是整理想法，这里都能陪你一步步聊清楚。
          </p>
        </div>

        <div className="grid grid-cols-1 gap-3 pt-4 text-xs text-gray-600 sm:grid-cols-2 dark:text-gray-400">
          <div className="cursor-pointer rounded-xl border border-gray-100 bg-white p-3 transition-colors hover:border-gray-200 dark:border-gray-700 dark:bg-gray-800 dark:hover:border-gray-600">
            🚀 快速开始
          </div>
          <div className="cursor-pointer rounded-xl border border-gray-100 bg-white p-3 transition-colors hover:border-gray-200 dark:border-gray-700 dark:bg-gray-800 dark:hover:border-gray-600">
            🧠 探索助理
          </div>
        </div>
      </div>
    </div>
  )
}
