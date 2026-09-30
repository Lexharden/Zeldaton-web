import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import type { ConnectionStatus, WsMessage } from '@/types/websocket'
import { computeServerOffsetMs } from '@/utils/clock'
import { runtime } from '@/config/runtime'

export const useConnectionStore = defineStore('connection', () => {
  const status = ref<ConnectionStatus>('disconnected')
  const lastMessage = ref<WsMessage | null>(null)
  const lastMessageAt = ref<number | null>(null)
  const serverTimeUtc = ref<string | null>(null)
  /** server - local, so event countdowns follow server time. */
  const serverOffsetMs = ref(0)
  const reconnectCount = ref(0)
  const reconnectAttempts = ref(0)
  const maxAttempts = ref<number>(runtime.ws.maxAttempts)
  const nextRetryInMs = ref<number | null>(null)

  const isLive = computed(() => status.value === 'connected')

  function setStatus(
    next: ConnectionStatus,
    info?: { reconnectAttempts: number; maxAttempts: number; nextRetryInMs?: number },
  ) {
    if (next === 'reconnecting') reconnectCount.value += 1
    status.value = next
    if (info) {
      reconnectAttempts.value = info.reconnectAttempts
      maxAttempts.value = info.maxAttempts
      nextRetryInMs.value = info.nextRetryInMs ?? null
    }
  }

  function recordMessage(message: WsMessage, serverTime?: string) {
    lastMessage.value = message
    lastMessageAt.value = Date.now()
    if (serverTime) {
      serverTimeUtc.value = serverTime
      const now = Date.now()
      serverOffsetMs.value = computeServerOffsetMs(serverTime, now)
    }
  }

  return {
    status,
    lastMessage,
    lastMessageAt,
    serverTimeUtc,
    serverOffsetMs,
    reconnectCount,
    reconnectAttempts,
    maxAttempts,
    nextRetryInMs,
    isLive,
    setStatus,
    recordMessage,
  }
})
