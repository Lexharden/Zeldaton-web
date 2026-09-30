import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { raceApi } from '@/services/api'
import type { EventInfo, EventStatus } from '@/types/event'
import { errorMessage } from '@/utils/errors'

export const useEventStore = defineStore('event', () => {
  const info = ref<EventInfo | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  const status = computed<EventStatus>(() => info.value?.status ?? 'upcoming')
  const isLive = computed(() => status.value === 'live')
  const budgetSeconds = computed(() => info.value?.dailyBudgetSeconds ?? 0)
  const budgetHours = computed(() => Math.round((budgetSeconds.value / 3600) * 10) / 10)
  const startMs = computed(() => (info.value ? Date.parse(info.value.startAtUtc) : 0))

  async function load() {
    loading.value = true
    error.value = null
    try {
      info.value = await raceApi.getEvent()
    } catch (e) {
      error.value = errorMessage(e)
    } finally {
      loading.value = false
    }
  }

  function setStatus(next: EventStatus) {
    if (info.value) info.value.status = next
  }

  return {
    info,
    loading,
    error,
    status,
    isLive,
    budgetSeconds,
    budgetHours,
    startMs,
    load,
    setStatus,
  }
})
