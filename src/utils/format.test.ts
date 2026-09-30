import { describe, expect, it } from 'vitest'
import { formatDuration, formatPercent, splitDuration } from './format'

describe('formatting', () => {
  it('formats durations as HH:MM:SS', () => {
    expect(formatDuration(9797)).toBe('02:43:17')
    expect(formatDuration(4603)).toBe('01:16:43')
    expect(formatDuration(-5)).toBe('00:00:00')
    expect(formatDuration(4 * 3600)).toBe('04:00:00')
  })

  it('splits countdown parts', () => {
    expect(splitDuration(3 * 86400 + 14 * 3600 + 52 * 60 + 11)).toEqual({
      days: 3,
      hours: 14,
      minutes: 52,
      seconds: 11,
    })
  })

  it('formats and clamps percentages', () => {
    expect(formatPercent(76.4)).toBe('76%')
    expect(formatPercent(140)).toBe('100%')
    expect(formatPercent(undefined)).toBe('—')
  })
})
