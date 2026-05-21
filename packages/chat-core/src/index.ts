import type {
  ChatMessage,
  ChatStreamEvent,
  ItemStatus,
  MessageContentPart,
  MessageContentToolResult,
  TurnTerminalReasonCode,
  ToolMedia,
  ToolCallSummary,
} from '@openchat/protocol'
export * from './threadRuntime'

export interface ToolCallViewModel {
  id: string
  name: string
  displayName?: string
  argumentsText: string
  resultText: string
  status: ItemStatus
  media: ToolMedia[]
}

export interface ChatRuntimeState {
  messages: ChatMessage[]
  toolCalls: Record<string, ToolCallViewModel>
  pendingToolCallsByMessageId: Record<string, ToolCallSummary[]>
  isStreaming: boolean
  pending: 'idle' | 'thinking' | 'reasoning' | 'tool' | 'image'
  activeTurnId?: string
  error?: {
    code?: TurnTerminalReasonCode | null
    message: string
  }
}

export const createInitialChatRuntimeState = (): ChatRuntimeState => ({
  messages: [],
  toolCalls: {},
  pendingToolCallsByMessageId: {},
  isStreaming: false,
  pending: 'idle',
})

const deriveHydratedStreamingState = (messages: ChatMessage[]) => {
  const inProgressMessages = messages.filter((message) => message.status === 'in_progress')
  const activeTurnId = inProgressMessages[inProgressMessages.length - 1]?.turnId

  if (!activeTurnId) {
    return {
      isStreaming: false,
      pending: 'idle' as const,
      activeTurnId: undefined,
    }
  }

  const activeTurnMessages = messages.filter((message) => message.turnId === activeTurnId)
  const hasRunningToolCall = activeTurnMessages.some((message) =>
    (message.toolCalls ?? []).some((toolCall) => toolCall.status === 'in_progress'),
  )
  const hasRunningReasoning = activeTurnMessages.some(
    (message) => message.role === 'reasoning' && message.status === 'in_progress',
  )

  return {
    isStreaming: true,
    pending: hasRunningToolCall
      ? ('tool' as const)
      : hasRunningReasoning
        ? ('reasoning' as const)
        : ('thinking' as const),
    activeTurnId,
  }
}

export const hydrateChatRuntimeState = (messages: ChatMessage[]): ChatRuntimeState => {
  const toolCalls = buildToolCallStateFromMessages(messages)
  const streamingState = deriveHydratedStreamingState(messages)

  return {
    messages,
    toolCalls,
    pendingToolCallsByMessageId: {},
    isStreaming: streamingState.isStreaming,
    pending: streamingState.pending,
    activeTurnId: streamingState.activeTurnId,
  }
}

const isToolResultPart = (
  part: MessageContentPart,
): part is MessageContentToolResult => part.type === 'tool_result'

const getToolResultPart = (
  content: MessageContentPart[] | undefined,
): MessageContentToolResult | undefined => content?.find(isToolResultPart)

const formatToolResultText = (result: MessageContentToolResult['result']): string => {
  if (typeof result === 'string') {
    return result
  }

  if (result == null) {
    return ''
  }

  const clone = JSON.parse(JSON.stringify(result)) as Record<string, unknown>

  const output =
    clone && typeof clone === 'object' && 'output' in clone
      ? (clone as Record<string, unknown>).output
      : null

  if (output && typeof output === 'object' && !Array.isArray(output)) {
    delete (output as Record<string, unknown>).downloadUrl
  }

  return JSON.stringify(clone, null, 2)
}

