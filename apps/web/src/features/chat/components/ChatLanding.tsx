import { ChatBrandIcon } from './ChatBrandIcon'
import { Compass, FileText, Lightbulb, ListChecks, Mail, Rocket, SearchCheck, Sparkles } from 'lucide-react'

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
  onSelectPrompt,
}: {
  username?: string | null
  providerInitial: string
  onSelectPrompt: (prompt: string) => void
}) {
  const greeting = getGreeting(username)
  const quickStartItems = [
    {
      icon: FileText,
      title: '整理一段内容',
      prompt: '帮我把下面这段内容整理成结构清晰的要点：\n\n',
    },
    {
      icon: ListChecks,
      title: '生成执行清单',
      prompt: '帮我把这个目标拆成可执行清单，并按优先级排序：\n\n',
    },
    {
      icon: Mail,
      title: '写一封邮件',
      prompt: '帮我写一封语气清晰、礼貌、简洁的邮件，主题是：\n\n',
    },
  ] as const
  const assistantItems = [
    {
      icon: SearchCheck,
      title: '帮我分析',
      prompt: '请从背景、关键问题、风险和下一步建议四个角度分析：\n\n',
    },
    {
      icon: Lightbulb,
      title: '给我方案',
      prompt: '请给我 3 个可选方案，并比较它们的优缺点：\n\n',
    },
    {
      icon: Sparkles,
      title: '优化表达',
      prompt: '请帮我优化下面这段表达，让它更清晰、更自然：\n\n',
    },
  ] as const

  return (
    <div className="flex min-h-0 flex-1 flex-col items-center justify-center bg-gray-50/30 px-2 py-6 text-center dark:bg-gray-900/30 sm:px-4 sm:py-8">
      <div className="w-full max-w-2xl space-y-5 sm:space-y-6">
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

        <div className="grid grid-cols-1 gap-4 pt-3 text-left sm:grid-cols-2 sm:pt-4">
          <PromptGroup
            icon={Rocket}
            title="快速开始"
            items={quickStartItems}
            onSelectPrompt={onSelectPrompt}
          />
          <PromptGroup
            icon={Compass}
            title="探索助理"
            items={assistantItems}
            onSelectPrompt={onSelectPrompt}
          />
        </div>
      </div>
    </div>
  )
}

function PromptGroup({
  icon: Icon,
  title,
  items,
  onSelectPrompt,
}: {
  icon: typeof Rocket
  title: string
  items: ReadonlyArray<{
    icon: typeof Rocket
    title: string
    prompt: string
  }>
  onSelectPrompt: (prompt: string) => void
}) {
  return (
    <section className="space-y-2.5">
      <div className="flex items-center gap-2 px-1 text-[13px] font-semibold text-gray-500 dark:text-gray-400">
        <Icon className="h-4 w-4" />
        <span>{title}</span>
      </div>
      <div className="space-y-2">
        {items.map((item) => {
          const ItemIcon = item.icon
          return (
            <button
              key={item.title}
              type="button"
              onClick={() => onSelectPrompt(item.prompt)}
              className="flex w-full items-center gap-3 rounded-lg border border-gray-100 bg-white px-3 py-3 text-left text-[14px] font-medium text-gray-700 shadow-sm transition hover:border-gray-200 hover:bg-gray-50 dark:border-gray-700 dark:bg-gray-800 dark:text-gray-200 dark:hover:border-gray-600 dark:hover:bg-gray-700"
            >
              <ItemIcon className="h-4 w-4 flex-shrink-0 text-gray-400" />
              <span className="min-w-0 truncate">{item.title}</span>
            </button>
          )
        })}
      </div>
    </section>
  )
}
