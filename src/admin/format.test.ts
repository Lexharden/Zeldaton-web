import { describe, expect, it } from 'vitest'
import { actionLabel, ago, hms, isoToLocalInput, localInputToIso, shortDuration } from './format'

describe('admin formatting', () => {
  it('formats clocks and durations', () => {
    expect(hms(0)).toBe('00:00:00')
    expect(hms(14340)).toBe('03:59:00')
    expect(hms(-5)).toBe('00:00:00')
    expect(shortDuration(9000)).toBe('2 h 30 min')
    expect(shortDuration(14400)).toBe('4 h')
    expect(shortDuration(600)).toBe('10 min')
  })

  it('says how long ago something happened', () => {
    expect(ago(null)).toBe('—')
    expect(ago(2)).toBe('ahora')
    expect(ago(30)).toBe('hace 30 s')
    expect(ago(300)).toBe('hace 5 min')
    expect(ago(7300)).toBe('hace 2 h')
  })

  it('round-trips the datetime-local input through UTC', () => {
    const iso = '2026-10-07T12:00:00.000Z'
    const local = isoToLocalInput(iso)
    expect(local).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}$/)
    expect(localInputToIso(local)).toBe(iso)
    expect(isoToLocalInput(undefined)).toBe('')
    expect(localInputToIso('')).toBeUndefined()
    expect(localInputToIso('garbage')).toBeUndefined()
  })

  it('translates audit codes and passes unknown ones through', () => {
    expect(actionLabel('racer.adjust-time')).toBe('Ajustó el tiempo de un corredor')
    expect(actionLabel('something.new')).toBe('something.new')
  })
})
