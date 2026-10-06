import type { StreamPlatform } from './racer'

/** One donor as the public board shows them: ranked by the time their donations moved. */
export interface PublicDonor {
  rank: number
  /** The name TikTok / Twitch reports for the viewer. */
  viewer: string
  platform: StreamPlatform
  currency: 'diamonds' | 'bits'
  donations: number
  /** Total diamonds or bits paid. */
  amount: number
  addedSeconds: number
  removedSeconds: number
}

export interface DonorsBoard {
  /** The organizer can switch the whole board off. */
  enabled: boolean
  /** Event-wide totals (everyone, hidden donors included: it is an aggregate). */
  totals: { donations: number; addedSeconds: number; removedSeconds: number }
  donors: PublicDonor[]
}
