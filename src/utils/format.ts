import { intlLocale } from '@/i18n'

const pad = (n: number, len = 2) => String(Math.floor(n)).padStart(len, '0')

export interface DurationParts {
  days: number
  hours: number
  minutes: number
  seconds: number
}

export function splitDuration(totalSeconds: number): DurationParts {
  const s = Math.max(0, Math.floor(totalSeconds))
  return {
    days: Math.floor(s / 86400),
    hours: Math.floor((s % 86400) / 3600),
    minutes: Math.floor((s % 3600) / 60),
    seconds: s % 60,
  }
}

/** HH:MM:SS (hours are not wrapped at 24). */
export function formatDuration(totalSeconds: number): string {
  const s = Math.max(0, Math.floor(totalSeconds))
  return `${pad(s / 3600)}:${pad((s % 3600) / 60)}:${pad(s % 60)}`
}

export function formatPercent(value: number | undefined): string {
  if (value === undefined || Number.isNaN(value)) return '—'
  return `${Math.round(Math.min(100, Math.max(0, value)))}%`
}

export function formatCount(value: number): string {
  return new Intl.NumberFormat(intlLocale.value).format(Math.round(value))
}

/** "45s", "12m" or "2h 10m": a short span for totals (not a clock). */
export function formatSpan(totalSeconds: number): string {
  const s = Math.max(0, Math.round(totalSeconds))
  if (s < 60) return `${s}s`
  if (s < 3600) return `${Math.floor(s / 60)}m`
  const m = Math.floor((s % 3600) / 60)
  return m ? `${Math.floor(s / 3600)}h ${m}m` : `${Math.floor(s / 3600)}h`
}

export const twoDigits = (n: number) => pad(n)
