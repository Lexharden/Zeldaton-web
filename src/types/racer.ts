export type RacerStatus = 'online' | 'live' | 'paused' | 'offline' | 'exhausted' | 'finished'

export type StreamPlatform = 'twitch' | 'tiktok' | 'youtube'

/** A place where a racer streams. A racer can have several (e.g. Twitch and TikTok) or none. */
export interface RacerChannel {
  platform: StreamPlatform
  handle: string
  url: string
}

/** Live state of the racer's stream, independent of which platform hosts it. */
export interface RacerStream {
  isLive: boolean
  thumbnailUrl?: string
  viewers?: number
}

export interface RacerStats {
  /** The Link being played right now. */
  age?: 'child' | 'adult'
  hearts?: number
  maxHearts?: number
  rupees?: number
  skulltulas?: number
  bossesDefeated?: number
}

export interface Racer {
  id: string
  displayName: string
  slug: string
  avatarUrl?: string
  country?: string
  /** IANA identifier. Never a UTC offset. */
  timezone: string

  status: RacerStatus

  elapsedSeconds: number
  /** Time really played (the game running) today and in total; donations and adjustments do not change it. */
  playedTodaySeconds?: number
  playedSeconds?: number
  remainingSeconds: number

  progressPercentage: number

  currentArea?: string
  currentObjective?: string
  /** Ids of completed objectives, in completion order. */
  completedObjectives: string[]
  finishedAtUtc?: string
  finalTimeSeconds?: number

  /** All channels; may be empty. Twitch is preferred as the primary one. */
  channels: RacerChannel[]
  stream?: RacerStream
  stats?: RacerStats
  items?: Record<string, boolean>
}
