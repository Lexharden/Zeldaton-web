import { runtime } from '@/config/runtime'
import type { ConnectionStatus, TransportInfo, WsClientMessage, WsMessage } from '@/types/websocket'
import { normalizeMessage } from './normalize'
import {
  Emitter,
  type MessageHandler,
  type RealtimeTransport,
  type StateHandler,
} from './transport'

export interface RaceSocketOptions {
  url: string
  maxAttempts?: number
  baseDelayMs?: number
  maxDelayMs?: number
  heartbeatMs?: number
}

/** Exponential backoff with a small jitter, capped at `max`. */
export function backoffDelay(attempt: number, base: number, max: number, random = Math.random) {
  const exp = Math.min(max, base * 2 ** Math.max(0, attempt - 1))
  return Math.round(exp * (0.85 + random() * 0.3))
}

/**
 * Native WebSocket adapter: reconnect with exponential backoff, heartbeat, and safe parsing.
 * Emits typed messages; it never touches stores (the dispatcher does).
 */
export class RaceSocket implements RealtimeTransport {
  private ws: WebSocket | null = null
  private messages = new Emitter<[WsMessage, string | undefined]>()
  private states = new Emitter<[ConnectionStatus, TransportInfo]>()
  private attempts = 0
  private retryTimer: ReturnType<typeof setTimeout> | null = null
  private heartbeat: ReturnType<typeof setInterval> | null = null
  private manualClose = false
  private opts: Required<RaceSocketOptions>

  constructor(options: RaceSocketOptions) {
    this.opts = {
      maxAttempts: runtime.ws.maxAttempts,
      baseDelayMs: runtime.ws.baseDelayMs,
      maxDelayMs: runtime.ws.maxDelayMs,
      heartbeatMs: runtime.ws.heartbeatMs,
      ...options,
    }
  }

  connect(): void {
    if (this.ws && this.ws.readyState <= WebSocket.OPEN) return
    this.manualClose = false
    this.open(this.attempts > 0 ? 'reconnecting' : 'connecting')
  }

  disconnect(): void {
    this.manualClose = true
    this.clearTimers()
    this.ws?.close(1000, 'client disconnect')
    this.ws = null
    this.attempts = 0
    this.setState('disconnected')
  }

  reconnect(): void {
    this.disconnect()
    this.attempts = 0
    this.connect()
  }

  send(message: WsClientMessage): void {
    if (this.ws?.readyState === WebSocket.OPEN) this.ws.send(JSON.stringify(message))
  }

  subscribe(handler: MessageHandler): () => void {
    return this.messages.on(handler)
  }

  onState(handler: StateHandler): () => void {
    return this.states.on(handler)
  }

  private open(status: ConnectionStatus) {
    this.setState(status)
    let socket: WebSocket
    try {
      socket = new WebSocket(this.opts.url)
    } catch (error) {
      console.warn('[ws] could not create socket', error)
      this.setState('error')
      this.scheduleRetry()
      return
    }
    this.ws = socket

    socket.onopen = () => {
      this.attempts = 0
      this.setState('connected')
      this.startHeartbeat()
    }
    socket.onmessage = (event) => {
      const result = normalizeMessage(event.data)
      if (result.ok) {
        const server = (result.message as { serverTimeUtc?: string }).serverTimeUtc
        this.messages.emit(result.message, server)
      } else if (result.reason === 'unknown') {
        console.warn('[ws] ignoring unknown event type', result.type)
      } else {
        console.warn('[ws] ignoring malformed payload', result.type ?? '')
      }
    }
    socket.onerror = () => this.setState('error')
    socket.onclose = () => {
      this.stopHeartbeat()
      this.ws = null
      if (!this.manualClose) this.scheduleRetry()
    }
  }

  private scheduleRetry() {
    if (this.attempts >= this.opts.maxAttempts) {
      this.setState('error')
      return
    }
    this.attempts += 1
    const delay = backoffDelay(this.attempts, this.opts.baseDelayMs, this.opts.maxDelayMs)
    this.setState('reconnecting', delay)
    this.retryTimer = setTimeout(() => this.open('reconnecting'), delay)
  }

  private startHeartbeat() {
    this.stopHeartbeat()
    this.heartbeat = setInterval(
      () => this.send({ type: 'PING', sentAt: Date.now() }),
      this.opts.heartbeatMs,
    )
  }

  private stopHeartbeat() {
    if (this.heartbeat) clearInterval(this.heartbeat)
    this.heartbeat = null
  }

  private clearTimers() {
    this.stopHeartbeat()
    if (this.retryTimer) clearTimeout(this.retryTimer)
    this.retryTimer = null
  }

  private setState(status: ConnectionStatus, nextRetryInMs?: number) {
    this.states.emit(status, {
      reconnectAttempts: this.attempts,
      maxAttempts: this.opts.maxAttempts,
      nextRetryInMs,
    })
  }
}
