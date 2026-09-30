import { defineStore } from 'pinia'
import { ref } from 'vue'
import { raceApi } from '@/services/api'
import type { HiveShockStats } from '@/types/hiveshock'
import type { ActivityItem } from '@/types/race'
import { errorMessage } from '@/utils/errors'

const MAX_ACTIVITY = 60

export const useHiveShockStore = defineStore('hiveshock', () => {
  const stats = ref<HiveShockStats | null>(null)
  const activity = ref<ActivityItem[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function load() {
    loading.value = true
    error.value = null
    try {
      const [s, a] = await Promise.all([raceApi.getHiveShockStats(), raceApi.getActivity()])
      stats.value = s
      activity.value = a.slice(0, MAX_ACTIVITY)
    } catch (e) {
      error.value = errorMessage(e)
    } finally {
      loading.value = false
    }
  }

  function updateStats(next: Partial<HiveShockStats>) {
    stats.value = {
      connectedRacers: 0,
      gameEvents: 0,
      itemEvents: 0,
      progressEvents: 0,
      chatEvents: 0,
      ...stats.value,
      ...next,
    }
  }

  function pushActivity(item: ActivityItem) {
    if (activity.value.some((a) => a.id === item.id)) return
    activity.value = [item, ...activity.value].slice(0, MAX_ACTIVITY)
  }

  return { stats, activity, loading, error, load, updateStats, pushActivity }
})
