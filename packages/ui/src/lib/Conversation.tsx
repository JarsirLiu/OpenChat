import { useEffect, useRef, useState, type ReactNode } from 'react'
import type { ChatRuntimeState } from '@openchat/chat-core'
import type { ChatMessage, TurnTerminalReasonCode } from '@openchat/protocol'
import { ChatMessageItem } from './chat/ChatMessageItem'
import { ContentLoading } from './chat/parts/ContentLoading'

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

const isTimeoutErrorCode = (code: TurnTerminalReasonCode | null | undefined) =>
  code === 'model_connect_timeout' || code === 'model_stream_idle_timeout'

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
        items.push(
          <ChatMessageItem
            key={`turn:${message.turnId}`}
            message={next}
            reasoningMessage={message}
            isReasoningActive={isTurnReasoningActive(state, message.turnId)}
            toolState={state.toolCalls}
          />,
        )
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

    items.push(
      <ChatMessageItem
        key={getMessageKey(message)}
        message={message}
        isReasoningActive={isTurnReasoningActive(state, message.turnId)}
        toolState={state.toolCalls}
      />,
    )
  }

  const timeoutMessage = isTimeoutErrorCode(state.error?.code) ? state.error?.message : null

  return (
    <div className="flex flex-col space-y-6 p-4 w-full">
      {items}
      {timeoutMessage ? (
        <article className="group flex w-full">
          <div className="flex-shrink-0 mr-3">
            <div className="flex h-8 w-8 items-center justify-center overflow-hidden rounded-full border border-gray-100 bg-white shadow-sm dark:border-gray-800 dark:bg-gray-900">
              <img
                src="https://unpkg.com/@lobehub/assets-logo@1.2.0/assets/logo-3d.webp"
                alt="OpenChat"
                className="h-6 w-6 object-contain"
              />
            </div>
          </div>
          <div className="flex min-w-0 flex-1 flex-col">
            <div className="mb-1 flex items-center gap-2">
              <span className="text-[13px] font-semibold text-gray-800 dark:text-gray-200">OpenChat</span>
            </div>
            <div className="max-w-[min(720px,100%)] rounded-2xl border border-amber-200 bg-amber-50 px-4 py-3 text-[14px] leading-6 text-amber-900 dark:border-amber-900/60 dark:bg-amber-950/30 dark:text-amber-100">
              {timeoutMessage}
            </div>
          </div>
        </article>
      ) : null}
      {isWaitingForResponse &&
        !hasResponse &&
        state.messages.length > 0 &&
        state.messages[state.messages.length - 1].role === 'user' && (
        <article className="group flex w-full">
          <div className="flex-shrink-0 mr-3">
            <div className="flex h-8 w-8 items-center justify-center overflow-hidden rounded-full border border-gray-100 bg-white shadow-sm dark:border-gray-800 dark:bg-gray-900">
              <img
                src="https://unpkg.com/@lobehub/assets-logo@1.2.0/assets/logo-3d.webp"
                alt="OpenChat"
                className="h-6 w-6 object-contain"
              />
            </div>
          </div>
          <div className="flex flex-col flex-1 min-w-0">
            <div className="flex items-center gap-2 mb-1">
              <span className="text-[13px] font-semibold text-gray-800 dark:text-gray-200">OpenChat</span>
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
