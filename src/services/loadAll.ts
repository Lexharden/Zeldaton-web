import { useCatalogStore } from '@/stores/catalog'
import { useDonorsStore } from '@/stores/donors'
import { useEventStore } from '@/stores/event'
import { useHiveShockStore } from '@/stores/hiveshock'
import { useRaceStore } from '@/stores/race'
import { useRacersStore } from '@/stores/racers'

/** Loads every store from REST. Used at start-up and whenever the server says the event changed. */
export async function loadAllData(): Promise<void> {
  await Promise.allSettled([
    useEventStore().load(),
    useRacersStore().load(),
    useHiveShockStore().load(),
    useCatalogStore().load(),
    useDonorsStore().load(),
  ])
  await useRaceStore().load()
}
