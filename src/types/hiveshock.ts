export interface HiveShockStats {
  connectedRacers: number
  gameEvents: number
  itemEvents: number
  progressEvents: number
  chatEvents: number
}

export type CapabilityState = 'live' | 'experimental' | 'planned'

export interface HiveShockCapability {
  id: string
  state: CapabilityState
}
