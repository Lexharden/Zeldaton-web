import type { Catalog, CatalogItem, CatalogObjective } from '@/types/catalog'
import type { DonationTimePolicy, EventInfo } from '@/types/event'
import type { HiveShockStats } from '@/types/hiveshock'
import type { ActivityItem } from '@/types/race'
import type { Racer } from '@/types/racer'

export type AdminRole = 'admin' | 'moderator'

export interface AdminUser {
  /** Null for the emergency ADMIN_TOKEN, which is not an account. */
  id: number | null
  username: string
  role: AdminRole
  disabled?: boolean
  createdAt?: string
  lastLoginAt?: string | null
}

export interface MeResponse {
  user: AdminUser
  via: 'session' | 'token'
  csrfToken: string | null
}

export interface LoginResponse {
  user: AdminUser
  csrfToken: string
  expiresAtUtc: string
}

export interface OverviewRacer {
  racer: Racer
  connected: boolean
  lastHeartbeatUtc: string | null
  heartbeatAgeSeconds: number | null
}

export interface OverviewAlert {
  level: 'info' | 'warn' | 'error'
  code: string
  racerId: string | null
  racerName: string | null
  message: string
}

export interface Overview {
  serverTimeUtc: string
  event: EventInfo
  summary: {
    racers: number
    connected: number
    live: number
    paused: number
    online: number
    offline: number
    exhausted: number
    finished: number
  }
  racers: OverviewRacer[]
  alerts: OverviewAlert[]
  hiveshock: HiveShockStats
  activity: ActivityItem[]
  winner: { racerId: string; finalTimeSeconds?: number; finishedAtUtc: string } | null
  catalogVersion: string
}

export interface AuditRow {
  id: number
  ts: string
  actor: string
  action: string
  racerId: string | null
  payload: Record<string, unknown>
}

export interface ChannelInput {
  platform: 'twitch' | 'tiktok' | 'youtube'
  handle: string
}

export interface NewRacerInput {
  id: string
  displayName: string
  timezone: string
  country?: string
  avatarUrl?: string
  channels: ChannelInput[]
}

export type RacerPatchInput = Partial<Omit<NewRacerInput, 'id'>>

export interface EventPatchInput {
  name?: string
  startAtUtc?: string
  endAtUtc?: string
  dailyBudgetSeconds?: number
  dailyResetLocalTime?: string
  winCondition?: string
  requiredObjectiveIds?: string[]
  donationTime?: DonationTimePolicy
  rehearsal?: boolean
}

/** One time donation as HiveShock reported it and what the clock really got. */
export interface DonationRow {
  id: number
  ts: string
  racerId: string
  platform: 'tiktok' | 'twitch'
  currency: 'diamonds' | 'bits'
  amount: number
  gift: string | null
  giftCount: number | null
  viewer: string | null
  requestedSeconds: number
  appliedSeconds: number
  /** Why less than asked was applied: per_donation, daily_limit, clock_max, clock_zero. */
  limitedBy: string | null
}

export interface DonationTotals {
  racerId: string
  donations: number
  addedSeconds: number
  removedSeconds: number
}

/** State of the Discord "X is live" announcements. The webhook URL is never sent to the browser. */
export interface DiscordStatus {
  /** DISCORD_WEBHOOK_URL is set on the server. */
  configured: boolean
  /** The organizer's switch. */
  enabled: boolean
  /** Rehearsal mode: nothing is announced while it is on. */
  rehearsal: boolean
  lastSentAt: string | null
  lastError: string | null
}

/** A viewer ranked by the time their donations moved. */
export interface TopDonor {
  viewer: string
  platform: 'tiktok' | 'twitch'
  currency: 'diamonds' | 'bits'
  donations: number
  /** Total diamonds or bits paid. */
  amount: number
  addedSeconds: number
  removedSeconds: number
  /** How many different racers they donated to. */
  racers: number
  lastAt: string
  /** Hidden from the public board (still counted and listed here). */
  hidden: boolean
}

export interface DonationsResponse {
  /** Whether the public site shows the donors board. */
  donorsPublic: boolean
  policy: DonationTimePolicy
  /** What counts against today's limits, per racer. */
  today: { racerId: string; addedSeconds: number; removedSeconds: number }[]
  /** Whole event. */
  totals: DonationTotals[]
  /** Top donors of the whole event, by time moved. */
  donors: TopDonor[]
  recent: DonationRow[]
}

export type EventAction = 'start' | 'pause' | 'resume' | 'finish'
export type RacerAction =
  'pause' | 'resume' | 'force-close' | 'reset-day' | 'adjust-time' | 'finish'

export interface RacerActionBody {
  deltaSeconds?: number
  finalTimeSeconds?: number
  reason?: string
}

export interface AdminCatalog extends Catalog {
  items: CatalogItem[]
  objectives: CatalogObjective[]
}

export interface UserRow extends AdminUser {
  id: number
  disabled: boolean
  createdAt: string
}
