import type { ScheduleSlot } from './types'

/** How a slot stands against the clock and what the racer's connection says. */
export type SlotState = 'done' | 'upcoming' | 'starting' | 'live' | 'late'

export const SLOT_LABEL: Record<SlotState, string> = {
  done: 'Terminó',
  upcoming: 'Próximo',
  starting: 'Empezando',
  live: 'En vivo',
  late: 'Retrasado',
}

/** Same default as the server's `noShowMinutes`. */
export const NO_SHOW_MINUTES = 10

/**
 * `racerOnline`: the racer is connected or broadcasting. Inside the slot's hours an absent racer is
 * "starting" for the grace period and "late" afterwards.
 */
export function slotState(
  slot: Pick<ScheduleSlot, 'startUtc' | 'endUtc'>,
  racerOnline: boolean,
  nowMs: number,
  graceMinutes = NO_SHOW_MINUTES,
): SlotState {
  const start = new Date(slot.startUtc).getTime()
  const end = new Date(slot.endUtc).getTime()
  if (nowMs >= end) return 'done'
  if (nowMs < start) return 'upcoming'
  if (racerOnline) return 'live'
  return nowMs - start >= graceMinutes * 60_000 ? 'late' : 'starting'
}

/** "18:30" in `timeZone` (the viewer's zone when omitted). */
export function clock(iso: string, timeZone?: string): string {
  const d = new Date(iso)
  if (Number.isNaN(d.getTime())) return '—'
  try {
    return new Intl.DateTimeFormat('es-MX', {
      hour: '2-digit',
      minute: '2-digit',
      hour12: false,
      timeZone,
    }).format(d)
  } catch {
    return '—' // an unknown time zone name
  }
}

/** "mié 7 oct" in the viewer's zone, used to group slots by day. */
export function dayLabel(iso: string): string {
  return new Intl.DateTimeFormat('es-MX', {
    weekday: 'short',
    day: 'numeric',
    month: 'short',
  }).format(new Date(iso))
}

/** The value of an `<input type="datetime-local">` (viewer's zone) as an ISO instant. */
export function localInputToIso(value: string): string {
  return new Date(value).toISOString()
}

/** An ISO instant as the value of an `<input type="datetime-local">`. */
export function isoToLocalInput(iso: string): string {
  const d = new Date(iso)
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`
}
