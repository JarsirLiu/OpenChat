import { useEffect, useRef, useState, type ReactNode } from 'react'
import type { ChatRuntimeState } from '@openchat/chat-core'
import type { ChatMessage, TurnTerminalReasonCode } from '@openchat/protocol'
import { Bot } from 'lucide-react'
import { ChatMessageItem } from './chat/ChatMessageItem'
import { ImageGenerationCard } from './chat/ImageGenerationCard'
import { ContentLoading } from './chat/parts/ContentLoading'
import { isImageToolCall } from './chat/toolCallMeta'

interface ConversationProps {
  state: ChatRuntimeState
  requestPending?: boolean
}

const isTurnReasoningActive = (state: ChatRuntimeState, turnId: string) =>
  state.activeTurnId === turnId && state.pending === 'reasoning'

const getMessageKey = (message: ChatMessage) => {
  if (message.role === 'assistant') {
    return `assistant:${message.turnId}`
  }

  if (message.role === 'reasoning') {
    return `reasoning:${message.id}`
  }

  return `${message.role}:${message.id}`
}

const splitAssistantToolCalls = (
  message: ChatMessage,
  toolState: ChatRuntimeState['toolCalls'],
) => {
  const imageToolCalls = (message.toolCalls ?? []).filter((toolCall) =>
    isImageToolCall(toolCall, toolState[toolCall.id]),
  )
  const contentToolCalls = (message.toolCalls ?? []).filter((toolCall) =>
    !isImageToolCall(toolCall, toolState[toolCall.id]),
  )

  return {
    imageToolCalls,
    contentToolCalls,
  }
}

const hasRenderableAssistantContent = (message: ChatMessage, reasoningMessage?: ChatMessage) => {
  const hasText = message.content.some(
    (part) => part.type === 'text' && part.text.trim().length > 0,
  )
  const hasVisibleToolCalls = (message.toolCalls?.length ?? 0) > 0
  const hasReasoning = Boolean(reasoningMessage)

  return hasText || hasVisibleToolCalls || hasReasoning || message.status === 'in_progress'
}

const isTimeoutErrorCode = (code: TurnTerminalReasonCode | null | undefined) =>
  code === 'model_connect_timeout' || code === 'model_stream_idle_timeout'

const formatConversationErrorMessage = (
  code: TurnTerminalReasonCode | null | undefined,
  message: string | null | undefined,
) => {
  if (code === 'provider_authentication_failed') {
    return '您的 API Key 无效，请检查是否使用了正确的配置。'
  }

  return message ?? null
}

const AssistantAvatar = () => (
  <div className="flex h-8 w-8 items-center justify-center overflow-hidden rounded-full border border-gray-100 bg-white shadow-sm dark:border-gray-800 dark:bg-gray-900">
    <img
      src="/openchat-logo-3d.webp"
      alt="OpenChat"
      className="h-6 w-6 object-contain"
      onError={(event) => {
        event.currentTarget.style.display = 'none'
        event.currentTarget.nextElementSibling?.classList.remove('hidden')
      }}
    />
    <Bot className="hidden h-4 w-4 text-gray-700 dark:text-gray-300" />
  </div>
)

