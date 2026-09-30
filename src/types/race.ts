import type { RacerStatus, RacerStream } from './racer'

export interface StandingEntry {
  racerId: string
  rank: number
}

/** Authoritative clock snapshot for one racer. The frontend only interpolates it. */
export interface ClockState {
  racerId: string
  serverTimeUtc: string
  remainingMs: number
  status: RacerStatus
  resetAtUtc: string
}

export interface StreamInfo extends RacerStream {
  racerId: string
}

export type ActivityKind = 'area' | 'item' | 'boss' | 'status' | 'reset' | 'finish' | 'system'

export interface ActivityItem {
  id: string
  timestampUtc: string
  kind: ActivityKind
  racerId?: string
  racerName?: string
  /** Sentence for the friendly "Live Activity" list. */
  message: string
  /** Machine code for the terminal feed, e.g. AREA_CHANGED. */
  code: string
  detail?: string
  /** Catalog id (area/item/boss) so each client can translate it. */
  subject?: string
}

export interface FinishInfo {
  racerId: string
  finalTimeSeconds?: number
  finishedAtUtc?: string
}
