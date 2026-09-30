import { computed, toValue, type MaybeRefOrGetter } from 'vue'
import { useConnectionStore } from '@/stores/connection'
import { splitDuration } from '@/utils/format'
import { useNow } from './useNow'

/**
 * Countdown (or count-up) to a UTC instant, following server time when known.
 * `mode: 'down'` clamps at zero; `'up'` measures time elapsed since the instant.
 */
export function useCountdown(target: MaybeRefOrGetter<number>, mode: 'down' | 'up' = 'down') {
  const { now } = useNow()
  const connection = useConnectionStore()

  const totalSeconds = computed(() => {
    const serverNow = now.value + connection.serverOffsetMs
    const diff = mode === 'down' ? toValue(target) - serverNow : serverNow - toValue(target)
    return Math.max(0, Math.floor(diff / 1000))
  })
  const parts = computed(() => splitDuration(totalSeconds.value))
  const done = computed(() => mode === 'down' && totalSeconds.value <= 0)

  return { totalSeconds, parts, done }
}
