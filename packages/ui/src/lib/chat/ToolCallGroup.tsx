import type { ToolCallViewModel } from '@openchat/chat-core'
import type { ToolCallSummary } from '@openchat/protocol'
import { ChevronDown, Users } from 'lucide-react'
import { useMemo, useState } from 'react'
import { ToolCallItem } from './ToolCallItem'

interface ToolCallGroupProps {
  toolCalls: ToolCallSummary[]
  toolState: Record<string, ToolCallViewModel>
}

export function ToolCallGroup({ toolCalls, toolState }: ToolCallGroupProps) {
  const [isExpanded, setIsExpanded] = useState(true)
  const runningCount = useMemo(
    () =>
      toolCalls.filter((toolCall) => {
        const status = toolState[toolCall.id]?.status ?? toolCall.status
        return status === 'in_progress'
      }).length,
    [toolCalls, toolState],
  )
  const groupLabel = useMemo(() => {
    if (runningCount > 0) {
      return runningCount === 1 ? '正在调用 1 个工具' : `正在调用 ${runningCount} 个工具`
    }
    if (toolCalls.length === 1) {
      return toolCalls[0].displayName ?? toolCalls[0].name
    }
    return `已调用 ${toolCalls.length} 个工具`
  }, [runningCount, toolCalls])

  return (
    <div className="lc-tool-group">
      <button
        type="button"
        className="lc-tool-group-trigger"
        onClick={() => setIsExpanded((value) => !value)}
        aria-expanded={isExpanded}
      >
        <div className="lc-tool-group-trigger-main">
          <Users size={14} />
          <span className="lc-tool-group-title">{groupLabel}</span>
        </div>
        <ChevronDown
          className={`lc-tool-group-chevron ${isExpanded ? 'is-open' : ''}`}
          aria-hidden="true"
        />
      </button>
      {isExpanded ? (
        <div className="lc-tool-group-body">
          {toolCalls.map((toolCall) => (
            <ToolCallItem key={toolCall.id} toolCall={toolCall} liveState={toolState[toolCall.id]} />
          ))}
        </div>
      ) : null}
    </div>
  )
}
