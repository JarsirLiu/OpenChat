import type { ToolCallViewModel } from '@openchat/chat-core'
import type { ToolCallSummary } from '@openchat/protocol'
import { useMemo, useState } from 'react'
import { ImageGen } from './tool-output/ImageGen'
import { ToolIcon, getToolIconType } from './tool-output/ToolIcon'
import { ProgressText } from './tool-output/ProgressText'
import { ToolCallInfo } from './tool-output/ToolCallInfo'
import { getToolResultPart, isImageToolCall } from './toolCallMeta'

interface ToolCallItemProps {
  toolCall: ToolCallSummary
  liveState?: ToolCallViewModel
}

export function ToolCallItem({ toolCall, liveState }: ToolCallItemProps) {
  const [isExpanded, setIsExpanded] = useState(liveState?.status === 'in_progress')
  const iconType = useMemo(() => getToolIconType(toolCall.name), [toolCall.name])
  const isImageTool = isImageToolCall(toolCall, liveState)
  const status = liveState?.status ?? 'queued'
  const label = toolCall.displayName ?? toolCall.name
  const toolResult = getToolResultPart(toolCall.content)

  const toggle = () => setIsExpanded((value) => !value)

  if (isImageTool) {
    return (
      <ImageGen
        toolName={label}
        argumentsText={liveState?.argumentsText ?? ''}
        resultText={liveState?.resultText ?? ''}
        media={liveState?.media ?? toolResult?.media ?? []}
        status={status}
        isExpanded={isExpanded || status === 'in_progress'}
        onToggle={toggle}
      />
    )
  }

  return (
    <div className="lc-tool-card">
      <div className="lc-tool-summary">
        <div className="lc-tool-summary-main">
          <ToolIcon type={iconType} isAnimating={liveState?.status === 'in_progress'} />
          <ProgressText
            label={label}
            status={status}
            isExpanded={isExpanded}
            onClick={toggle}
          />
        </div>
      </div>
      {isExpanded ? (
        <div className="lc-tool-body">
          <ToolCallInfo
            input={liveState?.argumentsText}
            output={liveState?.resultText}
            status={status}
          />
        </div>
      ) : null}
    </div>
  )
}
