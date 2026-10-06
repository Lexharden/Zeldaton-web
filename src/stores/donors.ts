import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { raceApi } from '@/services/api'
import type { DonorsBoard } from '@/types/donors'
import { errorMessage } from '@/utils/errors'

/** The public donors board (who moved the race clock the most). Read-only; the organizer curates it. */
export const useDonorsStore = defineStore('donors', () => {
  const board = ref<DonorsBoard | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)
  let soon: ReturnType<typeof setTimeout> | null = null

  /** Shown while the first answer is pending and whenever the organizer has it on; hidden on failure. */
  const visible = computed(() => (board.value ? board.value.enabled : loading.value))
  const donors = computed(() => board.value?.donors ?? [])

  async function load() {
    loading.value = true
    error.value = null
    try {
      board.value = await raceApi.getDonors()
    } catch (e) {
      error.value = errorMessage(e)
    } finally {
      loading.value = false
    }
  }

  /** A donation just landed: refresh shortly (one request for a burst of them). */
  function refreshSoon() {
    if (soon) return
    soon = setTimeout(() => {
      soon = null
      void load()
    }, 3000)
  }

  return { board, loading, error, visible, donors, load, refreshSoon }
})
