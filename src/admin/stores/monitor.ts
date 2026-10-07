import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { adminApi } from '../api/AdminApi'
import { messageOf } from '../composables/useToasts'
import type { Incident, IncidentStatus, MonitorData } from '../types'

const INTERVAL_MS = 5000
/** A hidden tab still checks for incidents, only less often: a referee may keep it in the background. */
const HIDDEN_EVERY = 6

/** Incidents and the "since your last visit" summary. Polled while the panel is open. */
export const useMonitorStore = defineStore('monitor', () => {
  const data = ref<MonitorData | null>(null)
  const error = ref<string | null>(null)
  let timer: ReturnType<typeof setInterval> | null = null
  let inflight = false
  let ticks = 0
  let users = 0

  const openCount = computed(() => data.value?.openCount ?? 0)

  async function refresh() {
    if (inflight) return
    inflight = true
    try {
      data.value = await adminApi.monitor()
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

  /** The layout and the monitor page both call start() on mount and stop() on unmount. */
  function start() {
    users += 1
    if (timer) return
    void refresh()
    timer = setInterval(() => {
      ticks += 1
      if (!document.hidden || ticks % HIDDEN_EVERY === 0) void refresh()
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

  async function review(incident: Incident, status: IncidentStatus, note?: string) {
    await adminApi.reviewIncident(incident.id, status, note)
    await refresh()
  }

  /** The referee read the summary: the next one starts from now. */
  async function markSeen() {
    await adminApi.monitorSeen()
    await refresh()
  }

  async function addNote(text: string, racerId?: string) {
    await adminApi.addNote(text, racerId)
    await refresh()
  }
  async function deleteNote(id: number) {
    await adminApi.deleteNote(id)
    await refresh()
  }

  return {
    data,
    error,
    openCount,
    refresh,
    start,
    stop,
    review,
    markSeen,
    addNote,
    deleteNote,
  }
})
