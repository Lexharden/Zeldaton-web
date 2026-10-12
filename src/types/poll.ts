export type PollMode = 'auto' | 'open' | 'closed'

export interface PollOption {
  racerId: string
  name: string
  avatarUrl?: string
}

export interface PollResult {
  racerId: string
  votes: number
  /** One decimal. */
  percent: number
}

/**
 * The "who will win?" poll as one connection sees it. The results only arrive once that connection
 * has voted or the poll is closed, so nobody votes with the crowd.
 */
export interface Poll {
  open: boolean
  mode: PollMode
  options: PollOption[]
  /** The racer this connection voted for (the server knows it by address, not by browser). */
  voted: string | null
  totalVotes?: number
  results?: PollResult[]
}

/** One row of the organizers' list of who voted. */
export interface PollVoter {
  id: number
  /** Missing for votes cast before names were asked. */
  name: string | null
  racerId: string
  at: string
}

export interface AdminPoll extends Poll {
  voters: PollVoter[]
}

export const NAME_MIN = 2
export const NAME_MAX = 40
