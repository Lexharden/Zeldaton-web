import { runtime } from '@/config/runtime'
import type { ConnectionStatus, TransportInfo, WsClientMessage, WsMessage } from '@/types/websocket'
import {
  Emitter,
  type MessageHandler,
  type RealtimeTransport,
  type StateHandler,
} from '../websocket/transport'
import type { MockRaceService } from './MockRaceService'

/** Simulated socket: same contract as RaceSocket, driven by the in-memory mock server. */
export class MockWebSocketService implements RealtimeTransport {
  private messages = new Emitter<[WsMessage, string | undefined]>()
  private states = new Emitter<[ConnectionStatus, TransportInfo]>()
  private timers: ReturnType<typeof setInterval>[] = []
  private connectTimer: ReturnType<typeof setTimeout> | null = null
  private status: ConnectionStatus = 'disconnected'

  constructor(private server: MockRaceService) {}

  connect(): void {
    if (this.status === 'connected' || this.status === 'connecting') return
    this.setState('connecting')
    this.connectTimer = setTimeout(() => {
      this.setState('connected')
      this.push({ type: 'CLOCK_SNAPSHOT', clocks: this.server.clockSnapshot() })
      this.timers.push(
        setInterval(() => this.pushAll(this.server.tickClocks(runtime.demoMode)), 1000),
      )
      if (runtime.demoMode) {
        this.timers.push(
          setInterval(() => this.pushAll(this.server.demoStep()), runtime.demoTickMs),
        )
      }
    }, 600)
  }

  disconnect(): void {
    this.stop()
    this.setState('disconnected')
  }

  reconnect(): void {
    this.stop()
    this.status = 'disconnected'
    this.connect()
  }

  send(message: WsClientMessage): void {
    if (this.status !== 'connected') return
    if (message.type === 'CLOCK_SYNC_REQUEST') {
      this.push({ type: 'CLOCK_SNAPSHOT', clocks: this.server.clockSnapshot() })
    }
  }

  subscribe(handler: MessageHandler): () => void {
    return this.messages.on(handler)
  }

  onState(handler: StateHandler): () => void {
    return this.states.on(handler)
  }

  private stop() {
    for (const t of this.timers) clearInterval(t)
    this.timers = []
    if (this.connectTimer) clearTimeout(this.connectTimer)
    this.connectTimer = null
  }

  private pushAll(list: WsMessage[]) {
    for (const m of list) this.push(m)
  }

  private push(message: WsMessage) {
    this.messages.emit(message, new Date().toISOString())
  }

  private setState(status: ConnectionStatus) {
    this.status = status
    this.states.emit(status, { reconnectAttempts: 0, maxAttempts: runtime.ws.maxAttempts })
  }
}
