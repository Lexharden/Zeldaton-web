import type { Catalog } from '@/types/catalog'
import type { EventInfo } from '@/types/event'
import type { HiveShockStats } from '@/types/hiveshock'
import type { Racer } from '@/types/racer'
import type { ActivityItem, ClockState, StandingEntry, StreamInfo } from '@/types/race'

/** Everything the UI can ask the backend for. Mock and HTTP implement the same contract. */
export interface RaceApi {
  getEvent(): Promise<EventInfo>
  getRacers(): Promise<Racer[]>
  getStandings(): Promise<StandingEntry[]>
  getRacer(id: string): Promise<Racer>
  getStreams(): Promise<StreamInfo[]>
  getActivity(): Promise<ActivityItem[]>
  getHiveShockStats(): Promise<HiveShockStats>
  getClocks(): Promise<ClockState[]>
  getCatalog(): Promise<Catalog>
}

export type ApiErrorKind = 'timeout' | 'http' | 'network' | 'malformed' | 'not-found'

export class ApiError extends Error {
  constructor(
    public kind: ApiErrorKind,
    message: string,
    public status?: number,
  ) {
    super(message)
    this.name = 'ApiError'
  }
}
