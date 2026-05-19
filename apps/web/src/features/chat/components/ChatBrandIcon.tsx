import { Bot } from 'lucide-react'

export function ChatBrandIcon({
  providerInitial,
  className = '',
  size = 41,
}: {
  providerInitial: string
  className?: string
  size?: number
}) {
  return (
    <div
      className={`shadow-stroke relative flex items-center justify-center rounded-full bg-white text-black ${className}`.trim()}
      style={{ width: size, height: size }}
    >
      {providerInitial ? (
        <span className="text-lg font-semibold">{providerInitial}</span>
      ) : (
        <Bot className="h-2/3 w-2/3" strokeWidth={1.8} />
      )}
    </div>
  )
}
