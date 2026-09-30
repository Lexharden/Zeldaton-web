import { computed, toValue, type MaybeRefOrGetter } from 'vue'
import { ITEMS, OBJECTIVES } from '@/config/event'
import { useRaceStore } from '@/stores/race'
import { useRacersStore } from '@/stores/racers'
import { buildGameProgress, countItems } from '@/utils/progress'
import { trackPosition } from '@/utils/status'

/** Derived, display-ready view of one racer. */
export function useRacer(id: MaybeRefOrGetter<string>) {
  const racers = useRacersStore()
  const race = useRaceStore()

  const racer = computed(() => racers.getById(toValue(id)))
  const rank = computed(() => race.rankOf[toValue(id)])
  const progress = computed(() => (racer.value ? buildGameProgress(racer.value) : undefined))
  const items = computed(() =>
    racer.value
      ? countItems(
          racer.value,
          ITEMS.map((i) => i.id),
        )
      : { owned: 0, total: ITEMS.length },
  )
  const trackFraction = computed(() =>
    racer.value
      ? trackPosition(
          racer.value.completedObjectives.length,
          OBJECTIVES.length,
          racer.value.progressPercentage,
        )
      : 0,
  )

  return { racer, rank, progress, items, trackFraction }
}
