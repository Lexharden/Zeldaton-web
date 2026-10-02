export type EventStatus = 'upcoming' | 'live' | 'paused' | 'finished'

/** Configurable rules. The UI never decides who wins; it reads this + backend payloads. */
export interface EventRules {
  /** Human readable description of the finish line. */
  winCondition: string
  /** Objective ids that must be completed to finish the race. */
  requiredObjectiveIds: string[]
}

export interface EventInfo {
  id: string
  name: string
  game: string
  edition: string
  status: EventStatus
  startAtUtc: string
  endAtUtc?: string
  /** IANA timezone used whenever an event-level time is displayed. */
  timezone: string
  dailyBudgetSeconds: number
  /** Wall clock time (HH:MM) at which each racer's budget resets, in the racer's own timezone. */
  dailyResetLocalTime: string
  rules: EventRules
  /** Organizer limits for time from viewer donations (absent on older backends). */
  donationTime?: DonationTimePolicy
  /** Test run: the data is not the real event's and will be wiped (absent on older backends). */
  rehearsal?: boolean
}

/**
 * How much time TikTok gifts / Twitch bits may add or remove. The streamer sets the rate in
 * HiveShock; the backend applies it only within these limits (all values in seconds).
 */
export interface DonationTimePolicy {
  enabled: boolean
  allowAdd: boolean
  allowRemove: boolean
  maxSecondsPerDonation: number
  /** Per racer and day; resets with the daily budget. */
  maxAddedSecondsPerDay: number
  maxRemovedSecondsPerDay: number
}
