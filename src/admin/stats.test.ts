import { describe, expect, it } from 'vitest'
import { dayLabel, statsCsv, totalsOf } from './stats'
import type { DayStat } from './types'

const row = (over: Partial<DayStat> = {}): DayStat => ({
  racerId: 'ralbat',
  racerName: 'Ralbat',
  day: '2026-10-07',
  playedSeconds: 3600,
  sessions: 1,
  objectives: 2,
  items: 5,
  bosses: 1,
  areas: 4,
  donations: 3,
  donationAddedSeconds: 60,
  donationRemovedSeconds: 30,
  donationCapped: 0,
  diamonds: 10,
  bits: 0,
  adjustSeconds: -120,
  exhausted: 0,
  forcedCloses: 1,
  progressStart: 10,
  progressEnd: 25,
  peakViewers: 100,
  partial: false,
  ...over,
})

describe('day statistics helpers', () => {
  it('names a day by its calendar date without timezone drift', () => {
    // Whatever the machine's timezone, 2026-10-07 is the 7th (a Wednesday).
    expect(dayLabel('2026-10-07')).toMatch(/7/)
    expect(dayLabel('2026-10-07').toLowerCase()).toMatch(/mi/)
    expect(dayLabel('nonsense')).toBe('nonsense')
  })

  it('adds the figures up', () => {
    const t = totalsOf([
      row(),
      row({ racerId: 'xime', playedSeconds: 1800, items: 1, adjustSeconds: 60 }),
    ])
    expect(t).toMatchObject({ playedSeconds: 5400, items: 6, adjustSeconds: -60, forcedCloses: 2 })
    expect(totalsOf([]).playedSeconds).toBe(0)
  })

  it('exports a CSV that Excel reads and that cannot run a formula', () => {
    const csv = statsCsv([
      row({ racerName: 'Ral,bat "El" Rápido' }),
      row({
        racerId: 'x',
        racerName: '=HYPERLINK("http://evil")',
        peakViewers: null,
        partial: true,
      }),
    ])
    const lines = csv
      .replace(/^\uFEFF/, '')
      .trim()
      .split('\n')
    expect(csv.charCodeAt(0)).toBe(0xfeff)
    expect(lines[0].startsWith('dia,corredor,id,jugado_segundos')).toBe(true)
    expect(lines).toHaveLength(3)
    expect(lines[1]).toContain('"Ral,bat ""El"" Rápido"')
    expect(lines[2]).toContain("'=HYPERLINK")
    expect(lines[2].endsWith(',si')).toBe(true) // partial
    expect(lines[2]).toContain(',,') // a missing figure is an empty cell
  })
})
