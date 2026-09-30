import { watch } from 'vue'
import { getTransport } from '@/services/websocket'
import { dispatchMessage } from '@/services/websocket/dispatcher'
import { useConnectionStore } from '@/stores/connection'
import { useEventStore } from '@/stores/event'
import { useHiveShockStore } from '@/stores/hiveshock'
import { useRaceStore } from '@/stores/race'
import { useRacersStore } from '@/stores/racers'
import { useClockResync } from './useRaceClock'

let started = false
let startTimer: ReturnType<typeof setTimeout> | null = null

/**
 * Bootstraps the data flow once: REST -> stores, then transport -> dispatcher -> stores.
 * Pages read from stores; they never fetch or open sockets themselves.
 */
export function useRaceData() {
  const event = useEventStore()
  const racers = useRacersStore()
  const race = useRaceStore()
  const hive = useHiveShockStore()
  const connection = useConnectionStore()

  async function loadAll() {
    await Promise.allSettled([event.load(), racers.load(), hive.load()])
    await race.load()
  }

  function start() {
    if (started) return
    started = true
    const transport = getTransport()
    transport.subscribe(dispatchMessage)
    transport.onState((status, info) => connection.setStatus(status, info))
    useClockResync().start()
    void loadAll()
    transport.connect()

    // When the countdown ends, ask the backend for the new event status (upcoming -> live).
    watch(
      () => [event.status, event.startMs] as const,
      ([status, startMs]) => {
        if (startTimer) clearTimeout(startTimer)
        if (status !== 'upcoming' || !startMs) return
        const wait = startMs - (Date.now() + connection.serverOffsetMs) + 1500
        // setTimeout overflows past ~24.8 days; skip far-away starts (re-armed on next load).
        if (wait > 0 && wait < 2_000_000_000) startTimer = setTimeout(() => void loadAll(), wait)
      },
      { immediate: true },
    )
  }

  return { start, reload: loadAll }
}
