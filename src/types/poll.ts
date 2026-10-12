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
