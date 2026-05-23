import type { ReactNode } from 'react'
import type { ChatRuntimeState } from '@openchat/chat-core'
import type { ChatMessage } from '@openchat/protocol'
import { buildMessageParts } from './messageParts'
import { ToolCallGroup } from './ToolCallGroup'
import { Document } from './parts/Document'
import { Image } from './parts/Image'
import { Text } from './parts/Text'

interface ContentPartsProps {
  message: ChatMessage
  toolState: ChatRuntimeState['toolCalls']
  emptyFallback?: ReactNode
}

export function ContentParts({ message, toolState, emptyFallback }: ContentPartsProps) {
  const parts = buildMessageParts(message, toolState)

  if (parts.length === 0) {
    return emptyFallback ?? null
  }

  return (
    <div className="lc-message-content">
      {parts.map((part, index) => {
        if (part.type === 'text') {
          return (
            <Text
              key={`text:${message.turnId}:${index}`}
              text={part.text}
              isCreatedByUser={message.role === 'user'}
              showCursor={message.status === 'in_progress'}
            />
          )
        }

        if (part.type === 'image') {
          return <Image key={`image:${part.url}:${index}`} url={part.url} alt={part.alt} />
        }

        if (part.type === 'document') {
          return (
            <Document
              key={`document:${part.url}:${index}`}
              url={part.url}
              name={part.name}
              mimeType={part.mimeType}
              sizeBytes={part.sizeBytes}
            />
          )
        }

        return (
          <ToolCallGroup
            key={`tool-group:${part.toolCalls.map((toolCall) => toolCall.id).join(',')}`}
            toolCalls={part.toolCalls}
            toolState={part.toolState}
          />
        )
      })}
    </div>
  )
}
