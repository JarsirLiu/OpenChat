# OpenChat Thread Items Architecture

## Why

The current chat stack models assistant output as:

- `ChatMessage`
- optional attached `toolCalls[]`
- special frontend-only rendering splits for image generation

This leads to two product problems:

1. image generation looks like a separate assistant reply instead of part of the same assistant turn
2. ordering between reasoning, assistant text, and generated images is not protocol-native

The root cause is architectural:

- ordering truth is not represented directly in the data model
- tool/image UI grouping is decided in the frontend render layer
- history snapshots and live stream events do not share one canonical item model

## Goal

Move OpenChat to a Codex-style item timeline:

- history is returned as `Turn[]`
- each `Turn` owns an ordered `items[]`
- each item is a first-class domain object
- frontend runtime stores items, not `message + toolCalls`
- visual grouping is separate from item ordering

## Design Principles

1. `Turn` is the unit of execution lifecycle.
2. `ThreadItem` is the unit of transcript rendering.
3. Ordering must be explicit and stable via `seq`.
4. History and streaming must share the same item semantics.
5. Product-domain items should be first class.

## Item Model

Recommended first-generation item set:

- `userMessage`
- `reasoning`
- `agentMessage`
- `imageGeneration`
- `webSearch`
- `fileOperation`
- `error`

The transcript should prefer domain items over generic tool items. When a domain item is produced by
an internal tool call, the item may retain `sourceToolCallId` and `sourceToolName` for diagnostics,
but the primary transcript should render the domain item directly.

## Ordering

Every item in a turn must have:

- `seq`
- `createdAt`
- `updatedAt`

`seq` is the canonical ordering key. Timestamps are for display and diagnostics only.

## Streaming

Target streaming lifecycle:

- `turn.started`
- `item.started`
- `item.delta`
- `item.updated`
- `item.completed`
- `turn.completed`
- `turn.failed`

Image generation no longer emits side-channel `image_generated` events as a primary render trigger.
`imageGeneration` is updated and rendered as a first-class `ThreadItem`.

## History API

Target history shape:

```ts
interface SessionDetail {
  session: Session
  turns: ThreadTurn[]
  historyPage: {
    hasMore: boolean
    nextBeforeTurnId?: string | null
  }
}
```

Each `ThreadTurn` contains an ordered `items[]`.

## Frontend Runtime

Target runtime shape:

```ts
interface ChatRuntimeV2State {
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
```

The runtime stores transcript facts. Display grouping is derived separately.

## UI Rendering

Rendering should have two layers:

1. ordered transcript rendering by `turn.startedAt` and `item.seq`
2. visual grouping that places adjacent assistant-side items under one assistant frame

This keeps:

- ordering as a data concern
- avatar ownership as a presentation concern

## Migration Plan

### Phase 1: Parallel Model

- add `ThreadItem` and `ThreadTurn` protocol types
- add `turns` to session detail payloads while keeping `messages`
- add frontend normalization for `turns`
- add item-based runtime skeleton without replacing current UI

### Phase 2: Server Truth

- introduce item-oriented server assembly
- assign stable `seq` values within each turn
- return `turns[].items[]` as canonical history

### Phase 3: Frontend Runtime Switch

- move primary chat runtime from `ChatMessage[]` to `ThreadTurn[]`
- remove assistant `toolCalls[]` as render-time ordering source

### Phase 4: Cleanup

- remove image-specific split rendering
- remove attached tool-call transcript rendering for image generation
- deprecate message-first history model

## Mainline vs Compatibility Contract

The repository must treat the two models as different lanes with strict ownership:

- Mainline lane: `ThreadTurn[]` + `ThreadItem[]`
- Compatibility lane: legacy `ChatMessage[]` + `toolCalls[]`

Rules:

1. New product behavior must land only in the mainline lane.
2. Compatibility lane is read-only for behavior, write-only for adapters and bugfixes.
3. Any file that consumes legacy message-first data must include a clear compatibility marker comment.
4. UI components that render from legacy models must be marked as temporary and excluded from new feature wiring.

This prevents regressions where new capabilities accidentally get attached to the old model.

## Compatibility Deletion Gate

Legacy message-first support can be removed only after all checks pass:

1. Session history API can serve all active sessions from `turns.items`.
2. Streaming updates produce and reconcile item lifecycle events without relying on legacy message assembly.
3. Frontend transcript rendering no longer depends on `ChatMessage[]`.
4. Operations confirms production sessions remain readable after the migration window.

When these checks pass, compatibility modules can be deleted in one cleanup release.

## Notes on Current Parallel Implementation

The initial parallel `turns` payload can be assembled from the current `messages + tool_calls`
snapshot. This transitional shape is useful for integration, but it is still a compatibility
projection. Long term, persistence should move toward native item storage or a server-side rollout
history builder that is the single source of truth.

Current status in repository:

- Main chat UI renders from `ThreadConversation` only.
- Session detail history reads from `turns.items` only.
- Stream protocol and V2 runtime no longer depend on `image_generated`.
- Session-level version gate is active:
  - new sessions are created with `transcript_version = "v2"`
  - migrated sessions continue on `thread_items` as the mainline model
- Runtime responsibilities are now split explicitly:
  - `TurnLoop` owns streaming control, cancellation, retries, and step boundaries
  - `ModelEventDispatcher` owns `ModelStreamEvent -> transcript rule` mapping
  - `TranscriptProjector` owns `ThreadItem` projection, SSE emission, and `thread_items` persistence
- New turn writes flow through a centralized `TranscriptProjector`.
  - model deltas are consumed in memory and projected immediately into `ThreadItem` lifecycle updates
  - the database stores the latest aggregated `ThreadItem` state rather than token-level delta rows
  - streaming to the frontend and persistence now share the same projection entrypoint
  - transcript persistence is strict: `thread_items` writes happen before SSE emission
  - projection persistence failures terminate the current turn with `transcript_projection_failed`
  - SSE delivery is best-effort, but transcript storage is the required source of truth
- `imageGeneration` items are created at tool-call start and closed on tool-call completion or failure.
- New assistant output segments after a tool call create a fresh `agentMessage` item instead of mutating the pre-tool item.
- Legacy `messages` / `tool_calls` tables are retained only as migration source/history artifacts and are no longer used by the main read/write path.
