import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { DEFAULT_CATALOG } from '@/config/catalog'
import { locale, t } from '@/i18n'
import { raceApi } from '@/services/api'
import type { Catalog, CatalogItem, CatalogObjective } from '@/types/catalog'
import { groupItems, groupObjectives, publicView } from '@/utils/catalog'

/**
 * Items and objectives the race tracks. Starts with the factory catalog so the site renders
 * immediately (and in mock mode / with the backend down), then follows `/api/catalog` and the
 * `CATALOG_UPDATED` message so an organizer's edit shows up without a reload.
 */
export const useCatalogStore = defineStore('catalog', () => {
  const catalog = ref<Catalog>(publicView(DEFAULT_CATALOG))
  const loaded = ref(false)
  let loading: Promise<void> | null = null

  const items = computed(() => catalog.value.items)
  const objectives = computed(() => catalog.value.objectives)
  const version = computed(() => catalog.value.version)
  const itemSections = computed(() => groupItems(items.value))
  const objectiveSections = computed(() => groupObjectives(objectives.value))

  const itemById = computed(() => new Map(items.value.map((i) => [i.id, i])))
  const objectiveById = computed(() => new Map(objectives.value.map((o) => [o.id, o])))

  /** Name in the current language; ids the catalog does not know fall back to the i18n keys, then the id. */
  function itemName(id: string): string {
    const item = itemById.value.get(id)
    if (item) return locale.value === 'es' ? item.nameEs : item.nameEn
    const key = `items.${id}`
    const out = t(key)
    return out === key ? id : out
  }

  function objectiveName(o: CatalogObjective | string): string {
    const entry = typeof o === 'string' ? objectiveById.value.get(o) : o
    if (entry) return locale.value === 'es' ? entry.nameEs : entry.nameEn
    const id = typeof o === 'string' ? o : (o as CatalogObjective).id
    const key = `objectives.${id}`
    const out = t(key)
    return out === key ? id : out
  }

  const label = (item: CatalogItem) => itemName(item.id)

  function set(next: Catalog) {
    catalog.value = publicView(next)
    loaded.value = true
  }

  async function load(): Promise<void> {
    loading ??= (async () => {
      try {
        set(await raceApi.getCatalog())
      } catch {
        // Keep whatever we have (factory catalog or the last good copy).
      } finally {
        loading = null
      }
    })()
    return loading
  }

  /** `CATALOG_UPDATED`: reload only when the server's version differs from ours. */
  async function onUpdated(serverVersion: string): Promise<void> {
    if (serverVersion !== version.value) await load()
  }

  return {
    catalog,
    loaded,
    items,
    objectives,
    version,
    itemSections,
    objectiveSections,
    itemName,
    objectiveName,
    label,
    set,
    load,
    onUpdated,
  }
})
