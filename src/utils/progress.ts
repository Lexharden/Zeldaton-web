import type { CatalogObjective } from '@/types/catalog'
import type { GameObjective, GameProgress } from '@/types/game'
import type { Racer } from '@/types/racer'

/** Builds the normalized GameProgress view from racer telemetry and the objectives catalog. */
export function buildGameProgress(
  racer: Racer,
  objectives: CatalogObjective[],
  nameOf: (o: CatalogObjective) => string,
): GameProgress {
  const done = new Set(racer.completedObjectives)
  const list: GameObjective[] = objectives.map((o) => ({
    id: o.id,
    label: nameOf(o),
    age: o.age,
    completed: done.has(o.id),
  }))
  return {
    percentage: racer.progressPercentage,
    currentArea: racer.currentArea,
    currentObjective: racer.currentObjective,
    completedObjectives: racer.completedObjectives,
    objectives: list,
  }
}

export function countItems(racer: Racer, knownIds: string[]): { owned: number; total: number } {
  const items = racer.items ?? {}
  return { owned: knownIds.filter((id) => items[id]).length, total: knownIds.length }
}
