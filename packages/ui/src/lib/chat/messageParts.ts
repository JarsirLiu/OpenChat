import type { ToolCallViewModel } from '@openchat/chat-core'
import type {
  ChatMessage,
  MessageContentPart,
  ToolCallSummary,
} from '@openchat/protocol'
import { isImageToolCall } from './toolCallMeta'

export interface RenderTextPart {
  type: 'text'
  text: string
}

export interface RenderImagePart {
  type: 'image'
  url: string
  alt: string
}

export interface RenderDocumentPart {
  type: 'document'
  url: string
  name: string
  mimeType: string
  sizeBytes: number
}

export interface RenderToolCallGroupPart {
  type: 'tool_call_group'
  toolCalls: ToolCallSummary[]
  toolState: Record<string, ToolCallViewModel>
}

export type RenderMessagePart =
  | RenderTextPart
  | RenderImagePart
  | RenderDocumentPart
  | RenderToolCallGroupPart

const mapContentPart = (
  part: MessageContentPart,
): RenderTextPart | RenderImagePart | RenderDocumentPart | null => {
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

  if (part.type === 'document') {
    return {
      type: 'document',
      url: part.url,
      name: part.name,
      mimeType: part.mime_type,
      sizeBytes: part.size_bytes,
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

  const visibleToolCalls =
    message.toolCalls?.filter((toolCall) => !isImageToolCall(toolCall, toolState[toolCall.id])) ?? []

  if (visibleToolCalls.length) {
    parts.push({
      type: 'tool_call_group',
      toolCalls: visibleToolCalls,
      toolState,
    })
  }

  return parts
}
