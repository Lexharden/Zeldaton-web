import type { Racer } from '@/types/racer'
import type { StandingEntry } from '@/types/race'

/**
 * Ranking order. The finish criterion is intentionally isolated here: swap this comparator
 * (or drive it from `event.rules`) without touching any component.
 * Finished racers first (earliest finish), then more objectives, then higher progress,
 * then less time used.
 */
export function compareRacers(a: Racer, b: Racer): number {
  const fa = a.status === 'finished' || a.finishedAtUtc !== undefined
  const fb = b.status === 'finished' || b.finishedAtUtc !== undefined
  if (fa !== fb) return fa ? -1 : 1
  if (fa && fb) {
    const ta = a.finalTimeSeconds ?? Number.POSITIVE_INFINITY
    const tb = b.finalTimeSeconds ?? Number.POSITIVE_INFINITY
    if (ta !== tb) return ta - tb
  }
  const oa = a.completedObjectives.length
  const ob = b.completedObjectives.length
  if (oa !== ob) return ob - oa
  if (a.progressPercentage !== b.progressPercentage)
    return b.progressPercentage - a.progressPercentage
  if (a.elapsedSeconds !== b.elapsedSeconds) return a.elapsedSeconds - b.elapsedSeconds
  return a.displayName.localeCompare(b.displayName)
}

export function computeStandings(racers: Racer[]): StandingEntry[] {
  return [...racers]
    .sort(compareRacers)
    .map((racer, index) => ({ racerId: racer.id, rank: index + 1 }))
}
