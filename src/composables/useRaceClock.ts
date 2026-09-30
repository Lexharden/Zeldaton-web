import { computed, toValue, type MaybeRefOrGetter } from 'vue'
import { runtime } from '@/config/runtime'
import { getTransport } from '@/services/websocket'
import { useEventStore } from '@/stores/event'
import { useRacersStore } from '@/stores/racers'
import { computeRemainingMs } from '@/utils/clock'
import { useNow } from './useNow'

/**
 * Per-racer clock view. The backend snapshot (`remainingMs`, `status`, `resetAtUtc`,
 * `serverTimeUtc`) is the truth; this only interpolates it between snapshots so the
 * display is smooth. Every CLOCK_SNAPSHOT / CLOCK_SYNC overrides the local value.
 */
export function useRaceClock(racerId: MaybeRefOrGetter<string>) {
  const racers = useRacersStore()
  const event = useEventStore()
  const { monotonic } = useNow()

  const clock = computed(() => racers.clocks[toValue(racerId)])
  const status = computed(() => clock.value?.status ?? 'offline')
  const resetAtUtc = computed(() => clock.value?.resetAtUtc ?? '')
  const serverTimeUtc = computed(() => clock.value?.serverTimeUtc ?? '')

  const remainingMs = computed(() =>
    clock.value ? computeRemainingMs(clock.value, monotonic.value) : 0,
  )
  const remainingSeconds = computed(() => Math.ceil(remainingMs.value / 1000))
  const usedSeconds = computed(() =>
    Math.max(0, event.budgetSeconds - Math.ceil(remainingMs.value / 1000)),
  )
  const isExpired = computed(
    () =>
      status.value === 'exhausted' ||
      (!!clock.value && remainingMs.value <= 0 && status.value === 'live'),
  )
  const usedRatio = computed(() =>
    event.budgetSeconds ? Math.min(1, usedSeconds.value / event.budgetSeconds) : 0,
  )

  return {
    status,
    resetAtUtc,
    serverTimeUtc,
    remainingMs,
    remainingSeconds,
    usedSeconds,
    usedRatio,
    isExpired,
  }
}

/** Periodically asks the backend for a fresh snapshot. The client is never the authority. */
export function useClockResync() {
  let timer: ReturnType<typeof setInterval> | null = null
  return {
    start() {
      if (timer) return
      timer = setInterval(
        () => getTransport().send({ type: 'CLOCK_SYNC_REQUEST' }),
        runtime.clockResyncMs,
      )
    },
    stop() {
      if (timer) clearInterval(timer)
      timer = null
    },
  }
}
