import { ChatBrandIcon } from './ChatBrandIcon'

function getGreeting(username?: string | null) {
  const hour = new Date().getHours()
  const base = hour < 5
    ? '夜深了，该休息了'
    : hour < 11
      ? '早上好'
      : hour < 14
        ? '中午好'
        : hour < 19
          ? '下午好'
          : hour < 23
            ? '晚上好'
            : '夜深了，该休息了'
  return `${base}, ${username || 'L'}`
}

export function ChatLanding({
  username,
  providerInitial,
}: {
  username?: string | null
  providerInitial: string
}) {
  const greeting = getGreeting(username)

  return (
    <div className="flex min-h-0 flex-1 flex-col items-center justify-center bg-gray-50/30 px-2 py-6 text-center dark:bg-gray-900/30 sm:px-4 sm:py-8">
      <div className="max-w-md space-y-5 sm:space-y-6">
        <div className="flex justify-center">
          <div className="rounded-[28px] border border-gray-100 bg-white p-3 shadow-sm dark:border-gray-700 dark:bg-gray-800 sm:p-4">
            <ChatBrandIcon providerInitial={providerInitial} size={44} />
          </div>
        </div>

        <div className="space-y-2">
          <h1 className="text-[24px] font-bold leading-tight tracking-[-0.03em] text-gray-900 dark:text-white sm:text-3xl">
            {greeting}
          </h1>
          <p className="mx-auto max-w-sm text-[15px] leading-7 text-gray-500 dark:text-gray-400 sm:text-sm sm:leading-6">
            无论是提问、追问，还是整理想法，这里都能陪你一步步聊清楚。
          </p>
        </div>

        <div className="grid grid-cols-1 gap-3 pt-3 text-[15px] text-gray-600 sm:grid-cols-2 sm:pt-4 sm:text-xs dark:text-gray-400">
          <div className="cursor-pointer rounded-2xl border border-gray-100 bg-white px-4 py-4 transition-colors hover:border-gray-200 dark:border-gray-700 dark:bg-gray-800 dark:hover:border-gray-600">
            🚀 快速开始
          </div>
          <div className="cursor-pointer rounded-2xl border border-gray-100 bg-white px-4 py-4 transition-colors hover:border-gray-200 dark:border-gray-700 dark:bg-gray-800 dark:hover:border-gray-600">
            🧠 探索助理
          </div>
        </div>
      </div>
    </div>
  )
}
