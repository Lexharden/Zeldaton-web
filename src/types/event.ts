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
}
