import { describe, expect, it } from 'vitest'
import { clock, isoToLocalInput, localInputToIso, slotState } from './schedule'

const slot = { startUtc: '2026-10-07T18:00:00.000Z', endUtc: '2026-10-07T21:00:00.000Z' }
const at = (hhmm: string) => new Date(`2026-10-07T${hhmm}:00.000Z`).getTime()

describe('slotState', () => {
  it('follows the clock and the racer connection', () => {
    expect(slotState(slot, false, at('17:00'))).toBe('upcoming')
    expect(slotState(slot, false, at('18:05'))).toBe('starting')
    expect(slotState(slot, true, at('18:05'))).toBe('live')
    expect(slotState(slot, false, at('18:10'))).toBe('late')
    expect(slotState(slot, true, at('18:10'))).toBe('live')
    expect(slotState(slot, false, at('21:00'))).toBe('done')
    expect(slotState(slot, true, at('22:00'))).toBe('done')
  })

  it('takes the grace period as a parameter', () => {
    expect(slotState(slot, false, at('18:05'), 3)).toBe('late')
  })
})

describe('time helpers', () => {
  it('shows a time in the racer zone', () => {
    expect(clock('2026-10-07T18:30:00.000Z', 'UTC')).toBe('18:30')
    expect(clock('2026-10-07T18:30:00.000Z', 'America/Mexico_City')).toBe('12:30')
    expect(clock('nope')).toBe('—')
    expect(clock('2026-10-07T18:30:00.000Z', 'Not/AZone')).toBe('—')
  })

  it('round-trips the datetime-local value', () => {
    const iso = '2026-10-07T18:30:00.000Z'
    expect(localInputToIso(isoToLocalInput(iso))).toBe(iso)
  })
})
