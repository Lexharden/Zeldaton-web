import type { Racer } from '@/types/racer'
import type { StandingEntry } from '@/types/race'

/**
 * Ranking order. The backend (`backend/src/standings.rs`) sorts the same way and
 * `contract/standings-cases.json` is read by both test suites, so they cannot drift apart. The website
 * sorts its own table from the racers it holds.
 *
 * Finished racers first, in the order they crossed the line (the first is the winner), then by less
 * time really played. Racing: more *required* objectives, higher progress (in tenths of a percent),
 * more items, reached that number of objectives earlier, less time really played, id.
 * Nothing here depends on donations or time adjustments: they move the clock, not the race.
 */

/** Objectives completed that the event requires to win. Without the list (not loaded yet) all count. */
export function requiredDone(racer: Racer, required?: readonly string[]): number {
  if (!required) return racer.completedObjectives.length
  return racer.completedObjectives.filter((id) => required.includes(id)).length
}

/** Progress in tenths of a percent: an integer, so the order is total (no NaN, no float noise). */
export function progressTenths(pct: number): number {
  return Number.isFinite(pct) ? Math.round(Math.min(100, Math.max(0, pct)) * 10) : 0
}

export function itemsOwned(racer: Racer): number {
  return Object.values(racer.items ?? {}).filter(Boolean).length
}

const isFinished = (r: Racer) => r.status === 'finished' || r.finishedAtUtc !== undefined

/** A timestamp as milliseconds, or `null` when absent or unreadable. */
const ms = (iso?: string): number | null => {
  if (!iso) return null
  const t = Date.parse(iso)
  return Number.isNaN(t) ? null : t
}

/** Smaller first; an absent value goes first or last as `absent` says. */
function byOptional(a: number | null, b: number | null, absent: 'first' | 'last'): number {
  if (a === b) return 0
  if (a === null) return absent === 'first' ? -1 : 1
  if (b === null) return absent === 'first' ? 1 : -1
  return a - b
}

export function compareRacers(a: Racer, b: Racer, required?: readonly string[]): number {
  const fa = isFinished(a)
  const fb = isFinished(b)
  if (fa !== fb) return fa ? -1 : 1
  if (fa && fb) {
    const crossed = byOptional(ms(a.finishedAtUtc), ms(b.finishedAtUtc), 'first')
    if (crossed !== 0) return crossed
    const ta = a.finalTimeSeconds ?? Number.MAX_SAFE_INTEGER
    const tb = b.finalTimeSeconds ?? Number.MAX_SAFE_INTEGER
    if (ta !== tb) return ta - tb
  } else {
    const oa = requiredDone(a, required)
    const ob = requiredDone(b, required)
    if (oa !== ob) return ob - oa
    const pa = progressTenths(a.progressPercentage)
    const pb = progressTenths(b.progressPercentage)
    if (pa !== pb) return pb - pa
    const ia = itemsOwned(a)
    const ib = itemsOwned(b)
    if (ia !== ib) return ib - ia
    const reached = byOptional(ms(a.milestoneAtUtc), ms(b.milestoneAtUtc), 'last')
    if (reached !== 0) return reached
    const played = (a.playedSeconds ?? 0) - (b.playedSeconds ?? 0)
    if (played !== 0) return played
  }
  return a.id < b.id ? -1 : a.id > b.id ? 1 : 0
}

export function computeStandings(racers: Racer[], required?: readonly string[]): StandingEntry[] {
  return [...racers]
    .sort((a, b) => compareRacers(a, b, required))
    .map((racer, index) => ({ racerId: racer.id, rank: index + 1 }))
}
