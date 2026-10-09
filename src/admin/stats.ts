import type { DayStat } from './types'

/** "2026-10-07" -> "mié 7 oct" (the date is a plain calendar day, no timezone involved). */
export function dayLabel(day: string): string {
  const [y, m, d] = day.split('-').map(Number)
  if (!y || !m || !d) return day
  return new Intl.DateTimeFormat('es-MX', {
    weekday: 'short',
    day: 'numeric',
    month: 'short',
    timeZone: 'UTC',
  }).format(new Date(Date.UTC(y, m - 1, d)))
}

export interface Totals {
  playedSeconds: number
  sessions: number
  objectives: number
  items: number
  bosses: number
  areas: number
  donations: number
  donationAddedSeconds: number
  donationRemovedSeconds: number
  adjustSeconds: number
  exhausted: number
  forcedCloses: number
}

export function totalsOf(rows: DayStat[]): Totals {
  const sum = (pick: (r: DayStat) => number) => rows.reduce((acc, r) => acc + pick(r), 0)
  return {
    playedSeconds: sum((r) => r.playedSeconds),
    sessions: sum((r) => r.sessions),
    objectives: sum((r) => r.objectives),
    items: sum((r) => r.items),
    bosses: sum((r) => r.bosses),
    areas: sum((r) => r.areas),
    donations: sum((r) => r.donations),
    donationAddedSeconds: sum((r) => r.donationAddedSeconds),
    donationRemovedSeconds: sum((r) => r.donationRemovedSeconds),
    adjustSeconds: sum((r) => r.adjustSeconds),
    exhausted: sum((r) => r.exhausted),
    forcedCloses: sum((r) => r.forcedCloses),
  }
}

const COLUMNS: [string, (r: DayStat) => string | number | null][] = [
  ['dia', (r) => r.day],
  ['corredor', (r) => r.racerName || r.racerId],
  ['id', (r) => r.racerId],
  ['jugado_segundos', (r) => r.playedSeconds],
  ['sesiones', (r) => r.sessions],
  ['progreso_inicio', (r) => r.progressStart],
  ['progreso_fin', (r) => r.progressEnd],
  ['objetivos', (r) => r.objectives],
  ['objetos', (r) => r.items],
  ['jefes', (r) => r.bosses],
  ['zonas', (r) => r.areas],
  ['donaciones', (r) => r.donations],
  ['donaciones_topadas', (r) => r.donationCapped],
  ['diamantes', (r) => r.diamonds],
  ['bits', (r) => r.bits],
  ['tiempo_sumado_segundos', (r) => r.donationAddedSeconds],
  ['tiempo_restado_segundos', (r) => r.donationRemovedSeconds],
  ['ajustes_segundos', (r) => r.adjustSeconds],
  ['sin_tiempo', (r) => r.exhausted],
  ['cierres', (r) => r.forcedCloses],
  ['viewers_pico', (r) => r.peakViewers],
  ['incompleto', (r) => (r.partial ? 'si' : 'no')],
]

/** A CSV cell: quoted when it needs to be, and never a spreadsheet formula. */
function cell(value: string | number | null): string {
  if (value === null) return ''
  let text = String(value)
  if (/^[=+\-@\t\r]/.test(text) && typeof value === 'string') text = `'${text}`
  return /[",\n]/.test(text) ? `"${text.replace(/"/g, '""')}"` : text
}

const BOM = String.fromCharCode(0xfeff)

/** Every row as CSV (UTF-8 with BOM so Excel reads the accents). */
export function statsCsv(rows: DayStat[]): string {
  const head = COLUMNS.map(([name]) => name).join(',')
  const lines = rows.map((r) => COLUMNS.map(([, pick]) => cell(pick(r))).join(','))
  return BOM + [head, ...lines].join('\n') + '\n'
}
