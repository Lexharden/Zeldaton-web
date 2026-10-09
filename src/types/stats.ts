/** One game day of a racer, as the public site shows it (no organizer internals). */
export interface PublicDay {
  /** The local date the game day began on, e.g. "2026-10-07". */
  day: string
  playedSeconds: number
  sessions: number
  objectives: number
  items: number
  bosses: number
  areas: number
  donations: number
  /** Time that donations added / removed that day. */
  donationAddedSeconds: number
  donationRemovedSeconds: number
  /** Times the racer ran out of time. */
  exhausted: number
  progressStart: number | null
  progressEnd: number | null
  peakViewers: number | null
  /** The day began before the statistics existed: some figures are incomplete. */
  partial: boolean
}
