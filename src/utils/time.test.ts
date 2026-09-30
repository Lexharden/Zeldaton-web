import { describe, expect, it } from 'vitest'
import { setLocale } from '@/i18n'
import {
  formatResetTime,
  formatTimezone,
  getZonedParts,
  nextResetUtc,
  zonedTimeToUtc,
} from './time'

describe('timezone helpers', () => {
  it('formats IANA ids as friendly names (Spanish by default)', () => {
    setLocale('es')
    expect(formatTimezone('America/Mexico_City')).toBe('Ciudad de México')
    expect(formatTimezone('America/Argentina/Buenos_Aires')).toBe('Buenos Aires')
    expect(formatTimezone('Asia/Tokyo')).toBe('Tokio')
    // Unknown zones fall back to the city part of the IANA id.
    expect(formatTimezone('Africa/Addis_Ababa')).toBe('Addis Ababa')
  })

  it('formats in English when selected', () => {
    setLocale('en')
    expect(formatTimezone('America/Mexico_City')).toBe('Mexico City')
    expect(formatResetTime('06:00')).toBe('06:00 local')
    setLocale('es')
  })

  it('formats reset time as local wall clock', () => {
    setLocale('es')
    expect(formatResetTime('06:00')).toBe('06:00 hora local')
  })

  it('converts wall-clock time to UTC', () => {
    // Madrid is UTC+1 in January.
    expect(zonedTimeToUtc(2026, 1, 15, 6, 0, 'Europe/Madrid')).toBe(Date.UTC(2026, 0, 15, 5, 0))
    // Tokyo is UTC+9 all year.
    expect(zonedTimeToUtc(2026, 7, 1, 6, 0, 'Asia/Tokyo')).toBe(Date.UTC(2026, 5, 30, 21, 0))
  })

  it('finds the next reset in the racer timezone, not the browser one', () => {
    // 2026-01-15 04:00 in Madrid -> reset the same day at 06:00 Madrid.
    const now = Date.UTC(2026, 0, 15, 3, 0)
    expect(nextResetUtc(now, 'Europe/Madrid', '06:00')).toBe(Date.UTC(2026, 0, 15, 5, 0))
    // Already past 06:00 in Madrid -> next day.
    const later = Date.UTC(2026, 0, 15, 12, 0)
    expect(nextResetUtc(later, 'Europe/Madrid', '06:00')).toBe(Date.UTC(2026, 0, 16, 5, 0))
  })

  it('reads zoned parts', () => {
    const p = getZonedParts(Date.UTC(2026, 0, 15, 12, 30), 'America/New_York')
    expect(p.hour).toBe(7)
    expect(p.minute).toBe(30)
  })
})
