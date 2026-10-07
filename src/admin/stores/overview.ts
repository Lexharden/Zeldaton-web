import { defineStore } from 'pinia'
import { ref } from 'vue'
import { adminApi } from '../api/AdminApi'
import { messageOf } from '../composables/useToasts'
import type { Overview } from '../types'

const INTERVAL_MS = 3000

/** Dashboard data. Polled while a page that needs it is open; paused while the tab is hidden. */
export const useOverviewStore = defineStore('overview', () => {
  const data = ref<Overview | null>(null)
  const error = ref<string | null>(null)
  /** Local time (ms) when `data` arrived, to keep the clocks ticking between polls. */
  const receivedAt = ref(0)
  let timer: ReturnType<typeof setInterval> | null = null
  let inflight = false
  let users = 0

  async function refresh() {
    if (inflight) return
    inflight = true
    try {
      data.value = await adminApi.overview()
      receivedAt.value = Date.now()
      error.value = null
    } catch (e) {
      error.value = messageOf(e)
    } finally {
      inflight = false
    }
  }

  function onVisible() {
    if (!document.hidden) void refresh()
  }

  /** Every page that shows the overview calls start() on mount and stop() on unmount. */
  function start() {
    users += 1
    if (timer) return
    void refresh()
    timer = setInterval(() => {
      if (!document.hidden) void refresh()
    }, INTERVAL_MS)
    document.addEventListener('visibilitychange', onVisible)
  }

  function stop() {
    users = Math.max(0, users - 1)
    if (users > 0 || !timer) return
    clearInterval(timer)
    timer = null
    document.removeEventListener('visibilitychange', onVisible)
  }

  /** Remaining seconds of a racer's day, counting down locally while the clock runs. */
  function remainingNow(status: string, remainingSeconds: number, now: number): number {
    if (status !== 'live') return remainingSeconds
    return Math.max(0, remainingSeconds - Math.floor((now - receivedAt.value) / 1000))
  }

  /** Seconds really played today: grows locally while the game runs, until the next poll. */
  function playedNow(status: string, playedSeconds: number, now: number): number {
    if (status !== 'live') return playedSeconds
    return playedSeconds + Math.max(0, Math.floor((now - receivedAt.value) / 1000))
  }

  return { data, error, receivedAt, refresh, start, stop, remainingNow, playedNow }
})
