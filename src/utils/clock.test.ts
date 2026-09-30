import { describe, expect, it } from 'vitest'
import { computeRemainingMs, computeServerOffsetMs, trackClock } from './clock'
import { setLocale } from '@/i18n'
import { eventHeadline, trackPosition } from './status'

const base = {
  racerId: 'a',
  serverTimeUtc: '2026-01-01T00:00:00Z',
  remainingMs: 10_000,
  resetAtUtc: '',
}

describe('clock interpolation', () => {
  it('counts down only while live', () => {
    expect(computeRemainingMs(trackClock({ ...base, status: 'live' }, 1000), 4000)).toBe(7000)
    expect(computeRemainingMs(trackClock({ ...base, status: 'paused' }, 1000), 4000)).toBe(10_000)
    expect(
      computeRemainingMs(trackClock({ ...base, status: 'exhausted', remainingMs: 0 }, 1000), 4000),
    ).toBe(0)
  })

  it('never goes below zero', () => {
    expect(computeRemainingMs(trackClock({ ...base, status: 'live' }, 0), 60_000)).toBe(0)
  })

  it('computes server offset', () => {
    expect(computeServerOffsetMs('2026-01-01T00:00:05Z', Date.parse('2026-01-01T00:00:00Z'))).toBe(
      5000,
    )
  })
})

describe('event state mapping', () => {
  it('maps status to hero copy in both languages', () => {
    setLocale('en')
    expect(eventHeadline('upcoming').eyebrow).toBe('EVENT STARTS IN')
    expect(eventHeadline('live')).toEqual({ eyebrow: 'LIVE NOW', cta: 'WATCH LIVE' })
    expect(eventHeadline('finished').eyebrow).toBe('EVENT COMPLETE')
    setLocale('es')
    expect(eventHeadline('upcoming').eyebrow).toBe('EL EVENTO EMPIEZA EN')
    expect(eventHeadline('live').eyebrow).toBe('EN VIVO AHORA')
  })

  it('positions racers by objectives, falling back to percentage', () => {
    expect(trackPosition(5, 10, 99)).toBe(0.5)
    expect(trackPosition(0, 10, 30)).toBe(0.3)
  })
})
