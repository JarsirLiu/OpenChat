import type {
  ChatStreamEvent,
  ItemStatus,
  MessageContentPart,
  ThreadItem,
  ThreadTurn,
  TurnTerminalReasonCode,
} from '@openchat/protocol'

export interface ChatRuntimeV2State {
  turns: ThreadTurn[]
  itemsById: Record<string, ThreadItem>
  activeTurnId?: string
  pending: 'idle' | 'thinking' | 'reasoning' | 'tool' | 'image'
  isStreaming: boolean
  error?: {
    code?: TurnTerminalReasonCode | null
    message: string
  }
}

export const createInitialChatRuntimeV2State = (): ChatRuntimeV2State => ({
  turns: [],
  itemsById: {},
  isStreaming: false,
  pending: 'idle',
})

const buildAssistantPlaceholderId = (turnId: string) => `assistant-placeholder:${turnId}`

const buildItemsById = (turns: ThreadTurn[]) =>
  Object.fromEntries(
    turns.flatMap((turn) => turn.items.map((item) => [item.id, item] as const)),
  )

export const hydrateChatRuntimeV2State = (turns: ThreadTurn[]): ChatRuntimeV2State => {
  const activeTurn = [...turns]
    .reverse()
    .find(
      (turn) =>
        turn.status === 'running' ||
        turn.items.some((item) => item.status === 'in_progress'),
    )

  return {
    turns,
    itemsById: buildItemsById(turns),
    activeTurnId: activeTurn?.id,
    isStreaming: Boolean(activeTurn),
    pending: activeTurn ? 'thinking' : 'idle',
  }
}

export const appendOptimisticTurn = (
  state: ChatRuntimeV2State,
  target: { id: string; sessionId: string; startedAt?: string | null },
): ChatRuntimeV2State => {
  const existing = state.turns.find((turn) => turn.id === target.id)
  if (existing) {
    return {
      ...hydrateChatRuntimeV2State(state.turns),
      activeTurnId: target.id,
      isStreaming: true,
      pending: 'thinking',
      error: undefined,
    }
  }

  return {
    ...hydrateChatRuntimeV2State([
      ...state.turns,
      {
        id: target.id,
        sessionId: target.sessionId,
        status: 'running',
        startedAt: target.startedAt ?? null,
        completedAt: null,
        items: [
          {
            id: buildAssistantPlaceholderId(target.id),
            type: 'assistantPlaceholder',
            sessionId: target.sessionId,
            turnId: target.id,
            status: 'in_progress',
            seq: 0,
            createdAt: target.startedAt ?? null ?? undefined,
            updatedAt: target.startedAt ?? null ?? undefined,
          },
        ],
        terminalReason: null,
      },
    ]),
    activeTurnId: target.id,
    isStreaming: true,
    pending: 'thinking',
    error: undefined,
  }
}

const ensureTurn = (
  turns: ThreadTurn[],
  target: { id: string; sessionId: string; status?: string; startedAt?: string | null },
) => {
  const existing = turns.find((turn) => turn.id === target.id)
  if (existing) {
    return turns.map((turn) =>
      turn.id === target.id
        ? {
            ...turn,
            status: target.status ?? turn.status,
            startedAt: target.startedAt ?? turn.startedAt,
          }
        : turn,
    )
  }

  return [
    ...turns,
    {
      id: target.id,
      sessionId: target.sessionId,
      status: target.status ?? 'running',
      startedAt: target.startedAt ?? null,
      completedAt: null,
      items: [],
      terminalReason: null,
    },
  ]
}

const upsertItem = (
  turns: ThreadTurn[],
  turnId: string,
  item: ThreadItem,
) =>
  turns.map((turn) => {
    if (turn.id !== turnId) {
      return turn
    }
    const existingIndex = turn.items.findIndex((candidate) => candidate.id === item.id)
    if (existingIndex >= 0) {
      const nextItems = [...turn.items]
      nextItems[existingIndex] = item
      return { ...turn, items: nextItems.sort((left, right) => left.seq - right.seq) }
    }
    return { ...turn, items: [...turn.items, item].sort((left, right) => left.seq - right.seq) }
  })

