import { Claude, Gemini, OpenAI } from '@lobehub/icons'
import { Bot } from 'lucide-react'
import clsx from 'clsx'

type ModelAvatarProps = {
  model: string
  size?: number
}

function renderKnownProviderAvatar(model: string, size: number) {
  const normalized = model.trim().toLowerCase()

  if (normalized === 'openai') {
    return <OpenAI.Avatar size={size} type="gpt5" />
  }

  if (normalized === 'claude') {
    return <Claude.Avatar size={size} />
  }

  if (normalized === 'gemini') {
    return <Gemini.Avatar size={size} />
  }

  return null
}

function resolveFallbackLabel(model: string) {
  const normalized = model.trim()
  if (!normalized) {
    return null
  }

  return normalized.slice(0, 2).toUpperCase()
}

export function ModelAvatar({ model, size = 16 }: ModelAvatarProps) {
  const knownAvatar = renderKnownProviderAvatar(model, size)
  if (knownAvatar) {
    return knownAvatar
  }

  const fallbackLabel = resolveFallbackLabel(model)
  const dimension = `${size}px`

  return (
    <span
      className={clsx(
        'inline-flex shrink-0 items-center justify-center rounded-full border border-black/5 bg-gray-100 font-semibold text-gray-600 shadow-sm dark:bg-gray-800 dark:text-gray-300',
      )}
      style={{
        width: dimension,
        height: dimension,
        fontSize: `${Math.max(9, Math.floor(size * 0.44))}px`,
        lineHeight: 1,
      }}
      aria-hidden="true"
    >
      {fallbackLabel ? (
        fallbackLabel
      ) : (
        <Bot size={Math.max(10, Math.floor(size * 0.65))} strokeWidth={2} />
      )}
    </span>
  )
}
