import type { ThreadItem, ThreadTurn } from '@openchat/protocol'

interface SessionThreadItemPayload {
  id?: string
  type?: string
  sessionId?: string
  turnId?: string
  status?: 'in_progress' | 'completed' | 'interrupted' | 'failed'
  seq?: number
  createdAt?: string
  updatedAt?: string
  parentId?: string | null
  content?: unknown
  text?: string | null
  prompt?: string | null
  revisedPrompt?: string | null
  model?: string | null
  size?: string | null
  quality?: string | null
  count?: number | null
  sourceToolCallId?: string | null
  sourceToolName?: string | null
  images?: Array<{
    url?: string
    mimeType?: string
    sizeBytes?: number
  }>
}

interface SessionTurnPayload {
  id?: string
  sessionId?: string
  status?: string
  startedAt?: string | null
  completedAt?: string | null
  items?: SessionThreadItemPayload[]
}

const normalizeItem = (item: SessionThreadItemPayload): ThreadItem | null => {
  if (
    !item.id ||
    !item.type ||
    !item.turnId ||
    !item.sessionId ||
    !item.status ||
    typeof item.seq !== 'number'
  ) {
    return null
  }

  if (item.type === 'userMessage') {
    return {
      id: item.id,
      type: 'userMessage',
      sessionId: item.sessionId,
      turnId: item.turnId,
      status: item.status,
      seq: item.seq,
      createdAt: item.createdAt,
      updatedAt: item.updatedAt,
      parentId: item.parentId,
      content: Array.isArray(item.content) ? (item.content as never) : [],
    }
  }

  if (item.type === 'reasoning') {
    return {
      id: item.id,
      type: 'reasoning',
      sessionId: item.sessionId,
      turnId: item.turnId,
      status: item.status,
      seq: item.seq,
      createdAt: item.createdAt,
      updatedAt: item.updatedAt,
      parentId: item.parentId,
      content: item.text ? [item.text] : [],
    }
  }

  if (item.type === 'imageGeneration') {
    return {
      id: item.id,
      type: 'imageGeneration',
      sessionId: item.sessionId,
      turnId: item.turnId,
      status: item.status,
      seq: item.seq,
      createdAt: item.createdAt,
      updatedAt: item.updatedAt,
      parentId: item.parentId,
      prompt: item.prompt ?? '',
      revisedPrompt: item.revisedPrompt,
      model: item.model,
      size: item.size,
      quality: item.quality,
      count: item.count,
      sourceToolCallId: item.sourceToolCallId,
      sourceToolName: item.sourceToolName,
      images: Array.isArray(item.images)
        ? item.images.flatMap((image) =>
            image?.url && image?.mimeType
              ? [{ url: image.url, mimeType: image.mimeType, sizeBytes: image.sizeBytes }]
              : [],
          )
        : [],
    }
  }

  return {
    id: item.id,
    type: 'agentMessage',
    sessionId: item.sessionId,
    turnId: item.turnId,
    status: item.status,
    seq: item.seq,
    createdAt: item.createdAt,
    updatedAt: item.updatedAt,
    parentId: item.parentId,
    text: item.text ?? '',
    phase: null,
  }
}

export const normalizeSessionTurns = (value: SessionTurnPayload[] | undefined): ThreadTurn[] =>
  Array.isArray(value)
    ? value.flatMap((turn) => {
        if (!turn?.id || !turn.sessionId || !turn.status) {
          return []
        }

        return [
          {
            id: turn.id,
            sessionId: turn.sessionId,
            status: turn.status,
            startedAt: turn.startedAt,
            completedAt: turn.completedAt,
            items: Array.isArray(turn.items)
              ? turn.items
                  .map(normalizeItem)
                  .filter((item): item is ThreadItem => Boolean(item))
                  .sort((left, right) => left.seq - right.seq)
              : [],
            terminalReason: null,
          },
        ]
      })
    : []
