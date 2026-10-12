import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { raceApi } from '@/services/api'
import { ApiError } from '@/services/api/RaceApi'
import type { Poll } from '@/types/poll'
import { errorMessage } from '@/utils/errors'

const SNOOZE_KEY = 'zeldathon-poll-snooze'
const SNOOZE_MS = 24 * 60 * 60 * 1000

function readSnooze(): number {
  try {
    return Number(localStorage.getItem(SNOOZE_KEY)) || 0
  } catch {
    return 0
  }
}

/**
 * The "who will win?" poll. The server decides who already voted (by connection), so clearing the
 * browser never allows a second vote; localStorage only remembers that the visitor said "not now".
 */
export const usePollStore = defineStore('poll', () => {
  const poll = ref<Poll | null>(null)
  const loading = ref(false)
  const busy = ref(false)
  const error = ref<string | null>(null)
  const snoozedUntil = ref(readSnooze())

  const canVote = computed(() => !!poll.value && poll.value.open && !poll.value.voted)
  /** Whether the entry modal should invite this visitor to vote. */
  const shouldPrompt = computed(
    () =>
      canVote.value && (poll.value?.options.length ?? 0) > 1 && Date.now() >= snoozedUntil.value,
  )

  async function load() {
    loading.value = true
    error.value = null
    try {
      poll.value = await raceApi.getPoll()
    } catch (e) {
      error.value = errorMessage(e)
    } finally {
      loading.value = false
    }
  }

  /** Casts the vote. Returns whether it counted (or the server says this connection already voted). */
  async function vote(racerId: string, name: string): Promise<boolean> {
    busy.value = true
    error.value = null
    try {
      poll.value = await raceApi.vote(racerId, name.trim())
      return true
    } catch (e) {
      if (e instanceof ApiError && e.status === 409) {
        await load()
        return true
      }
      if (e instanceof ApiError && e.status === 403) await load()
      error.value = errorMessage(e)
      return false
    } finally {
      busy.value = false
    }
  }

  /** "Not now": no more invitations for a day. */
  function snooze() {
    snoozedUntil.value = Date.now() + SNOOZE_MS
    try {
      localStorage.setItem(SNOOZE_KEY, String(snoozedUntil.value))
    } catch {
      // Blocked storage: it only lasts for this visit.
    }
  }

  return { poll, loading, busy, error, canVote, shouldPrompt, load, vote, snooze }
})
