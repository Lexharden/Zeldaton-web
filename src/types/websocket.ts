import type { ActivityItem, ClockState } from './race'
import type { GameProgress } from './game'
import type { HiveShockStats } from './hiveshock'
import type { RacerStats, RacerStatus, RacerStream } from './racer'

export type ConnectionStatus =
  'connecting' | 'connected' | 'disconnected' | 'reconnecting' | 'error'

export type ConnectionQuality = 'good' | 'degraded' | 'poor' | 'unknown'

interface RacerScoped {
  racerId: string
}

export type WsMessage =
  | { type: 'CLOCK_SNAPSHOT'; clocks: ClockState[] }
  | { type: 'CLOCK_SYNC'; clock: ClockState }
  | ({ type: 'DAILY_RESET'; clock: ClockState } & RacerScoped)
  | ({ type: 'SESSION_STARTED'; serverTimeUtc?: string } & RacerScoped)
  | ({ type: 'SESSION_PAUSED'; serverTimeUtc?: string } & RacerScoped)
  | ({ type: 'SESSION_RESUMED'; serverTimeUtc?: string } & RacerScoped)
  | ({ type: 'SESSION_EXHAUSTED'; serverTimeUtc?: string } & RacerScoped)
  | ({ type: 'GAME_FORCE_CLOSE'; serverTimeUtc?: string } & RacerScoped)
  | ({ type: 'GAME_PROGRESS'; progress: Partial<GameProgress> } & RacerScoped)
  | ({ type: 'ITEM_ACQUIRED'; item: string } & RacerScoped)
  | ({ type: 'AREA_CHANGED'; area: string } & RacerScoped)
  | ({ type: 'BOSS_DEFEATED'; boss: string; bossesDefeated?: number } & RacerScoped)
  | ({ type: 'STATS_UPDATED'; stats: RacerStats } & RacerScoped)
  | ({ type: 'GAME_FINISHED'; finalTimeSeconds?: number; finishedAtUtc?: string } & RacerScoped)
  | ({ type: 'RACER_STATUS_CHANGED'; status: RacerStatus } & RacerScoped)
  | ({ type: 'STREAM_UPDATED'; stream: RacerStream } & RacerScoped)
  | { type: 'LIVE_ACTIVITY'; activity: ActivityItem }
  | { type: 'CATALOG_UPDATED'; version: string }
  | { type: 'HIVESHOCK_STATS_UPDATED'; stats: Partial<HiveShockStats> }

export type WsMessageType = WsMessage['type']

/** Messages the client may send to the backend. */
export type WsClientMessage = { type: 'CLOCK_SYNC_REQUEST' } | { type: 'PING'; sentAt: number }

export interface TransportInfo {
  reconnectAttempts: number
  maxAttempts: number
  nextRetryInMs?: number
}
