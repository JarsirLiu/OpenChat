import { describe, expect, it } from 'vitest'
import type { ChatStreamEvent, ThreadTurn } from '@openchat/protocol'
import {
  appendOptimisticTurn,
  applyThreadStreamEvent,
  createInitialChatRuntimeV2State,
  hydrateChatRuntimeV2State,
} from './threadRuntime'

const sessionId = 'session_1'
const turnId = 'turn_1'
const at = '2026-05-21T10:00:00.000Z'

const buildRunningTurn = (items: ThreadTurn['items'] = []): ThreadTurn => ({
  id: turnId,
  sessionId,
  status: 'running',
  startedAt: at,
  completedAt: null,
  items,
  terminalReason: null,
})

describe('threadRuntime', () => {
  it('treats a running turn without items as streaming', () => {
    const state = hydrateChatRuntimeV2State([buildRunningTurn()])

    expect(state.isStreaming).toBe(true)
    expect(state.activeTurnId).toBe(turnId)
    expect(state.pending).toBe('thinking')
  })

  it('creates only a running optimistic turn shell', () => {
    const state = appendOptimisticTurn(createInitialChatRuntimeV2State(), {
      id: turnId,
      sessionId,
      startedAt: at,
    })

    expect(state.turns).toHaveLength(1)
    expect(state.turns[0].status).toBe('running')
    expect(state.turns[0].items).toEqual([])
    expect(state.isStreaming).toBe(true)
    expect(state.activeTurnId).toBe(turnId)
  })

  it('hydrates a real user item without creating duplicates', () => {
    const optimistic = appendOptimisticTurn(createInitialChatRuntimeV2State(), {
      id: turnId,
      sessionId,
      startedAt: at,
    })

    const next = applyThreadStreamEvent(optimistic, {
      type: 'item.started',
      sessionId,
      turnId,
      itemId: 'item_user_server',
      at,
      item: {
        id: 'item_user_server',
        turnId,
        kind: 'message',
        status: 'completed',
        role: 'user',
        content: [{ type: 'text', text: '你可以生成图片？' }],
      },
    } satisfies ChatStreamEvent)

    expect(next.turns[0].items).toHaveLength(1)
    expect(next.turns[0].items[0].id).toBe('item_user_server')
    expect(next.turns[0].items[0].type).toBe('userMessage')
  })

  it('projects tool calls onto the optimistic turn shell without fake user items', () => {
    const optimistic = appendOptimisticTurn(createInitialChatRuntimeV2State(), {
      id: turnId,
      sessionId,
      startedAt: at,
    })

    const next = applyThreadStreamEvent(optimistic, {
      type: 'item.tool_call.started',
      sessionId,
      turnId,
      itemId: 'call_1',
      toolCallId: 'call_1',
      parentItemId: null,
      toolName: 'image_gen_gpt_image_2',
      arguments: {
        prompt: 'a cat',
        size: '1024x1024',
      },
      at,
    } satisfies ChatStreamEvent)

    expect(next.turns[0].items).toHaveLength(1)
    expect(next.turns[0].items[0].type).toBe('imageGeneration')
    expect(next.turns[0].items[0].id).toBe('image:call_1')
  })

  it('keeps delta text when assistant started arrives after delta', () => {
    const withDelta = applyThreadStreamEvent(createInitialChatRuntimeV2State(), {
      type: 'item.message.delta',
      sessionId,
      turnId,
      itemId: 'item_assistant_1',
      at,
      delta: '你好',
    } satisfies ChatStreamEvent)

    const next = applyThreadStreamEvent(withDelta, {
      type: 'item.started',
      sessionId,
      turnId,
      itemId: 'item_assistant_1',
      at,
      item: {
        id: 'item_assistant_1',
        turnId,
        kind: 'message',
        status: 'in_progress',
        role: 'assistant',
        text: '',
      },
    } satisfies ChatStreamEvent)

    expect(next.turns[0].items).toHaveLength(1)
    expect(next.turns[0].items[0].type).toBe('agentMessage')
    if (next.turns[0].items[0].type !== 'agentMessage') {
      throw new Error('expected agentMessage')
    }
    expect(next.turns[0].items[0].text).toBe('你好')
  })

  it('keeps a running turn streaming after snapshot-style hydration', () => {
    const state = hydrateChatRuntimeV2State([
      {
        id: 'turn_snapshot',
        sessionId,
        status: 'running',
        startedAt: at,
        completedAt: null,
        terminalReason: null,
        items: [
          {
            id: 'item_user_snapshot',
            type: 'userMessage',
            sessionId,
            turnId: 'turn_snapshot',
            status: 'completed',
            seq: 10,
            content: [{ type: 'text', text: '给我生成一张图' }],
          },
        ],
      },
    ])

    expect(state.isStreaming).toBe(true)
    expect(state.activeTurnId).toBe('turn_snapshot')
    expect(state.pending).toBe('thinking')
  })

  it('keeps image tool timeline ordered across started completed and turn completed', () => {
    const started = applyThreadStreamEvent(createInitialChatRuntimeV2State(), {
      type: 'turn.started',
      sessionId,
      turnId,
      at,
      turn: {
        id: turnId,
        sessionId,
        status: 'running',
        startedAt: at,
        completedAt: null,
        terminalReason: null,
      },
    } satisfies ChatStreamEvent)

    const withUser = applyThreadStreamEvent(started, {
      type: 'item.started',
      sessionId,
      turnId,
      itemId: 'item_user_1',
      at,
      item: {
        id: 'item_user_1',
        turnId,
        kind: 'message',
        status: 'completed',
        role: 'user',
        content: [{ type: 'text', text: '生成一只猫' }],
      },
    } satisfies ChatStreamEvent)

    const withToolStarted = applyThreadStreamEvent(withUser, {
      type: 'item.tool_call.started',
      sessionId,
      turnId,
      itemId: 'call_cat',
      toolCallId: 'call_cat',
      parentItemId: null,
      toolName: 'image_gen_gpt_image_2',
      arguments: {
        prompt: 'a cute cat',
        size: '1024x1024',
        quality: 'high',
      },
      at: '2026-05-21T10:00:01.000Z',
    } satisfies ChatStreamEvent)

    const withToolCompleted = applyThreadStreamEvent(withToolStarted, {
      type: 'item.tool_call.completed',
      sessionId,
      turnId,
      itemId: 'call_cat',
      at: '2026-05-21T10:00:02.000Z',
      item: {
        id: 'call_cat',
        turnId,
        kind: 'tool_call',
        status: 'completed',
        toolCallId: 'call_cat',
        parentItemId: null,
        toolName: 'image_gen_gpt_image_2',
        toolDisplayName: 'GPT Image 2',
        argumentsText: JSON.stringify({
          prompt: 'a cute cat',
          size: '1024x1024',
          quality: 'high',
        }),
        content: [
          {
            type: 'tool_result',
            toolCallId: 'call_cat',
            toolName: 'image_gen_gpt_image_2',
            toolDisplayName: 'GPT Image 2',
            status: 'completed',
            result: { kind: 'tool_result' },
            media: [
              {
                kind: 'image',
                url: 'https://example.com/cat.png',
                mimeType: 'image/png',
                sizeBytes: 1234,
              },
            ],
          },
        ],
      },
    } satisfies ChatStreamEvent)

    const completed = applyThreadStreamEvent(withToolCompleted, {
      type: 'turn.completed',
      sessionId,
      turnId,
      at: '2026-05-21T10:00:03.000Z',
      turn: {
        id: turnId,
        sessionId,
        status: 'completed',
        startedAt: at,
        completedAt: '2026-05-21T10:00:03.000Z',
        terminalReason: null,
      },
    } satisfies ChatStreamEvent)

    expect(completed.turns).toHaveLength(1)
    expect(completed.turns[0].status).toBe('completed')
    expect(completed.isStreaming).toBe(false)
    expect(completed.turns[0].items.map((item) => item.type)).toEqual([
      'userMessage',
      'imageGeneration',
    ])
    expect(completed.turns[0].items.map((item) => item.status)).toEqual([
      'completed',
      'completed',
    ])
    const imageItem = completed.turns[0].items[1]
    expect(imageItem.type).toBe('imageGeneration')
    if (imageItem.type !== 'imageGeneration') {
      throw new Error('expected imageGeneration')
    }
    expect(imageItem.images).toEqual([
      {
        url: 'https://example.com/cat.png',
        mimeType: 'image/png',
        sizeBytes: 1234,
      },
    ])
  })
})
