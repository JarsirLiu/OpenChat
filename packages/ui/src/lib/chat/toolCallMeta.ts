import type { ToolCallViewModel } from '@openchat/chat-core'
import type { MessageContentPart, MessageContentToolResult, ToolCallSummary, ToolMedia } from '@openchat/protocol'

export const getToolResultPart = (content: MessageContentPart[] = []) =>
  content.find(
    (part): part is MessageContentToolResult => part.type === 'tool_result',
  )

export const getToolMedia = (
  toolCall: ToolCallSummary,
  liveState?: ToolCallViewModel,
): ToolMedia[] => liveState?.media ?? getToolResultPart(toolCall.content)?.media ?? []

export const isImageToolCall = (
  toolCall: ToolCallSummary,
  liveState?: ToolCallViewModel,
): boolean => {
  const toolMedia = getToolMedia(toolCall, liveState)
  if (toolMedia.some((item) => item.kind === 'image' && item.url)) {
    return true
  }

  const normalizedName = toolCall.name.trim().toLowerCase()
  const normalizedDisplayName = (toolCall.displayName ?? '').trim().toLowerCase()

  return (
    normalizedName.includes('image') ||
    normalizedName.includes('img') ||
    normalizedDisplayName.includes('image') ||
    normalizedDisplayName.includes('图片')
  )
}