const buildToolCallStateFromMessages = (
  messages: ChatMessage[],
): Record<string, ToolCallViewModel> => {
  const toolCalls: Record<string, ToolCallViewModel> = {}

  for (const message of messages) {
    if (message.role === 'assistant' && message.toolCalls?.length) {
      for (const toolCall of message.toolCalls) {
        const toolResult = getToolResultPart(toolCall.content)
        toolCalls[toolCall.id] = {
          id: toolCall.id,
          name: toolCall.name,
          displayName: toolCall.displayName,
          argumentsText: toolCall.argumentsText ?? '',
          resultText: formatToolResultText(toolResult?.result ?? null),
          status: toolCall.status ?? 'completed',
          media: toolResult?.media ?? [],
        }
      }
    }
  }

  return toolCalls
}

const mergeStreamChunk = (current: string, incoming: string): string => {
  if (!incoming) {
    return current
  }

  if (!current) {
    return incoming
  }

  if (incoming.startsWith(current)) {
    return incoming
  }

  if (current.endsWith(incoming)) {
    return current
  }

  return `${current}${incoming}`
}

const appendText = (parts: MessageContentPart[], delta: string): MessageContentPart[] => {
  const last = parts[parts.length - 1]
  if (last?.type === 'text') {
    return [...parts.slice(0, -1), { ...last, text: mergeStreamChunk(last.text, delta) }]
  }
  return [...parts, { type: 'text', text: delta }]
}

const normalizeEventMessageContent = (
  content: MessageContentPart[] | null | undefined,
  text: string | null | undefined,
): MessageContentPart[] => {
  if (Array.isArray(content)) {
    return content
  }

  return text ? [{ type: 'text', text }] : []
}

const ensureMessage = (
  messages: ChatMessage[],
  target: {
    id: string
    role: ChatMessage['role']
    turnId: string
    status: ItemStatus
    content?: MessageContentPart[]
    createdAt?: string
    updatedAt?: string
  },
): ChatMessage[] => {
  const existing = messages.find((message) => message.id === target.id)
  if (existing) {
    return messages.map((message) =>
      message.id === target.id
        ? {
            ...message,
            status: target.status,
            content: target.content ?? message.content,
            createdAt: target.createdAt ?? message.createdAt,
            updatedAt: target.updatedAt ?? message.updatedAt,
          }
        : message,
    )
  }

  return [
    ...messages,
    {
      id: target.id,
      role: target.role,
      turnId: target.turnId,
      status: target.status,
      content: target.content ?? [],
      createdAt: target.createdAt,
      updatedAt: target.updatedAt,
    },
  ]
}

const attachPendingDecorations = (
  state: ChatRuntimeState,
  message: ChatMessage,
): ChatMessage => {
  const pendingToolCalls = state.pendingToolCallsByMessageId[message.id] ?? []

  const nextToolCalls = [
    ...(message.toolCalls ?? []),
    ...pendingToolCalls.filter(
      (toolCall) => !(message.toolCalls ?? []).some((existing) => existing.id === toolCall.id),
    ),
  ]

  return {
    ...message,
    toolCalls: nextToolCalls.length ? nextToolCalls : message.toolCalls,
    content: message.content,
  }
}

const upsertMessageText = (
  messages: ChatMessage[],
  messageId: string,
  delta: string,
): ChatMessage[] =>
  messages.map((message) =>
    message.id === messageId
      ? {
          ...message,
          content: appendText(message.content, delta),
          status: 'in_progress',
          updatedAt: new Date().toISOString(),
        }
      : message,
  )

const updateMessageStatusByTurn = (
  messages: ChatMessage[],
  turnId: string,
  status: ItemStatus,
): ChatMessage[] =>
  messages.map((message) =>
    message.turnId === turnId && (message.role === 'assistant' || message.role === 'reasoning')
      ? {
          ...message,
          status,
        }
      : message,
  )

