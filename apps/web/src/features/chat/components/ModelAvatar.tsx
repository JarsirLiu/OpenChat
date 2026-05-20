import { Bot, Sparkles } from 'lucide-react'
import clsx from 'clsx'

type ModelAvatarProps = {
  model: string
  size?: number
}

type AvatarTone = {
  className: string
  label: string
}

const MODEL_TONES: Record<string, AvatarTone> = {
  claude: {
    className: 'bg-[#f3dfcf] text-[#7a3b12] dark:bg-[#5b341a] dark:text-[#ffd6b5]',
    label: 'Cl',
  },
  gemini: {
    className: 'bg-[#dfe8ff] text-[#2451d1] dark:bg-[#1f316f] dark:text-[#c9d7ff]',
    label: 'Ge',
  },
  openai: {
    className: 'bg-[#dff5ea] text-[#117a4b] dark:bg-[#163c2f] dark:text-[#c7f6df]',
    label: 'AI',
  },
}

function resolveTone(model: string) {
  const normalized = model.trim().toLowerCase()
  return MODEL_TONES[normalized]
}

function resolveLabel(model: string) {
  const normalized = model.trim()
  if (!normalized) {
    return null
  }

  return normalized.slice(0, 2).toUpperCase()
}

export function ModelAvatar({ model, size = 16 }: ModelAvatarProps) {
  const tone = resolveTone(model)
  const fallbackLabel = resolveLabel(model)
  const dimension = `${size}px`

  return (
    <span
      className={clsx(
        'inline-flex shrink-0 items-center justify-center rounded-full border border-black/5 font-semibold shadow-sm',
        tone
          ? tone.className
          : 'bg-gray-100 text-gray-600 dark:bg-gray-800 dark:text-gray-300',
      )}
      style={{
        width: dimension,
        height: dimension,
        fontSize: `${Math.max(9, Math.floor(size * 0.44))}px`,
        lineHeight: 1,
      }}
      aria-hidden="true"
    >
      {tone ? (
        tone.label
      ) : fallbackLabel ? (
        fallbackLabel
      ) : size >= 20 ? (
        <Sparkles size={Math.max(12, Math.floor(size * 0.65))} strokeWidth={2} />
      ) : (
        <Bot size={Math.max(10, Math.floor(size * 0.65))} strokeWidth={2} />
      )}
    </span>
  )
}