const getNextSeq = (turn: ThreadTurn) =>
  turn.items.reduce((max, item) => Math.max(max, item.seq), -1) + 1

const clearAssistantPlaceholder = (turns: ThreadTurn[], turnId: string) =>
  turns.map((turn) =>
    turn.id === turnId
      ? {
          ...turn,
          items: turn.items.filter((item) => item.type !== 'assistantPlaceholder'),
        }
      : turn,
  )

const resolveTerminalItemStatus = (turnStatus: string): ItemStatus =>
  turnStatus === 'interrupted' ? 'interrupted' : 'completed'

export const applyThreadStreamEvent = (
  state: ChatRuntimeV2State,
  event: ChatStreamEvent,
): ChatRuntimeV2State => {
  if (event.type === 'session.created' || event.type === 'session.updated') {
    return state
  }

  if (!event.turnId) {
    return state
  }

  if (event.type === 'turn.started') {
    const turns = ensureTurn(state.turns, {
      id: event.turnId,
      sessionId: event.sessionId,
      status: event.turn.status,
      startedAt: event.turn.startedAt ?? event.at,
    })
    return hydrateChatRuntimeV2State(turns)
  }

  const ensuredTurns = ensureTurn(state.turns, {
    id: event.turnId,
    sessionId: event.sessionId,
    status: 'running',
    startedAt: event.at,
  })
  const ensuredTurn = ensuredTurns.find((turn) => turn.id === event.turnId)
  if (!ensuredTurn) {
    return state
  }

  if (event.type === 'item.started') {
    const nextTurns = event.item.role === 'assistant'
      ? clearAssistantPlaceholder(ensuredTurns, event.turnId)
      : ensuredTurns
    const nextTurn = nextTurns.find((turn) => turn.id === event.turnId)
    if (!nextTurn) {
      return state
    }
    const existing = nextTurn.items.find((item) => item.id === event.itemId)
    const seq = existing?.seq ?? getNextSeq(nextTurn)
    const content = Array.isArray(event.item.content)
      ? event.item.content
      : event.item.text
        ? [{ type: 'text', text: event.item.text } satisfies MessageContentPart]
        : []
    const item: ThreadItem =
      event.item.role === 'user'
        ? {
            id: event.itemId,
            type: 'userMessage',
            sessionId: event.sessionId,
            turnId: event.turnId,
            status: event.item.status,
            seq,
            createdAt: event.at,
            updatedAt: event.at,
            content,
          }
        : {
            id: event.itemId,
            type: 'agentMessage',
            sessionId: event.sessionId,
            turnId: event.turnId,
            status: event.item.status,
            seq,
            createdAt: existing?.createdAt ?? event.at,
            updatedAt: event.at,
            text:
              typeof event.item.text === 'string' && event.item.text.length > 0
                ? event.item.text
                : existing?.type === 'agentMessage'
                  ? existing.text
                  : '',
            phase: null,
          }
    return hydrateChatRuntimeV2State(upsertItem(nextTurns, event.turnId, item))
  }

  if (event.type === 'item.message.delta') {
    const nextTurns = clearAssistantPlaceholder(ensuredTurns, event.turnId)
    const nextTurn = nextTurns.find((turn) => turn.id === event.turnId)
    if (!nextTurn) {
      return state
    }
    const targetItem = nextTurn.items.find((item) => item.id === event.itemId)
    if (!targetItem) {
      const seq = getNextSeq(nextTurn)
      return hydrateChatRuntimeV2State(
        upsertItem(nextTurns, event.turnId, {
          id: event.itemId,
          type: 'agentMessage',
          sessionId: event.sessionId,
          turnId: event.turnId,
          status: 'in_progress',
          seq,
          createdAt: event.at,
          updatedAt: event.at,
          text: event.delta,
          phase: null,
        }),
      )
    }
    if (targetItem.type !== 'agentMessage') {
      return hydrateChatRuntimeV2State(nextTurns)
    }
    return hydrateChatRuntimeV2State(
      upsertItem(nextTurns, event.turnId, {
        ...targetItem,
        status: 'in_progress',
        text: `${targetItem.text}${event.delta}`,
        updatedAt: event.at,
      }),
    )
  }

  if (event.type === 'reasoning.started' || event.type === 'reasoning.completed') {
    const nextTurns = clearAssistantPlaceholder(ensuredTurns, event.turnId)
    const nextTurn = nextTurns.find((turn) => turn.id === event.turnId)
    if (!nextTurn) {
      return state
    }
    const existing = nextTurn.items.find((item) => item.id === event.itemId)
    const seq = existing?.seq ?? getNextSeq(nextTurn)
    return hydrateChatRuntimeV2State(
      upsertItem(nextTurns, event.turnId, {
        id: event.itemId,
        type: 'reasoning',
        sessionId: event.sessionId,
        turnId: event.turnId,
        status: event.item.status,
        seq,
        createdAt: existing?.createdAt ?? event.at,
        updatedAt: event.at,
        content: event.item.text ? [event.item.text] : [],
      }),
    )
  }

  if (event.type === 'reasoning.delta') {
    const nextTurns = clearAssistantPlaceholder(ensuredTurns, event.turnId)
    const nextTurn = nextTurns.find((turn) => turn.id === event.turnId)
    if (!nextTurn) {
      return state
    }
    const existing = nextTurn.items.find((item) => item.id === event.itemId)
    const seq = existing?.seq ?? getNextSeq(nextTurn)
    const previous = existing?.type === 'reasoning' ? existing.content.join('\n\n') : ''
    return hydrateChatRuntimeV2State(
      upsertItem(nextTurns, event.turnId, {
        id: event.itemId,
        type: 'reasoning',
        sessionId: event.sessionId,
        turnId: event.turnId,
        status: 'in_progress',
        seq,
        createdAt: existing?.createdAt ?? event.at,
        updatedAt: event.at,
        content: [`${previous}${event.delta}`],
      }),
    )
  }

  if (event.type === 'item.tool_call.started') {
    const nextTurns = clearAssistantPlaceholder(ensuredTurns, event.turnId)
    const nextTurn = nextTurns.find((turn) => turn.id === event.turnId)
    if (!nextTurn) {
      return state
    }
    const parsedArguments = (() => {
      if (!event.arguments || typeof event.arguments !== 'object') {
        return null
      }
      return event.arguments as Record<string, unknown>
    })()
    const imageItemId = `image:${event.toolCallId}`
    const existing = nextTurn.items.find((item) => item.id === imageItemId)
    const seq = existing?.seq ?? getNextSeq(nextTurn)
    return hydrateChatRuntimeV2State(
      upsertItem(nextTurns, event.turnId, {
        id: imageItemId,
        type: 'imageGeneration',
        sessionId: event.sessionId,
        turnId: event.turnId,
        status: 'in_progress',
        seq,
        createdAt: existing?.createdAt ?? event.at,
        updatedAt: event.at,
        parentId: event.parentItemId ?? null,
        prompt:
          typeof parsedArguments?.prompt === 'string'
            ? parsedArguments.prompt
            : existing?.type === 'imageGeneration'
              ? existing.prompt
              : '',
        revisedPrompt: existing?.type === 'imageGeneration' ? existing.revisedPrompt : null,
        model: event.toolName,
        size:
          typeof parsedArguments?.size === 'string'
            ? parsedArguments.size
            : existing?.type === 'imageGeneration'
              ? existing.size
              : null,
        quality:
          typeof parsedArguments?.quality === 'string'
            ? parsedArguments.quality
            : existing?.type === 'imageGeneration'
              ? existing.quality
              : null,
        count:
          typeof parsedArguments?.n === 'number'
            ? parsedArguments.n
            : existing?.type === 'imageGeneration'
              ? existing.count
              : null,
        images: existing?.type === 'imageGeneration' ? existing.images : [],
        sourceToolCallId: event.toolCallId,
        sourceToolName: event.toolName,
      }),
    )
  }

  if (event.type === 'item.tool_call.completed') {
    const nextTurns = clearAssistantPlaceholder(ensuredTurns, event.turnId)
    const nextTurn = nextTurns.find((turn) => turn.id === event.turnId)
    if (!nextTurn) {
      return state
    }
    const parsedArguments = (() => {
      if (!event.item.argumentsText) {
        return null
      }
      try {
        return JSON.parse(event.item.argumentsText) as Record<string, unknown>
      } catch {
        return null
      }
    })()
    const toolResultPart = event.item.content.find((part) => part.type === 'tool_result')
    const mediaImages =
      toolResultPart?.media?.filter((media) => media.kind === 'image' && media.url.trim()) ?? []
    const imageItemId = `image:${event.item.toolCallId}`
    const existing = nextTurn.items.find((item) => item.id === imageItemId)
    const seq = existing?.seq ?? getNextSeq(nextTurn)
    return hydrateChatRuntimeV2State(
      upsertItem(nextTurns, event.turnId, {
        id: imageItemId,
        type: 'imageGeneration',
        sessionId: event.sessionId,
        turnId: event.turnId,
        status: event.item.status,
        seq,
        createdAt: existing?.createdAt ?? event.at,
        updatedAt: event.at,
        parentId: event.item.parentItemId ?? null,
        prompt:
          typeof parsedArguments?.prompt === 'string'
            ? parsedArguments.prompt
            : existing?.type === 'imageGeneration'
              ? existing.prompt
              : '',
        revisedPrompt: existing?.type === 'imageGeneration' ? existing.revisedPrompt : null,
        model: event.item.toolName,
        size:
          typeof parsedArguments?.size === 'string'
            ? parsedArguments.size
            : existing?.type === 'imageGeneration'
              ? existing.size
              : null,
        quality:
          typeof parsedArguments?.quality === 'string'
            ? parsedArguments.quality
            : existing?.type === 'imageGeneration'
              ? existing.quality
              : null,
        count:
          typeof parsedArguments?.n === 'number'
            ? parsedArguments.n
            : existing?.type === 'imageGeneration'
              ? existing.count
              : null,
        images:
          mediaImages.length > 0
            ? mediaImages.map((image) => ({
                url: image.url,
                objectKey: image.objectKey ?? null,
                mimeType: image.mimeType,
                sizeBytes: image.sizeBytes,
              }))
            : existing?.type === 'imageGeneration'
              ? existing.images
              : [],
        sourceToolCallId: event.item.toolCallId,
        sourceToolName: event.item.toolName,
      }),
    )
  }

  if (event.type === 'turn.completed') {
    const turns = ensuredTurns.map((turn) =>
      turn.id === event.turnId
        ? {
            ...turn,
            status: event.turn.status,
            completedAt: event.turn.completedAt ?? event.at,
            terminalReason: event.turn.terminalReason
              ? {
                  code: event.turn.terminalReason.code,
                  message: event.turn.terminalReason.message,
                }
              : null,
            items: turn.items.map((item) =>
              item.status === 'in_progress'
                ? {
                    ...item,
                    status: resolveTerminalItemStatus(event.turn.status),
                    updatedAt: event.at,
                  }
                : item,
            ),
          }
        : turn,
    )
    return hydrateChatRuntimeV2State(turns)
  }

  if (event.type === 'turn.failed') {
    const turns = ensuredTurns.map((turn) =>
      turn.id === event.turnId
        ? {
            ...turn,
            status: 'failed',
            completedAt: event.at,
            terminalReason: event.error.code
              ? { code: event.error.code, message: event.error.message }
              : null,
            items: turn.items.map((item) =>
              item.status === 'in_progress'
                ? { ...item, status: 'failed' as const, updatedAt: event.at }
                : item,
            ),
          }
        : turn,
    )
    const next = hydrateChatRuntimeV2State(turns)
    return {
      ...next,
      error: {
        code: event.error.code,
        message: event.error.message,
      },
    }
  }

  return hydrateChatRuntimeV2State(ensuredTurns)
}
