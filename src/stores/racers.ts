import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { raceApi } from '@/services/api'
import type { Racer } from '@/types/racer'
import type { ClockState } from '@/types/race'
import { trackClock, type TrackedClock } from '@/utils/clock'
import { errorMessage } from '@/utils/errors'

/** Normalized racer state. Updates touch only the racer they belong to. */
export const useRacersStore = defineStore('racers', () => {
  const byId = ref<Record<string, Racer>>({})
  const order = ref<string[]>([])
  const clocks = ref<Record<string, TrackedClock>>({})
  const loading = ref(false)
  const loaded = ref(false)
  const error = ref<string | null>(null)

  const list = computed(() => order.value.map((id) => byId.value[id]).filter(Boolean))
  const getById = (id: string): Racer | undefined => byId.value[id]

  function setRacers(racers: Racer[]) {
    byId.value = Object.fromEntries(racers.map((r) => [r.id, r]))
    order.value = racers.map((r) => r.id)
    for (const r of racers) {
      // Seed clocks from REST only if no live snapshot exists yet.
      if (!clocks.value[r.id]) {
        clocks.value[r.id] = trackClock(
          {
            racerId: r.id,
            serverTimeUtc: new Date().toISOString(),
            remainingMs: r.remainingSeconds * 1000,
            status: r.status,
            resetAtUtc: '',
          },
          performance.now(),
        )
      }
    }
  }

  async function load() {
    loading.value = true
    error.value = null
    try {
      setRacers(await raceApi.getRacers())
      loaded.value = true
    } catch (e) {
      error.value = errorMessage(e)
    } finally {
      loading.value = false
    }
  }

  /** Loads (or refreshes) a single racer, used by the detail page on cold start. */
  async function loadOne(id: string): Promise<Racer | undefined> {
    if (byId.value[id]) return byId.value[id]
    try {
      const racer = await raceApi.getRacer(id)
      byId.value[id] = racer
      if (!order.value.includes(id)) order.value.push(id)
      return racer
    } catch {
      return undefined
    }
  }

  function patch(id: string, changes: Partial<Racer>) {
    const racer = byId.value[id]
    if (racer) Object.assign(racer, changes)
  }

  function setClock(clock: ClockState) {
    clocks.value[clock.racerId] = trackClock(clock, performance.now())
    const racer = byId.value[clock.racerId]
    if (racer) {
      racer.status = clock.status
      racer.remainingSeconds = Math.round(clock.remainingMs / 1000)
    }
  }

  return {
    byId,
    order,
    clocks,
    loading,
    loaded,
    error,
    list,
    getById,
    setRacers,
    load,
    loadOne,
    patch,
    setClock,
  }
})
