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

/** Difference (server - local) so event countdowns can use server time. */
export function computeServerOffsetMs(serverTimeUtc: string, localNowMs: number): number {
  const server = Date.parse(serverTimeUtc)
  return Number.isNaN(server) ? 0 : server - localNowMs
}
