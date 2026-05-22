import type { ItemStatus, MessageContentPart, Session, TurnTerminalReasonCode } from './index'

export interface ThreadItemBase {
  id: string
  sessionId: string
  turnId: string
  type: string
  status: ItemStatus
  seq: number
  createdAt?: string
  updatedAt?: string
  parentId?: string | null
}

export interface UserMessageThreadItem extends ThreadItemBase {
  type: 'userMessage'
  content: MessageContentPart[]
}

export interface ReasoningThreadItem extends ThreadItemBase {
  type: 'reasoning'
  content: string[]
}

export interface AgentMessageThreadItem extends ThreadItemBase {
  type: 'agentMessage'
  text: string
  phase?: 'commentary' | 'final' | null
}

export interface AssistantPlaceholderThreadItem extends ThreadItemBase {
  type: 'assistantPlaceholder'
}

export interface GeneratedImageAsset {
  url: string
  objectKey?: string | null
  mimeType: string
  sizeBytes?: number
}

export interface ImageGenerationThreadItem extends ThreadItemBase {
  type: 'imageGeneration'
  prompt: string
  revisedPrompt?: string | null
  model?: string | null
  size?: string | null
  quality?: string | null
  count?: number | null
  images: GeneratedImageAsset[]
  sourceToolCallId?: string | null
  sourceToolName?: string | null
}

export type ThreadItem =
  | UserMessageThreadItem
  | ReasoningThreadItem
  | AgentMessageThreadItem
  | AssistantPlaceholderThreadItem
  | ImageGenerationThreadItem

export interface ThreadTurn {
  id: string
  sessionId: string
  status: string
  startedAt?: string | null
  completedAt?: string | null
  items: ThreadItem[]
  terminalReason?: {
    code: TurnTerminalReasonCode
    message?: string | null
  } | null
}

export interface SessionDetailSnapshotV2 {
  session: Session
  turns: ThreadTurn[]
  historyPage: {
    hasMore: boolean
    nextBeforeTurnId?: string | null
  }
}
