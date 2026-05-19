import type { ItemStatus } from '@openchat/protocol'
import { Atom, ChevronDown, Loader2 } from 'lucide-react'
import { memo, useEffect, useMemo, useState } from 'react'

interface ReasoningProps {
  reasoning: string
  isActive?: boolean
  status?: ItemStatus
  durationMs?: number | null
}

const formatDuration = (durationMs: number) => {
  const seconds = durationMs / 1000
  if (seconds < 10) {
    return `${seconds.toFixed(1)} 秒`
  }

  return `${Math.round(seconds)} 秒`
}

const NonMemoizedReasoning = ({
  reasoning,
  isActive,
  status = 'completed',
  durationMs,
}: ReasoningProps) => {
  const displayText = useMemo(() => reasoning.replace(/\r\n/g, '\n').trim(), [reasoning])
  const isThinking = isActive ?? status === 'in_progress'
  const [isExpanded, setIsExpanded] = useState(isThinking)

  useEffect(() => {
    setIsExpanded(isThinking)
  }, [isThinking])

  if (!displayText && !isThinking) {
    return null
  }

  return (
    <div className="my-1.5 flex flex-col gap-2">
      <button
        type="button"
        className="flex items-center gap-2 text-[13px] text-gray-500 dark:text-gray-400 bg-gray-50 dark:bg-gray-800/50 px-2.5 py-1 rounded-lg w-fit hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
        onClick={() => setIsExpanded((value) => !value)}
        aria-expanded={isExpanded}
      >
        <div className="flex items-center justify-center w-4 h-4 rounded-full bg-purple-100 dark:bg-purple-900/30 text-purple-600 dark:text-purple-400">
          {isThinking ? (
            <Loader2 className="h-2.5 w-2.5 animate-spin" strokeWidth={2.5} />
          ) : (
            <Atom className="h-2.5 w-2.5" strokeWidth={2.5} />
          )}
        </div>
        <span className="font-medium">{isThinking ? '深度思考中' : '已深度思考'}</span>
        {!isThinking && durationMs != null && durationMs >= 0 && (
          <span className="text-[11px] text-gray-400">（用时 {formatDuration(durationMs)}）</span>
        )}
        <ChevronDown
          className={`h-3 w-3 text-gray-400 transition-transform duration-200 ${
            isExpanded ? 'rotate-180' : 'rotate-0'
          }`}
          strokeWidth={2}
        />
      </button>
      
      {isExpanded && (
        <div className="ml-2 border-l-2 border-purple-200 pl-3 dark:border-purple-800">
          <div className="max-h-[320px] overflow-y-auto pr-2 text-[13px] leading-relaxed text-gray-600 dark:text-gray-400 whitespace-pre-wrap">
            {displayText ? displayText : '正在思考…'}
          </div>
        </div>
      )}
    </div>
  )
}

export const Reasoning = memo(
  NonMemoizedReasoning,
  (prevProps, nextProps) =>
    prevProps.reasoning === nextProps.reasoning &&
    prevProps.isActive === nextProps.isActive &&
    prevProps.status === nextProps.status &&
    prevProps.durationMs === nextProps.durationMs,
)
