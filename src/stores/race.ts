import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { raceApi } from '@/services/api'
import type { FinishInfo, StandingEntry, StreamInfo } from '@/types/race'
import { computeStandings } from '@/utils/standings'
import { errorMessage } from '@/utils/errors'
import { useEventStore } from './event'
import { useRacersStore } from './racers'

export const useRaceStore = defineStore('race', () => {
  const racers = useRacersStore()
  const event = useEventStore()

  const serverStandings = ref<StandingEntry[]>([])
  const streams = ref<StreamInfo[]>([])
  const winner = ref<FinishInfo | null>(null)
  const winnerDismissed = ref(false)
  const activeRacerId = ref<string | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  /**
   * Standings are derived from racer progress, not from the clock or donations, so a 1s tick never
   * recomputes them. Server-provided standings only seed the order before racers arrive.
   */
  const standings = computed<StandingEntry[]>(() =>
    racers.list.length
      ? computeStandings(racers.list, event.info?.rules.requiredObjectiveIds)
      : serverStandings.value,
  )
  const rankOf = computed(() => Object.fromEntries(standings.value.map((s) => [s.racerId, s.rank])))
  const leader = computed(() => racers.getById(standings.value[0]?.racerId ?? ''))

  async function load() {
    loading.value = true
    error.value = null
    try {
      const [s, st] = await Promise.all([raceApi.getStandings(), raceApi.getStreams()])
      serverStandings.value = s
      streams.value = st
    } catch (e) {
      error.value = errorMessage(e)
    } finally {
      loading.value = false
    }
  }

  function setStreamLive(racerId: string, isLive: boolean) {
    const s = streams.value.find((x) => x.racerId === racerId)
    if (s) s.isLive = isLive
  }

  function declareWinner(info: FinishInfo) {
    if (!winner.value) {
      winner.value = info
      winnerDismissed.value = false
    }
  }

  function clearWinner() {
    winner.value = null
    winnerDismissed.value = false
  }

  return {
    clearWinner,
    serverStandings,
    streams,
    winner,
    winnerDismissed,
    activeRacerId,
    loading,
    error,
    standings,
    rankOf,
    leader,
    load,
    setStreamLive,
    declareWinner,
  }
})
