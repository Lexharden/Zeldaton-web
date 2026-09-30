/**
 * Wire-contract test. The Rust backend generates `contract/**.json` (see backend/tests/contract.rs);
 * this test proves the frontend accepts exactly what the server emits.
 */
import { DEFAULT_CATALOG } from '@/config/catalog'
import { readdirSync, readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it } from 'vitest'
import { normalizeMessage } from '@/services/websocket/normalize'
import { dispatchMessage } from '@/services/websocket/dispatcher'
import { useHiveShockStore } from '@/stores/hiveshock'
import { useRacersStore } from '@/stores/racers'
import type { Racer } from '@/types/racer'
import { buildGameProgress } from '@/utils/progress'

const root = fileURLToPath(new URL('../contract/', import.meta.url))
const read = <T = Record<string, unknown>>(rel: string): T =>
  JSON.parse(readFileSync(root + rel, 'utf8')) as T

const WS_TYPES = [
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
  'LIVE_ACTIVITY',
  'HIVESHOCK_STATS_UPDATED',
]

const RACER_STATUSES = ['online', 'live', 'paused', 'offline', 'exhausted', 'finished']

function expectKeys(value: Record<string, unknown>, keys: string[]) {
  for (const key of keys) expect(value, `missing "${key}"`).toHaveProperty(key)
}

describe('websocket fixtures', () => {
  it('has one fixture per message type', () => {
    const files = readdirSync(root + 'ws')
      .map((f) => f.replace('.json', ''))
      .sort()
    expect(files).toEqual([...WS_TYPES].sort())
  })

  it.each(WS_TYPES)('%s is accepted by normalizeMessage', (type) => {
    const fixture = read(`ws/${type}.json`)
    const result = normalizeMessage(fixture)
    expect(result.ok).toBe(true)
    if (result.ok) expect(result.message.type).toBe(type)
    // The socket reads this on every frame to keep its server-time offset fresh.
    expect(typeof fixture.serverTimeUtc).toBe('string')
  })

  it('carries clock fields the clock client depends on', () => {
    const snap = read<{ clocks: Record<string, unknown>[] }>('ws/CLOCK_SNAPSHOT.json')
    for (const c of snap.clocks) {
      expectKeys(c, ['racerId', 'serverTimeUtc', 'remainingMs', 'status', 'resetAtUtc'])
      expect(RACER_STATUSES).toContain(c.status)
    }
  })
})

describe('rest fixtures', () => {
  it('event', () => {
    const e = read<{ rules: Record<string, unknown>; status: string }>('rest/event.json')
    expectKeys(e, [
      'id',
      'name',
      'game',
      'edition',
      'status',
      'startAtUtc',
      'timezone',
      'dailyBudgetSeconds',
      'dailyResetLocalTime',
      'rules',
    ])
    expectKeys(e.rules, ['winCondition', 'requiredObjectiveIds'])
    expect(['upcoming', 'live', 'paused', 'finished']).toContain(e.status)
  })

  it('racers (with and without telemetry)', () => {
    for (const file of ['rest/racer.json', 'rest/racer-offline.json']) {
      const r = read<Record<string, unknown>>(file)
      expectKeys(r, [
        'id',
        'displayName',
        'slug',
        'timezone',
        'status',
        'elapsedSeconds',
        'remainingSeconds',
        'progressPercentage',
        'completedObjectives',
        'channels',
        'items',
      ])
      expect(RACER_STATUSES).toContain(r.status)
      // IANA identifier, never a UTC offset.
      expect(String(r.timezone)).toMatch(/^[A-Za-z_]+\/[A-Za-z_/]+$|^UTC$/)
    }
  })

  it('channels carry platform, handle and url', () => {
    const r = read<{ channels: Record<string, unknown>[] }>('rest/racer.json')
    expect(r.channels.map((c) => c.platform)).toEqual(['twitch', 'tiktok'])
    for (const c of r.channels) expectKeys(c, ['platform', 'handle', 'url'])
  })

  it('standings, stream, activity and stats', () => {
    for (const s of read<Record<string, unknown>[]>('rest/standings.json'))
      expectKeys(s, ['racerId', 'rank'])
    expectKeys(read('rest/stream.json'), ['racerId', 'isLive'])
    for (const a of read<Record<string, unknown>[]>('rest/activity.json')) {
      expectKeys(a, ['id', 'timestampUtc', 'kind', 'message', 'code'])
    }
    for (const file of ['rest/catalog.json', 'rest/catalog-default.json']) {
      const c = read<{
        version: string
        items: Record<string, unknown>[]
        objectives: Record<string, unknown>[]
      }>(file)
      expect(typeof c.version, file).toBe('string')
      for (const i of c.items)
        expectKeys(i, ['id', 'group', 'age', 'nameEs', 'nameEn', 'short', 'sortOrder', 'enabled'])
      for (const o of c.objectives)
        expectKeys(o, ['id', 'age', 'nameEs', 'nameEn', 'sortOrder', 'required', 'enabled'])
    }
    expectKeys(read('rest/hiveshock-stats.json'), [
      'connectedRacers',
      'gameEvents',
      'itemEvents',
      'progressEvents',
      'chatEvents',
    ])
  })

  it('a backend racer renders through the existing view helpers', () => {
    const racer = read<Racer>('rest/racer.json')
    const progress = buildGameProgress(racer, DEFAULT_CATALOG.objectives, (o) => o.nameEn)
    expect(progress.percentage).toBe(76)
    expect(progress.objectives.filter((o) => o.completed)).toHaveLength(2)
  })
})

describe('server frames drive the stores', () => {
  beforeEach(() => setActivePinia(createPinia()))

  it('GAME_PROGRESS updates only the matching racer', () => {
    const racers = useRacersStore()
    const ralbat = read<Racer>('rest/racer.json')
    const other = { ...read<Racer>('rest/racer-offline.json') }
    racers.setRacers([{ ...ralbat, progressPercentage: 10, completedObjectives: [] }, other])
    const frame = read('ws/GAME_PROGRESS.json')
    const parsed = normalizeMessage(frame)
    if (!parsed.ok) throw new Error('fixture rejected')
    dispatchMessage(parsed.message, frame.serverTimeUtc as string)
    expect(racers.getById('ralbat')?.progressPercentage).toBe(76)
    expect(racers.getById('ralbat')?.currentArea).toBe('water-temple')
    expect(racers.getById(other.id)?.progressPercentage).toBe(other.progressPercentage)
  })

  it('LIVE_ACTIVITY and HIVESHOCK_STATS_UPDATED reach the hiveshock store', () => {
    const hive = useHiveShockStore()
    for (const type of ['LIVE_ACTIVITY', 'HIVESHOCK_STATS_UPDATED']) {
      const frame = read(`ws/${type}.json`)
      const parsed = normalizeMessage(frame)
      if (!parsed.ok) throw new Error('fixture rejected')
      dispatchMessage(parsed.message, frame.serverTimeUtc as string)
    }
    expect(hive.activity[0]?.code).toBe('ITEM_ACQUIRED')
    expect(hive.stats?.gameEvents).toBe(12842)
  })
})
