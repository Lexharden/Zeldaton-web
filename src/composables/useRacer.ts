import { computed, toValue, type MaybeRefOrGetter } from 'vue'
import { useCatalogStore } from '@/stores/catalog'
import { useRaceStore } from '@/stores/race'
import { useRacersStore } from '@/stores/racers'
import { buildGameProgress, countItems } from '@/utils/progress'
import { trackPosition } from '@/utils/status'

/** Derived, display-ready view of one racer. */
export function useRacer(id: MaybeRefOrGetter<string>) {
  const racers = useRacersStore()
  const race = useRaceStore()
  const catalog = useCatalogStore()

  const racer = computed(() => racers.getById(toValue(id)))
  const rank = computed(() => race.rankOf[toValue(id)])
  const progress = computed(() =>
    racer.value
      ? buildGameProgress(racer.value, catalog.objectives, (o) => catalog.objectiveName(o))
      : undefined,
  )
  const items = computed(() =>
    racer.value
      ? countItems(
          racer.value,
          catalog.items.map((i) => i.id),
        )
      : { owned: 0, total: catalog.items.length },
  )
  const trackFraction = computed(() =>
    racer.value
      ? trackPosition(
          racer.value.completedObjectives.length,
          catalog.objectives.length,
          racer.value.progressPercentage,
        )
      : 0,
  )

  return { racer, rank, progress, items, trackFraction }
}
