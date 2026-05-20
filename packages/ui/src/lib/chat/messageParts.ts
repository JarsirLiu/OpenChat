import type { ToolCallViewModel } from '@openchat/chat-core'
import type {
  ChatMessage,
  MessageContentPart,
  ToolCallSummary,
} from '@openchat/protocol'

export interface RenderTextPart {
  type: 'text'
  text: string
}

export interface RenderImagePart {
  type: 'image'
  url: string
  alt: string
}

export interface RenderToolCallGroupPart {
  type: 'tool_call_group'
  toolCalls: ToolCallSummary[]
  toolState: Record<string, ToolCallViewModel>
}

export type RenderMessagePart =
  | RenderTextPart
  | RenderImagePart
  | RenderToolCallGroupPart

const mapContentPart = (part: MessageContentPart): RenderTextPart | RenderImagePart | null => {
  if (part.type === 'text') {
    return {
      type: 'text',
      text: part.text,
    }
  }

  if (part.type === 'image') {
    return {
      type: 'image',
      url: part.url,
      alt: part.alt,
    }
  }

  return null
}

export const buildMessageParts = (
  message: ChatMessage,
  toolState: Record<string, ToolCallViewModel>,
): RenderMessagePart[] => {
  const parts: RenderMessagePart[] = []

  for (const part of message.content) {
    if (message.role === 'assistant' && part.type === 'image') {
      continue
    }
    const mapped = mapContentPart(part)
    if (mapped) {
      parts.push(mapped)
    }
  }

  if (message.toolCalls?.length) {
    parts.push({
      type: 'tool_call_group',
      toolCalls: message.toolCalls,
      toolState,
    })
  }

  return parts
}
