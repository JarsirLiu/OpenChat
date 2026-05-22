import { Bot } from 'lucide-react'

export function ChatBrandIcon({
  providerInitial,
  className = '',
  size = 41,
  showProviderInitial = true,
}: {
  providerInitial: string
  className?: string
  size?: number
  showProviderInitial?: boolean
}) {
  return (
    <div
      className={`shadow-stroke relative flex items-center justify-center overflow-hidden rounded-full bg-white text-black ${className}`.trim()}
      style={{ width: size, height: size }}
    >
      {showProviderInitial && providerInitial ? (
        <span className="text-lg font-semibold">{providerInitial}</span>
      ) : (
        <>
          <img
            src="/openchat-logo-3d.webp"
            alt=""
            className="h-[72%] w-[72%] object-contain"
            onError={(event) => {
              event.currentTarget.style.display = 'none'
              event.currentTarget.nextElementSibling?.classList.remove('hidden')
            }}
          />
          <Bot className="hidden h-2/3 w-2/3" strokeWidth={1.8} />
        </>
      )}
    </div>
  )
}
