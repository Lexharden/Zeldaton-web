import type { WsMessage, WsMessageType } from '@/types/websocket'

const KNOWN: ReadonlySet<WsMessageType> = new Set<WsMessageType>([
  'CLOCK_SNAPSHOT',
  'CLOCK_SYNC',
  'DAILY_RESET',
  'SESSION_STARTED',
  'SESSION_PAUSED',
  'SESSION_RESUMED',
  'SESSION_EXHAUSTED',
  'GAME_FORCE_CLOSE',
  'GAME_PROGRESS',
  'ITEM_ACQUIRED',
  'AREA_CHANGED',
  'BOSS_DEFEATED',
  'STATS_UPDATED',
  'GAME_FINISHED',
  'RACER_STATUS_CHANGED',
  'STREAM_UPDATED',
  'CATALOG_UPDATED',
  'EVENT_UPDATED',
  'LIVE_ACTIVITY',
  'HIVESHOCK_STATS_UPDATED',
])

type Shape = Record<string, unknown>
const isObject = (v: unknown): v is Shape =>
  typeof v === 'object' && v !== null && !Array.isArray(v)
const isString = (v: unknown): v is string => typeof v === 'string' && v.length > 0

/** Required payload keys per message type. Anything else is optional. */
const REQUIRED: Partial<Record<WsMessageType, string[]>> = {
  CLOCK_SNAPSHOT: ['clocks'],
  CLOCK_SYNC: ['clock'],
  DAILY_RESET: ['racerId', 'clock'],
  SESSION_STARTED: ['racerId'],
  SESSION_PAUSED: ['racerId'],
  SESSION_RESUMED: ['racerId'],
  SESSION_EXHAUSTED: ['racerId'],
  GAME_FORCE_CLOSE: ['racerId'],
  GAME_PROGRESS: ['racerId', 'progress'],
  ITEM_ACQUIRED: ['racerId', 'item'],
  AREA_CHANGED: ['racerId', 'area'],
  BOSS_DEFEATED: ['racerId', 'boss'],
  STATS_UPDATED: ['racerId', 'stats'],
  GAME_FINISHED: ['racerId'],
  RACER_STATUS_CHANGED: ['racerId', 'status'],
  STREAM_UPDATED: ['racerId', 'stream'],
  CATALOG_UPDATED: ['version'],
  LIVE_ACTIVITY: ['activity'],
  HIVESHOCK_STATS_UPDATED: ['stats'],
}

export type NormalizeResult =
  { ok: true; message: WsMessage } | { ok: false; reason: 'malformed' | 'unknown'; type?: string }

/**
 * Turns raw socket data (string or parsed JSON) into a typed message.
 * Never throws: malformed or unknown payloads yield a result the caller can log and ignore.
 */
export function normalizeMessage(raw: unknown): NormalizeResult {
  let data: unknown = raw
  if (typeof raw === 'string') {
    try {
      data = JSON.parse(raw)
    } catch {
      return { ok: false, reason: 'malformed' }
    }
  }
  if (!isObject(data) || !isString(data.type)) return { ok: false, reason: 'malformed' }

  const type = data.type as WsMessageType
  if (!KNOWN.has(type)) return { ok: false, reason: 'unknown', type: data.type }

  for (const key of REQUIRED[type] ?? []) {
    const value = data[key]
    if (value === undefined || value === null) return { ok: false, reason: 'malformed', type }
  }
  if (type === 'CLOCK_SNAPSHOT' && !Array.isArray(data.clocks)) {
    return { ok: false, reason: 'malformed', type }
  }
  if (type !== 'CLOCK_SNAPSHOT' && REQUIRED[type]?.includes('racerId') && !isString(data.racerId)) {
    return { ok: false, reason: 'malformed', type }
  }
  return { ok: true, message: data as unknown as WsMessage }
}
