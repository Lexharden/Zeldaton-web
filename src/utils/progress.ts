import { t } from '@/i18n'
import { OBJECTIVES } from '@/config/event'
import type { GameObjective, GameProgress } from '@/types/game'
import type { Racer } from '@/types/racer'

/** Builds the normalized GameProgress view from racer telemetry. */
export function buildGameProgress(racer: Racer): GameProgress {
  const done = new Set(racer.completedObjectives)
  const objectives: GameObjective[] = OBJECTIVES.map((o) => ({
    id: o.id,
    label: t(`objectives.${o.id}`),
    completed: done.has(o.id),
  }))
  return {
    percentage: racer.progressPercentage,
    currentArea: racer.currentArea,
    currentObjective: racer.currentObjective,
    completedObjectives: racer.completedObjectives,
    objectives,
  }
}

export function countItems(racer: Racer, knownIds: string[]): { owned: number; total: number } {
  const items = racer.items ?? {}
  return { owned: knownIds.filter((id) => items[id]).length, total: knownIds.length }
}
