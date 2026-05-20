import type { ChatRuntimeState } from '@openchat/chat-core'
import type { ChatMessage } from '@openchat/protocol'
import { AssistantFrame } from './AssistantFrame'
import { AssistantActionsBar } from './AssistantActionsBar'
import { ContentParts } from './ContentParts'
import { ContentLoading } from './parts/ContentLoading'
import { Image } from './parts/Image'
import { Reasoning } from './parts/Reasoning'
import { Text } from './parts/Text'

const getReasoningText = (message: ChatMessage): string =>
  message.content
    .filter(
      (part): part is Extract<(typeof message.content)[number], { type: 'text' }> =>
        part.type === 'text',
    )
    .map((part) => part.text)
    .join('\n\n')

const formatRelativeTime = (dateString?: string) => {
  if (!dateString) {
    return null
  }

  const timestamp = Date.parse(dateString)
  if (Number.isNaN(timestamp)) {
    return null
  }

  const diffMs = Math.max(0, Date.now() - timestamp)
  const minuteMs = 60 * 1000
  const hourMs = 60 * minuteMs
  const dayMs = 24 * hourMs
  const weekMs = 7 * dayMs
  const monthMs = 30 * dayMs
  const yearMs = 365 * dayMs

  if (diffMs < minuteMs) {
    return '刚刚'
  }

  if (diffMs < hourMs) {
    return `${Math.floor(diffMs / minuteMs)} 分钟前`
  }

  if (diffMs < dayMs) {
    return `${Math.floor(diffMs / hourMs)} 小时前`
  }

  if (diffMs < weekMs) {
    return `${Math.floor(diffMs / dayMs)} 天前`
  }

  if (diffMs < monthMs) {
    return `${Math.floor(diffMs / weekMs)} 周前`
  }

  if (diffMs < yearMs) {
    return `${Math.floor(diffMs / monthMs)} 个月前`
  }

  return `${Math.floor(diffMs / yearMs)} 年前`
}

interface ChatMessageItemProps {
  message: ChatMessage
  reasoningMessage?: ChatMessage
  isReasoningActive?: boolean
  toolState: ChatRuntimeState['toolCalls']
}

const getDurationMs = (message?: ChatMessage) => {
  if (!message?.createdAt || !message.updatedAt || message.status === 'in_progress') {
    return null
  }

  const startedAt = Date.parse(message.createdAt)
  const completedAt = Date.parse(message.updatedAt)

  if (Number.isNaN(startedAt) || Number.isNaN(completedAt) || completedAt < startedAt) {
    return null
  }

  return completedAt - startedAt
}

export function ChatMessageItem({
  message,
  reasoningMessage,
  isReasoningActive = false,
  toolState,
}: ChatMessageItemProps) {
  if (message.role === 'user') {
    const imageParts = message.content.filter(
      (part): part is Extract<(typeof message.content)[number], { type: 'image' }> =>
        part.type === 'image',
    )
    const textParts = message.content.filter(
      (part): part is Extract<(typeof message.content)[number], { type: 'text' }> =>
        part.type === 'text' && part.text.trim().length > 0,
    )

    return (
      <article className="flex w-full px-2 py-2 sm:px-4">
        <div className="ml-auto flex max-w-[82%] flex-col items-end gap-2 sm:max-w-[75%]">
          {imageParts.length > 0 ? (
            <div className="lc-user-message-images">
              {imageParts.map((part, index) => (
                <Image
                  key={`user-image:${message.turnId}:${part.url}:${index}`}
                  url={part.url}
                  alt={part.alt}
                />
              ))}
            </div>
          ) : null}

          <div className="lc-user-message-bubble rounded-2xl bg-gray-100 px-4 py-2.5 text-[14px] leading-relaxed text-gray-900 shadow-sm dark:bg-gray-800 dark:text-gray-100">
            <div className="lc-message-content">
              {textParts.map((part, index) => (
                <Text
                  key={`user-text:${message.turnId}:${index}`}
                  text={part.text}
                  isCreatedByUser
                  showCursor={message.status === 'in_progress'}
                />
              ))}
            </div>
          </div>
        </div>
      </article>
    )
  }

  if (message.role === 'assistant') {
    const relativeTime = formatRelativeTime(message.createdAt ?? message.updatedAt)
    const reasoningText = reasoningMessage ? getReasoningText(reasoningMessage).trim() : ''
    const contentCount = message.content?.length ?? 0
    const toolCallCount = message.toolCalls?.length ?? 0
    const shouldShowPending =
      message.status === 'in_progress' &&
      contentCount === 0 &&
      toolCallCount === 0 &&
      reasoningText.length === 0

    return (
      <AssistantFrame
        footer={
          <div className="mt-1">
            <AssistantActionsBar message={message} />
          </div>
        }
        meta={relativeTime ? <span className="text-[11px] text-gray-400">{relativeTime}</span> : null}
      >
        {reasoningMessage ? (
          <Reasoning
            reasoning={getReasoningText(reasoningMessage)}
            isActive={isReasoningActive}
            durationMs={getDurationMs(reasoningMessage)}
          />
        ) : null}

        <div className="text-[14px] text-gray-800 dark:text-gray-200 leading-relaxed break-words">
          <ContentParts
            message={message}
            toolState={toolState}
            emptyFallback={
              shouldShowPending ? (
                <ContentLoading label="准备响应中" startedAt={message.createdAt} />
              ) : null
            }
          />
        </div>
      </AssistantFrame>
    )
  }

  if (message.role === 'reasoning') {
    return (
      <article className="flex w-full px-2 py-2 sm:px-4">
        <div className="mr-2.5 w-8 flex-shrink-0 sm:mr-3" />
        <div className="flex min-w-0 flex-1 flex-col">
          <Reasoning
            reasoning={getReasoningText(message)}
            isActive={isReasoningActive}
            durationMs={getDurationMs(message)}
          />
        </div>
      </article>
    )
  }

  return (
    <article className="flex w-full px-2 py-4 sm:px-4">
      <ContentParts message={message} toolState={toolState} />
    </article>
  )
}
