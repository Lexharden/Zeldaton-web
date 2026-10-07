import { describe, expect, it } from 'vitest'
import { computePlayedMs, computeRemainingMs, computeServerOffsetMs, trackClock } from './clock'
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

describe('played time interpolation', () => {
  const clock = (status: 'live' | 'paused', extra = {}) =>
    trackClock({ ...base, status, playedTodayMs: 60_000, playedTotalMs: 600_000, ...extra }, 1000)

  it('grows only while the game runs', () => {
    expect(computePlayedMs(clock('live'), 'today', 4000)).toBe(63_000)
    expect(computePlayedMs(clock('live'), 'total', 4000)).toBe(603_000)
    expect(computePlayedMs(clock('paused'), 'today', 4000)).toBe(60_000)
  })

  it('never grows past what was left on the clock', () => {
    expect(computePlayedMs(clock('live', { remainingMs: 2000 }), 'today', 61_000)).toBe(62_000)
  })

  it('is zero from an older backend that sends none', () => {
    const old = trackClock({ ...base, status: 'live' }, 1000)
    expect(computePlayedMs(old, 'today', 4000)).toBe(3000) // only the interpolated stretch
    expect(computePlayedMs(trackClock({ ...base, status: 'paused' }, 1000), 'total', 9000)).toBe(0)
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
