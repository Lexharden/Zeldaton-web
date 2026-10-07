import type { ClockState } from '@/types/race'

/** A clock snapshot plus the local monotonic moment it was received. */
export interface TrackedClock extends ClockState {
  receivedAtMs: number
}

export function trackClock(clock: ClockState, receivedAtMs: number): TrackedClock {
  return { ...clock, receivedAtMs }
}

/**
 * Interpolates remaining time from a server snapshot. Only a running ('live') clock
 * counts down; the backend remains the authority and every new snapshot overrides this.
 */
export function computeRemainingMs(clock: TrackedClock, nowMs: number): number {
  if (clock.status !== 'live') return Math.max(0, clock.remainingMs)
  return Math.max(0, clock.remainingMs - Math.max(0, nowMs - clock.receivedAtMs))
}

/**
 * Time really played, interpolated like `computeRemainingMs`: it only grows while the clock runs
 * (and never past what was left on it). Older backends send no value: 0.
 */
export function computePlayedMs(
  clock: TrackedClock,
  which: 'today' | 'total',
  nowMs: number,
): number {
  const base = Math.max(0, (which === 'today' ? clock.playedTodayMs : clock.playedTotalMs) ?? 0)
  if (clock.status !== 'live') return base
  const running = Math.max(0, nowMs - clock.receivedAtMs)
  return base + Math.min(running, Math.max(0, clock.remainingMs))
}

/** Difference (server - local) so event countdowns can use server time. */
export function computeServerOffsetMs(serverTimeUtc: string, localNowMs: number): number {
  const server = Date.parse(serverTimeUtc)
  return Number.isNaN(server) ? 0 : server - localNowMs
}