const upsertReasoningMessage = (
  messages: ChatMessage[],
  target: {
    id: string
    turnId: string
    status: ItemStatus
    content: MessageContentPart[]
    createdAt?: string
    updatedAt?: string
  },
): ChatMessage[] => {
  const existingIndex = messages.findIndex((message) => message.id === target.id)

  if (existingIndex >= 0) {
    return messages.map((message, index) =>
      index === existingIndex
        ? {
            id: target.id,
            role: 'reasoning',
            turnId: target.turnId,
            status: target.status,
            content: target.content,
            createdAt: target.createdAt ?? message.createdAt,
            updatedAt: target.updatedAt ?? message.updatedAt,
          }
        : message,
    )
  }

  const assistantIndex = messages.findIndex(
    (message) => message.role === 'assistant' && message.turnId === target.turnId,
  )

  const nextMessage: ChatMessage = {
    id: target.id,
    role: 'reasoning',
    turnId: target.turnId,
    status: target.status,
    content: target.content,
    createdAt: target.createdAt,
    updatedAt: target.updatedAt,
  }

  if (assistantIndex >= 0) {
    return [...messages.slice(0, assistantIndex), nextMessage, ...messages.slice(assistantIndex)]
  }

  return [...messages, nextMessage]
}

export const applyStreamEvent = (
  state: ChatRuntimeState,
  event: ChatStreamEvent,
): ChatRuntimeState => {
  switch (event.type) {
    case 'turn.started':
      return {
        ...state,
        isStreaming: true,
        pending: 'thinking',
        activeTurnId: event.turnId,
        error: undefined,
      }

    case 'item.started': {
      const nextPending = event.item.role === 'assistant' ? 'thinking' : state.pending

      return {
        ...state,
        isStreaming: true,
        pending: nextPending,
        messages: ensureMessage(state.messages, {
          id: event.itemId,
          role: event.item.role,
          turnId: event.turnId,
          status: event.item.status,
          content: normalizeEventMessageContent(event.item.content, event.item.text),
          createdAt: event.at,
          updatedAt: event.at,
        }).map((message) =>
          message.id === event.itemId ? attachPendingDecorations(state, message) : message,
        ),
      }
    }

    case 'reasoning.started':
      return {
        ...state,
        isStreaming: true,
        pending: 'reasoning',
        messages: upsertReasoningMessage(state.messages, {
          id: event.itemId,
          turnId: event.turnId,
          status: event.item.status,
          content: event.item.text ? [{ type: 'text', text: event.item.text }] : [],
          createdAt: event.at,
          updatedAt: event.at,
        }),
      }

    case 'reasoning.delta': {
      const currentReasoning = state.messages.find(
        (message) => message.role === 'reasoning' && message.id === event.itemId,
      )?.content
        .filter((part): part is Extract<MessageContentPart, { type: 'text' }> => part.type === 'text')
        .map((part) => part.text)
        .join('\n\n')
      const nextReasoning = mergeStreamChunk(currentReasoning ?? '', event.delta)

      return {
        ...state,
        isStreaming: true,
        pending: 'reasoning',
        messages: upsertReasoningMessage(state.messages, {
          id: event.itemId,
          turnId: event.turnId,
          status: 'in_progress',
          content: nextReasoning ? [{ type: 'text', text: nextReasoning }] : [],
          updatedAt: event.at,
        }),
      }
    }

    case 'reasoning.completed':
      return {
        ...state,
        isStreaming: true,
        pending: 'thinking',
        messages: upsertReasoningMessage(state.messages, {
          id: event.itemId,
          turnId: event.turnId,
          status: event.item.status,
          content: event.item.text ? [{ type: 'text', text: event.item.text }] : [],
          updatedAt: event.at,
        }),
      }

    case 'item.message.delta':
      return {
        ...state,
        isStreaming: true,
        pending: 'thinking',
        messages: upsertMessageText(state.messages, event.itemId, event.delta).map((message) =>
          message.id === event.itemId ? { ...message, updatedAt: event.at } : message,
        ),
      }

    case 'item.tool_call.started':
      if (!event.parentItemId) {
        return state
      }

      const toolSummary: ToolCallSummary = {
        id: event.toolCallId,
        name: event.toolName,
        parentItemId: event.parentItemId,
        argumentsText: event.arguments ? JSON.stringify(event.arguments, null, 2) : '',
        status: 'in_progress',
        content: [],
      }

      return {
        ...state,
        pending: 'tool',
        toolCalls: {
          ...state.toolCalls,
          [event.toolCallId]: {
            id: event.toolCallId,
            name: event.toolName,
            argumentsText: event.arguments ? JSON.stringify(event.arguments, null, 2) : '',
            resultText: '',
            status: 'in_progress',
            media: [],
          },
        },
        pendingToolCallsByMessageId: {
          ...state.pendingToolCallsByMessageId,
          [event.parentItemId]: [
            ...(state.pendingToolCallsByMessageId[event.parentItemId] ?? []),
            toolSummary,
          ].filter(
            (entry, index, list) => list.findIndex((candidate) => candidate.id === entry.id) === index,
          ),
        },
        messages: state.messages.map((message) =>
          message.id === event.parentItemId
            ? attachPendingDecorations(
                {
                  ...state,
                  pendingToolCallsByMessageId: {
                    ...state.pendingToolCallsByMessageId,
                    [event.parentItemId]: [
                      ...(state.pendingToolCallsByMessageId[event.parentItemId] ?? []),
                      toolSummary,
                    ],
                  },
                },
                message,
              )
            : message,
        ),
      }

    case 'item.tool_call.arguments.delta':
      return {
        ...state,
        pending: 'tool',
        toolCalls: {
          ...state.toolCalls,
          [event.toolCallId]: {
            ...state.toolCalls[event.toolCallId],
            argumentsText: (state.toolCalls[event.toolCallId]?.argumentsText ?? '') + event.delta,
          },
        },
      }

    case 'item.tool_call.completed': {
      const toolResult = getToolResultPart(event.item.content)
      const resultText = formatToolResultText(toolResult?.result ?? null)
      const parentItemId = event.item.parentItemId ?? undefined

      return {
        ...state,
        pending: 'thinking',
        toolCalls: {
          ...state.toolCalls,
          [event.item.toolCallId]: {
            ...state.toolCalls[event.item.toolCallId],
            id: event.item.toolCallId,
            name: event.item.toolName,
            displayName: event.item.toolDisplayName ?? undefined,
            argumentsText:
              event.item.argumentsText ?? state.toolCalls[event.item.toolCallId]?.argumentsText ?? '',
            resultText,
            status: event.item.status,
            media: toolResult?.media ?? state.toolCalls[event.item.toolCallId]?.media ?? [],
          },
        },
        pendingToolCallsByMessageId: parentItemId
          ? {
              ...state.pendingToolCallsByMessageId,
              [parentItemId]: (state.pendingToolCallsByMessageId[parentItemId] ?? []).map((toolCall) =>
                toolCall.id === event.item.toolCallId
                  ? {
                      ...toolCall,
                      argumentsText:
                        event.item.argumentsText ??
                        state.toolCalls[event.item.toolCallId]?.argumentsText ??
                        toolCall.argumentsText,
                      status: event.item.status,
                      content: event.item.content,
                    }
                  : toolCall,
              ),
            }
          : state.pendingToolCallsByMessageId,
      }
    }

    case 'turn.completed':
      return {
        ...state,
        isStreaming: false,
        pending: 'idle',
        activeTurnId: event.turnId,
        error: undefined,
        messages: updateMessageStatusByTurn(
          state.messages,
          event.turnId,
          event.turn.status === 'interrupted' ? 'interrupted' : 'completed',
        ),
      }

    case 'turn.failed':
      return {
        ...state,
        isStreaming: false,
        pending: 'idle',
        activeTurnId: event.turnId,
        messages: updateMessageStatusByTurn(state.messages, event.turnId, 'failed'),
        error: {
          code: event.error.code,
          message: event.error.message,
        },
      }

    case 'session.created':
    case 'session.updated':
      return state

    default:
      return state
  }
}
