import type { ConnectionStatus, TransportInfo, WsClientMessage, WsMessage } from '@/types/websocket'

export type MessageHandler = (message: WsMessage, serverTimeUtc?: string) => void
export type StateHandler = (status: ConnectionStatus, info: TransportInfo) => void

/** Contract implemented by both the real socket and the mock one. */
export interface RealtimeTransport {
  connect(): void
  disconnect(): void
  reconnect(): void
  send(message: WsClientMessage): void
  subscribe(handler: MessageHandler): () => void
  onState(handler: StateHandler): () => void
}

/** Small helper both implementations use to fan out to listeners. */
export class Emitter<A extends unknown[]> {
  private handlers = new Set<(...args: A) => void>()
  on(handler: (...args: A) => void): () => void {
    this.handlers.add(handler)
    return () => this.handlers.delete(handler)
  }
  emit(...args: A): void {
    for (const handler of this.handlers) {
      try {
        handler(...args)
      } catch (error) {
        console.error('[realtime] listener failed', error)
      }
    }
  }
  clear(): void {
    this.handlers.clear()
  }
}
