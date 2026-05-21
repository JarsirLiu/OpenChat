import { describe, expect, it } from 'vitest'
import { API_ERROR_CODES } from '../../lib/apiError'
import { shouldPreserveDraftHandoffState, toRuntimeRequestError } from './useChatWorkspace'

describe('shouldPreserveDraftHandoffState', () => {
  it('preserves preview/runtime state during draft to accepted session handoff when pending id matches', () => {
    expect(
      shouldPreserveDraftHandoffState({
        previousSessionId: '__draft__',
        nextSessionId: 'session_real',
        pendingSessionHandoffId: 'session_real',
        runtimeState: {
          isStreaming: false,
          turns: [],
        },
      }),
    ).toBe(true)
  })

  it('preserves preview/runtime state when streaming state has already moved onto the accepted session', () => {
    expect(
      shouldPreserveDraftHandoffState({
        previousSessionId: '__draft__',
        nextSessionId: 'session_real',
        pendingSessionHandoffId: null,
        runtimeState: {
          isStreaming: true,
          turns: [{ sessionId: 'session_real' }],
        },
      }),
    ).toBe(true)
  })

  it('does not preserve state when switching between established sessions', () => {
    expect(
      shouldPreserveDraftHandoffState({
        previousSessionId: 'session_old',
        nextSessionId: 'session_real',
        pendingSessionHandoffId: 'session_real',
        runtimeState: {
          isStreaming: true,
          turns: [{ sessionId: 'session_real' }],
        },
      }),
    ).toBe(false)
  })

  it('does not preserve state when staying on the draft session', () => {
    expect(
      shouldPreserveDraftHandoffState({
        previousSessionId: '__draft__',
        nextSessionId: '__draft__',
        pendingSessionHandoffId: '__draft__',
        runtimeState: {
          isStreaming: true,
          turns: [{ sessionId: '__draft__' }],
        },
      }),
    ).toBe(false)
  })

  it('does not preserve state when there is no handoff marker and no streaming turn for the accepted session', () => {
    expect(
      shouldPreserveDraftHandoffState({
        previousSessionId: '__draft__',
        nextSessionId: 'session_real',
        pendingSessionHandoffId: null,
        runtimeState: {
          isStreaming: true,
          turns: [{ sessionId: 'other_session' }],
        },
      }),
    ).toBe(false)
  })
})

describe('toRuntimeRequestError', () => {
  it('maps terminal stream failures into visible request errors', () => {
    const error = toRuntimeRequestError({
      code: API_ERROR_CODES.providerAuthenticationFailed.code,
      message: 'Provider `openai` authentication failed. Please update the API key and try again.',
    })

    expect(error).not.toBeNull()
    expect(error?.code).toBe(API_ERROR_CODES.providerAuthenticationFailed.code)
    expect(error?.category).toBe(API_ERROR_CODES.providerAuthenticationFailed.category)
    expect(error?.message).toContain('authentication failed')
  })

  it('returns null when stream runtime has no error payload', () => {
    expect(toRuntimeRequestError(null)).toBeNull()
  })
})
