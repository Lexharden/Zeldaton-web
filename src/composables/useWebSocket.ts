import { storeToRefs } from 'pinia'
import { computed } from 'vue'
import { runtime } from '@/config/runtime'
import { getTransport } from '@/services/websocket'
import type { MessageHandler } from '@/services/websocket/transport'
import { useConnectionStore } from '@/stores/connection'
import type { WsClientMessage } from '@/types/websocket'
import { connectionQuality } from '@/utils/connection'
import { useNow } from './useNow'

/** Reactive facade over the shared realtime transport. */
export function useWebSocket() {
  const store = useConnectionStore()
  const { status, lastMessage, lastMessageAt, serverTimeUtc, reconnectCount } = storeToRefs(store)
  const { now } = useNow()

  const quality = computed(() =>
    connectionQuality(
      status.value,
      lastMessageAt.value === null ? null : now.value - lastMessageAt.value,
      runtime.ws.staleAfterMs,
    ),
  )

  return {
    status,
    lastMessage,
    serverTimeUtc,
    reconnectCount,
    quality,
    connect: () => getTransport().connect(),
    disconnect: () => getTransport().disconnect(),
    reconnect: () => getTransport().reconnect(),
    send: (message: WsClientMessage) => getTransport().send(message),
    subscribe: (handler: MessageHandler) => getTransport().subscribe(handler),
  }
}
