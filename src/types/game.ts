import type { CatalogAge } from './catalog'

export interface GameObjective {
  id: string
  label: string
  /** Which Link reaches it (child temples / adult temples). */
  age: CatalogAge
  completed: boolean
  completedAtUtc?: string
}

export interface GameProgress {
  percentage: number
  currentArea?: string
  currentObjective?: string
  completedObjectives: string[]
  objectives: GameObjective[]
}

/** Static catalog entry; which items exist is UI config, acquisition is racer data. */
export interface ItemDefinition {
  id: string
  short: string
}

export interface ObjectiveDefinition {
  id: string
}
