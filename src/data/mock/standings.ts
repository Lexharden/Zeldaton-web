import type { Racer } from '@/types/racer'
import type { StandingEntry } from '@/types/race'
import { computeStandings } from '@/utils/standings'

/** The simulated ranking uses the same comparator as the real one. */
export function createMockStandings(racers: Racer[]): StandingEntry[] {
  return computeStandings(racers)
}
