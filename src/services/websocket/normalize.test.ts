import { describe, expect, it } from 'vitest'
import { backoffDelay } from './RaceSocket'
import { normalizeMessage } from './normalize'

describe('websocket normalization', () => {
  it('parses a GAME_PROGRESS payload from a JSON string', () => {
    const r = normalizeMessage(
      JSON.stringify({ type: 'GAME_PROGRESS', racerId: 'r1', progress: { percentage: 76 } }),
    )
    expect(r.ok && r.message.type).toBe('GAME_PROGRESS')
  })

  it('rejects malformed JSON without throwing', () => {
    expect(normalizeMessage('{nope')).toEqual({ ok: false, reason: 'malformed' })
  })

  it('flags unknown event types', () => {
    expect(normalizeMessage({ type: 'SOMETHING_NEW' })).toEqual({
      ok: false,
      reason: 'unknown',
      type: 'SOMETHING_NEW',
    })
  })

  it('rejects messages missing required fields', () => {
    expect(normalizeMessage({ type: 'ITEM_ACQUIRED', racerId: 'r1' }).ok).toBe(false)
    expect(normalizeMessage({ type: 'CLOCK_SNAPSHOT', clocks: 'x' }).ok).toBe(false)
  })

  it('backs off exponentially, capped', () => {
    const fixed = () => 0.5
    expect(backoffDelay(1, 1000, 20000, fixed)).toBe(1000)
    expect(backoffDelay(3, 1000, 20000, fixed)).toBe(4000)
    expect(backoffDelay(10, 1000, 20000, fixed)).toBe(20000)
  })
})