export function Conversation({ state, requestPending = false }: ConversationProps) {
  const lastMessagesLength = useRef(state.messages.length)
  const [hasResponse, setHasResponse] = useState(false)
  const isWaitingForResponse = requestPending

  useEffect(() => {
    if (!isWaitingForResponse) {
      setHasResponse(false)
      lastMessagesLength.current = state.messages.length
    } else if (state.messages.length > lastMessagesLength.current) {
      setHasResponse(true)
    }
  }, [isWaitingForResponse, state.messages.length])

  const items: ReactNode[] = []

  for (let index = 0; index < state.messages.length; index += 1) {
    const message = state.messages[index]

    if (!message) {
      continue
    }

    if (message.role === 'reasoning') {
      const next = state.messages[index + 1]

      if (next?.role === 'assistant' && next.turnId === message.turnId) {
        const { imageToolCalls, contentToolCalls } = splitAssistantToolCalls(next, state.toolCalls)
        const assistantMessage =
          contentToolCalls.length === (next.toolCalls?.length ?? 0)
            ? next
            : { ...next, toolCalls: contentToolCalls }

        if (hasRenderableAssistantContent(assistantMessage, message)) {
          items.push(
            <ChatMessageItem
              key={`turn:${message.turnId}`}
              message={assistantMessage}
              reasoningMessage={message}
              isReasoningActive={isTurnReasoningActive(state, message.turnId)}
              toolState={state.toolCalls}
            />,
          )
        } else {
          items.push(
            <ChatMessageItem
              key={getMessageKey(message)}
              message={message}
              isReasoningActive={isTurnReasoningActive(state, message.turnId)}
              toolState={state.toolCalls}
            />,
          )
        }

        imageToolCalls.forEach((toolCall) => {
          items.push(
            <ImageGenerationCard
              key={`image-tool:${toolCall.id}`}
              toolCall={toolCall}
              liveState={state.toolCalls[toolCall.id]}
              startedAt={next.createdAt}
              completedAt={next.updatedAt}
            />,
          )
        })

        index += 1
        continue
      }
    }

    const previous = state.messages[index - 1]
    if (
      message.role === 'assistant' &&
      previous?.role === 'reasoning' &&
      previous.turnId === message.turnId
    ) {
      continue
    }

    if (message.role === 'assistant') {
      const { imageToolCalls, contentToolCalls } = splitAssistantToolCalls(message, state.toolCalls)
      const assistantMessage =
        contentToolCalls.length === (message.toolCalls?.length ?? 0)
          ? message
          : { ...message, toolCalls: contentToolCalls }

      if (hasRenderableAssistantContent(assistantMessage)) {
        items.push(
          <ChatMessageItem
            key={getMessageKey(message)}
            message={assistantMessage}
            isReasoningActive={isTurnReasoningActive(state, message.turnId)}
            toolState={state.toolCalls}
          />,
        )
      }

      imageToolCalls.forEach((toolCall) => {
        items.push(
          <ImageGenerationCard
            key={`image-tool:${toolCall.id}`}
            toolCall={toolCall}
            liveState={state.toolCalls[toolCall.id]}
            startedAt={message.createdAt}
            completedAt={message.updatedAt}
          />,
        )
      })

      continue
    }

    items.push(
      <ChatMessageItem
        key={getMessageKey(message)}
        message={message}
        isReasoningActive={isTurnReasoningActive(state, message.turnId)}
        toolState={state.toolCalls}
      />,
    )
  }

  const errorMessage = formatConversationErrorMessage(state.error?.code, state.error?.message)
  const showTimeoutStyle = isTimeoutErrorCode(state.error?.code)

  return (
    <div className="flex w-full flex-col space-y-5 px-2 py-3 lg:space-y-6 lg:px-0 lg:py-4">
      {items}
      {errorMessage ? (
        <article className="group flex w-full px-2 lg:px-0">
          <div className="flex-shrink-0 mr-3">
            <AssistantAvatar />
          </div>
          <div className="flex min-w-0 flex-1 flex-col">
            <div className="mb-1 flex items-center gap-2">
              <span className="text-[13px] font-semibold text-gray-800 dark:text-gray-200">
                OpenChat
              </span>
            </div>
            <div
              className={
                showTimeoutStyle
                  ? 'max-w-[min(720px,100%)] rounded-2xl border border-amber-200 bg-amber-50 px-4 py-3 text-[14px] leading-6 text-amber-900 dark:border-amber-900/60 dark:bg-amber-950/30 dark:text-amber-100'
                  : 'max-w-[min(720px,100%)] rounded-2xl border border-red-200 bg-red-50 px-4 py-3 text-[14px] leading-6 text-red-800 dark:border-red-900/60 dark:bg-red-950/30 dark:text-red-100'
              }
            >
              {errorMessage}
            </div>
          </div>
        </article>
      ) : null}
      {isWaitingForResponse &&
        !hasResponse &&
        state.messages.length > 0 &&
        state.messages[state.messages.length - 1].role === 'user' && (
        <article className="group flex w-full px-2 lg:px-0">
          <div className="flex-shrink-0 mr-3">
            <AssistantAvatar />
          </div>
          <div className="flex flex-col flex-1 min-w-0">
            <div className="flex items-center gap-2 mb-1">
              <span className="text-[13px] font-semibold text-gray-800 dark:text-gray-200">
                OpenChat
              </span>
            </div>
            <div className="mt-2">
              <ContentLoading label="正在发送请求" startedAt={undefined} />
            </div>
          </div>
        </article>
      )}
    </div>
  )
}
