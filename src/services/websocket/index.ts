import { runtime } from '@/config/runtime'
import { getMockServer } from '../mock'
import { MockWebSocketService } from '../mock/MockWebSocketService'
import { RaceSocket } from './RaceSocket'
import type { RealtimeTransport } from './transport'

let transport: RealtimeTransport | null = null

/** Single shared transport for the whole app. */
export function getTransport(): RealtimeTransport {
  transport ??= runtime.mockMode
    ? new MockWebSocketService(getMockServer())
    : new RaceSocket({ url: runtime.wsUrl })
  return transport
}
