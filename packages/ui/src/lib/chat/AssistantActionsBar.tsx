import type { ChatMessage } from '@openchat/protocol'
import { Check, Copy, RefreshCw, ThumbsDown, ThumbsUp } from 'lucide-react'
import { useMemo, useState } from 'react'

interface AssistantActionsBarProps {
  message: ChatMessage
}

const toPlainText = (message: ChatMessage): string => {
  const segments: string[] = []

  const body = message.content
    .filter((part): part is Extract<(typeof message.content)[number], { type: 'text' }> => part.type === 'text')
    .map((part) => part.text)
    .join('\n\n')
    .trim()

  if (body) {
    segments.push(body)
  }

  return segments.join('\n\n')
}

function ActionBtn({ onClick, disabled, label, children }: {
  onClick?: () => void
  disabled?: boolean
  label: string
  children: React.ReactNode
}) {
  return (
    <button
      type="button"
      className="flex h-6 w-6 items-center justify-center rounded-md text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
      onClick={onClick}
      disabled={disabled}
      aria-label={label}
    >
      {children}
    </button>
  )
}

export function AssistantActionsBar({ message }: AssistantActionsBarProps) {
  const [copied, setCopied] = useState(false)
  const hasText = useMemo(() => Boolean(toPlainText(message)), [message])

  const copyMessage = async () => {
    const text = toPlainText(message)
    if (!text) return
    await navigator.clipboard.writeText(text)
    setCopied(true)
    window.setTimeout(() => setCopied(false), 1600)
  }

  return (
    <div className="flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity">
      <ActionBtn onClick={() => void copyMessage()} disabled={!hasText} label="Copy message">
        {copied
          ? <Check className="h-3.5 w-3.5" strokeWidth={2} />
          : <Copy className="h-3.5 w-3.5" strokeWidth={1.8} />
        }
      </ActionBtn>
      <ActionBtn label="Regenerate">
        <RefreshCw className="h-3.5 w-3.5" strokeWidth={1.8} />
      </ActionBtn>
      <ActionBtn label="Good response">
        <ThumbsUp className="h-3.5 w-3.5" strokeWidth={1.8} />
      </ActionBtn>
      <ActionBtn label="Bad response">
        <ThumbsDown className="h-3.5 w-3.5" strokeWidth={1.8} />
      </ActionBtn>
    </div>
  )
}


