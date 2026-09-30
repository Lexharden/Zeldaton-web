import { intlLocale, t } from '@/i18n'
import { findZone } from '@/config/timezones'

/** Timezone-aware helpers. Everything takes an IANA id; browser timezone is never assumed. */

const partsCache = new Map<string, Intl.DateTimeFormat>()

function partsFormatter(timezone: string): Intl.DateTimeFormat {
  let fmt = partsCache.get(timezone)
  if (!fmt) {
    fmt = new Intl.DateTimeFormat('en-US', {
      timeZone: timezone,
      hourCycle: 'h23',
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    })
    partsCache.set(timezone, fmt)
  }
  return fmt
}

export interface ZonedParts {
  year: number
  month: number
  day: number
  hour: number
  minute: number
  second: number
}

export function getZonedParts(ms: number, timezone: string): ZonedParts {
  const out: Record<string, number> = {}
  for (const p of partsFormatter(timezone).formatToParts(new Date(ms))) {
    if (p.type !== 'literal') out[p.type] = Number(p.value)
  }
  return {
    year: out.year,
    month: out.month,
    day: out.day,
    hour: out.hour,
    minute: out.minute,
    second: out.second,
  }
}

/** Offset (ms) of a timezone at a given instant. Internal use only; never displayed as an id. */
function zoneOffsetMs(ms: number, timezone: string): number {
  const p = getZonedParts(ms, timezone)
  const asUtc = Date.UTC(p.year, p.month - 1, p.day, p.hour, p.minute, p.second)
  return asUtc - Math.floor(ms / 1000) * 1000
}

/** Converts a wall-clock time in a timezone to a UTC timestamp (ms). */
export function zonedTimeToUtc(
  year: number,
  month: number,
  day: number,
  hour: number,
  minute: number,
  timezone: string,
): number {
  const guess = Date.UTC(year, month - 1, day, hour, minute)
  let ms = guess - zoneOffsetMs(guess, timezone)
  // Second pass handles DST boundaries.
  ms = guess - zoneOffsetMs(ms, timezone)
  return ms
}

export function parseLocalTime(value: string): { hour: number; minute: number } {
  const [h, m] = value.split(':')
  return { hour: Number(h) || 0, minute: Number(m) || 0 }
}

/** Next occurrence (UTC ms) of a wall-clock reset time in the given timezone. */
export function nextResetUtc(nowMs: number, timezone: string, resetLocalTime: string): number {
  const { hour, minute } = parseLocalTime(resetLocalTime)
  const p = getZonedParts(nowMs, timezone)
  let candidate = zonedTimeToUtc(p.year, p.month, p.day, hour, minute, timezone)
  if (candidate <= nowMs) {
    const next = new Date(Date.UTC(p.year, p.month - 1, p.day + 1))
    candidate = zonedTimeToUtc(
      next.getUTCFullYear(),
      next.getUTCMonth() + 1,
      next.getUTCDate(),
      hour,
      minute,
      timezone,
    )
  }
  return candidate
}

/** "America/Argentina/Buenos_Aires" -> "Buenos Aires". */
export function formatTimezone(timezone: string): string {
  const key = `timezones.${timezone}`
  const translated = t(key)
  if (translated !== key) return translated
  const known = findZone(timezone)
  if (known) return known.city
  const city = timezone.split('/').pop() ?? timezone
  return city.replace(/_/g, ' ')
}

export function formatLocalTime(
  value: number | string | Date,
  timezone: string,
  options: { seconds?: boolean } = {},
): string {
  return new Intl.DateTimeFormat(intlLocale.value, {
    timeZone: timezone,
    hour: '2-digit',
    minute: '2-digit',
    second: options.seconds ? '2-digit' : undefined,
    hourCycle: 'h23',
  }).format(new Date(value))
}

export function formatDateTime(value: number | string | Date, timezone: string): string {
  return new Intl.DateTimeFormat(intlLocale.value, {
    timeZone: timezone,
    day: '2-digit',
    month: 'short',
    hour: '2-digit',
    minute: '2-digit',
    hourCycle: 'h23',
  }).format(new Date(value))
}

/** "06:00 local" / "06:00 hora local". The reset is always expressed in the racer's own clock. */
export function formatResetTime(resetLocalTime: string): string {
  const { hour, minute } = parseLocalTime(resetLocalTime)
  return t('daily.resetLocal', {
    time: `${String(hour).padStart(2, '0')}:${String(minute).padStart(2, '0')}`,
  })
}

export function isDaytime(ms: number, timezone: string): boolean {
  const { hour } = getZonedParts(ms, timezone)
  return hour >= 6 && hour < 20
}

/** Bare wall-clock reset time, e.g. "06:00". */
export function formatResetClock(resetLocalTime: string): string {
  const { hour, minute } = parseLocalTime(resetLocalTime)
  return `${String(hour).padStart(2, '0')}:${String(minute).padStart(2, '0')}`
}
