import type { ToolCallViewModel } from '@openchat/chat-core'
import type { MessageContentPart, ToolCallSummary } from '@openchat/protocol'
import { useMemo, useState } from 'react'
import { ImageGen } from './tool-output/ImageGen'
import { ToolIcon, getToolIconType } from './tool-output/ToolIcon'
import { ProgressText } from './tool-output/ProgressText'
import { ToolCallInfo } from './tool-output/ToolCallInfo'

const imageToolNames = new Set(['image_gen_oai', 'image_edit_oai', 'gemini_image_gen'])

interface ToolCallItemProps {
  toolCall: ToolCallSummary
  liveState?: ToolCallViewModel
}

const getToolResultPart = (content: MessageContentPart[]) =>
  content.find((part): part is Extract<MessageContentPart, { type: 'tool_result' }> => part.type === 'tool_result')

export function ToolCallItem({ toolCall, liveState }: ToolCallItemProps) {
  const [isExpanded, setIsExpanded] = useState(liveState?.status === 'in_progress')
  const iconType = useMemo(() => getToolIconType(toolCall.name), [toolCall.name])
  const isImageTool = imageToolNames.has(toolCall.name)
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
