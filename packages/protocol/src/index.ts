export const PROTOCOL_VERSION = 'openchat-stream-v1' as const
export const RESOURCE_MODEL = 'session' as const
export * from './errorCodes'

export type SessionStatus = 'idle' | 'running' | 'completed' | 'interrupted' | 'failed' | 'archived'
export type TurnStatus = 'running' | 'completed' | 'interrupted' | 'failed'
export type ItemStatus = 'in_progress' | 'completed' | 'interrupted' | 'failed'
export type TurnTerminalReasonCode =
  | 'user_requested'
  | 'session_recovered'
  | 'model_connect_timeout'
  | 'model_stream_idle_timeout'
  | 'provider_authentication_failed'
  | 'upstream_error'
  | 'runtime_error'

export interface TurnTerminalReason {
  code: TurnTerminalReasonCode
  message?: string | null
}

export interface Session {
  id: string
  title: string | null
  status: SessionStatus
  createdAt: string
  updatedAt: string
}

export interface Turn {
  id: string
  sessionId: string
  status: TurnStatus
  startedAt?: string | null
  completedAt?: string | null
  terminalReason?: TurnTerminalReason | null
}

export interface MessageContentText {
  type: 'text'
  text: string
}

export interface MessageContentImage {
  type: 'image'
  url: string
  alt: string
}

export type MessageContentPart = MessageContentText | MessageContentImage

export interface ToolMedia {
  kind: string
  url: string
  mimeType: string
  sizeBytes: number
}

export interface ToolCallSummary {
  id: string
  name: string
  displayName?: string
  parentItemId?: string
  argumentsText?: string
  result?: Record<string, unknown> | string | null
  status?: ItemStatus
  media?: ToolMedia[] | null
}

export interface ChatMessage {
  id: string
  role: 'user' | 'assistant' | 'reasoning'
  turnId: string
  status: ItemStatus
  content: MessageContentPart[]
  toolCalls?: ToolCallSummary[]
  createdAt?: string
  updatedAt?: string
}

export interface MessageItem {
  id: string
  turnId: string
  kind: 'message'
  status: ItemStatus
  role: 'user' | 'assistant'
  text?: string | null
  content?: MessageContentPart[] | null
}

export interface ReasoningItem {
  id: string
  turnId: string
  kind: 'reasoning'
  status: ItemStatus
  text?: string | null
}

export interface ToolCallItem {
  id: string
  turnId: string
  kind: 'tool_call'
  status: ItemStatus
  toolCallId: string
  parentItemId?: string | null
  toolName: string
  toolDisplayName?: string | null
  argumentsText?: string | null
  result?: Record<string, unknown> | string | null
  media?: ToolMedia[] | null
}

export interface BaseEvent {
  type: string
  sessionId: string
  turnId?: string | null
  itemId?: string | null
  at: string
}

export interface SessionCreatedEvent extends BaseEvent {
  type: 'session.created'
  session: Session
}

export interface SessionUpdatedEvent extends BaseEvent {
  type: 'session.updated'
  session: Session
}

export interface TurnStartedEvent extends BaseEvent {
  type: 'turn.started'
  turnId: string
  turn: Turn
}

export interface ItemStartedEvent extends BaseEvent {
  type: 'item.started'
  turnId: string
  itemId: string
  item: MessageItem
}

export interface ItemMessageDeltaEvent extends BaseEvent {
  type: 'item.message.delta'
  turnId: string
  itemId: string
  delta: string
}

export interface ReasoningStartedEvent extends BaseEvent {
  type: 'reasoning.started'
  turnId: string
  itemId: string
  item: ReasoningItem
}

export interface ReasoningDeltaEvent extends BaseEvent {
  type: 'reasoning.delta'
  turnId: string
  itemId: string
  delta: string
}

export interface ReasoningCompletedEvent extends BaseEvent {
  type: 'reasoning.completed'
  turnId: string
  itemId: string
  item: ReasoningItem
}

export interface ItemToolCallStartedEvent extends BaseEvent {
  type: 'item.tool_call.started'
  turnId: string
  itemId: string
  toolCallId: string
  parentItemId?: string | null
  toolName: string
  arguments?: Record<string, unknown> | null
}

export interface ItemToolCallArgumentsDeltaEvent extends BaseEvent {
  type: 'item.tool_call.arguments.delta'
  turnId: string
  itemId: string
  toolCallId: string
  parentItemId?: string | null
  delta: string
}

export interface ItemToolCallCompletedEvent extends BaseEvent {
  type: 'item.tool_call.completed'
  turnId: string
  itemId: string
  item: ToolCallItem
}

export interface ImageGeneratedEvent extends BaseEvent {
  type: 'image_generated'
  media: ToolMedia
  targetItemId?: string | null
  canvasId?: string | null
}

export interface TurnCompletedEvent extends BaseEvent {
  type: 'turn.completed'
  turnId: string
  turn: Turn
}

export interface TurnFailedEvent extends BaseEvent {
  type: 'turn.failed'
  turnId: string
  error: {
    code?: TurnTerminalReasonCode | null
    message: string
  }
}

export type ChatStreamEvent =
  | SessionCreatedEvent
  | SessionUpdatedEvent
  | TurnStartedEvent
  | ItemStartedEvent
  | ItemMessageDeltaEvent
  | ReasoningStartedEvent
  | ReasoningDeltaEvent
  | ReasoningCompletedEvent
  | ItemToolCallStartedEvent
  | ItemToolCallArgumentsDeltaEvent
  | ItemToolCallCompletedEvent
  | ImageGeneratedEvent
  | TurnCompletedEvent
  | TurnFailedEvent

const isRecord = (value: unknown): value is Record<string, unknown> =>
  Boolean(value && typeof value === 'object')

export const normalizeStreamEvent = (value: unknown): ChatStreamEvent | null => {
  if (
    !isRecord(value) ||
    typeof value.type !== 'string' ||
    typeof value.sessionId !== 'string' ||
    typeof value.at !== 'string'
  ) {
    return null
  }

  return value as unknown as ChatStreamEvent
}
