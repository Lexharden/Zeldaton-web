import { DEFAULT_CATALOG } from '@/config/catalog'
import { OBJECTIVES, ITEMS } from '@/config/event'
import { createMockActivity, nextActivityId } from '@/data/mock/activity'
import { MOCK_AREAS, MOCK_BOSSES } from '@/data/mock/areas'
import { createMockEvent } from '@/data/mock/event'
import { createMockRacers } from '@/data/mock/racers'
import { createMockStandings } from '@/data/mock/standings'
import { createMockStreams } from '@/data/mock/streams'
import type { DonorsBoard } from '@/types/donors'
import type { PublicDay } from '@/types/stats'
import type { EventInfo, EventStatus } from '@/types/event'
import type { HiveShockStats } from '@/types/hiveshock'
import type { Racer } from '@/types/racer'
import type {
  ActivityItem,
  ActivityKind,
  ClockState,
  StandingEntry,
  StreamInfo,
} from '@/types/race'
import type { WsMessage } from '@/types/websocket'
import { nextResetUtc } from '@/utils/time'
import { computeStandings } from '@/utils/standings'
import type { RaceApi } from '../api/RaceApi'

const delay = (ms = 350) => new Promise((r) => setTimeout(r, ms))
const pick = <T>(list: T[]): T => list[Math.floor(Math.random() * list.length)]
const clone = <T>(v: T): T => structuredClone(v)
const upper = (s: string) => s.toUpperCase().replace(/[^A-Z0-9]+/g, '_')

interface ClockRef {
  remainingMs: number
  refAt: number
  exhaustedAt?: number
}

function readForcedStatus(): EventStatus | null {
  const q =
    typeof location === 'undefined' ? null : new URLSearchParams(location.search).get('state')
  return q === 'upcoming' || q === 'live' || q === 'paused' || q === 'finished' ? q : null
}

/**
 * In-memory stand-in for the backend. It owns the "official" state (including clocks) and can
 * emit the same WebSocket messages a real server would, so the UI cannot tell the difference.
 */
export class MockRaceService implements RaceApi {
  private event: EventInfo
  private racers = new Map<string, Racer>()
  private clocks = new Map<string, ClockRef>()
  private activity: ActivityItem[]
  private stats: HiveShockStats = {
    connectedRacers: 8,
    gameEvents: 12842,
    itemEvents: 1294,
    progressEvents: 842,
    chatEvents: 319,
  }

  private forced: EventStatus | null

  constructor(forced: EventStatus | null = readForcedStatus()) {
    this.forced = forced
    this.event = createMockEvent(forced)
    const status = this.event.status
    let racers = createMockRacers()
    if (status === 'upcoming') racers = racers.map((r) => this.reset(r))
    if (status === 'finished') racers = racers.map((r, i) => (i === 0 ? this.finish(r) : r))
    for (const r of racers) {
      this.racers.set(r.id, r)
      this.clocks.set(r.id, { remainingMs: r.remainingSeconds * 1000, refAt: Date.now() })
    }
    this.activity = status === 'upcoming' ? [] : createMockActivity()
    if (status === 'upcoming')
      this.stats = {
        connectedRacers: 0,
        gameEvents: 0,
        itemEvents: 0,
        progressEvents: 0,
        chatEvents: 0,
      }
  }

  private reset(r: Racer): Racer {
    return {
      ...r,
      status: 'offline',
      progressPercentage: 0,
      elapsedSeconds: 0,
      remainingSeconds: this.event.dailyBudgetSeconds,
      completedObjectives: [],
      currentArea: undefined,
      currentObjective: undefined,
      items: {},
      stats: { hearts: 3, maxHearts: 3, rupees: 0, skulltulas: 0, bossesDefeated: 0 },
      stream: r.stream && { ...r.stream, isLive: false, viewers: undefined },
    }
  }

  private finish(r: Racer): Racer {
    return {
      ...r,
      status: 'finished',
      progressPercentage: 100,
      completedObjectives: OBJECTIVES.map((o) => o.id),
      currentObjective: undefined,
      currentArea: "Ganon's Castle",
      finalTimeSeconds: 8 * 3600 + 1284,
      finishedAtUtc: new Date().toISOString(),
    }
  }

  // ---- REST -------------------------------------------------------------
  async getEvent() {
    await delay()
    // Without a forced state the status follows the real schedule (upcoming -> live at start).
    if (!this.forced) this.event = { ...this.event, status: createMockEvent(null).status }
    return clone(this.event)
  }
  async getRacers() {
    await delay(500)
    return [...this.racers.values()].map((r) => this.withClock(r))
  }
  async getStandings(): Promise<StandingEntry[]> {
    await delay(300)
    return createMockStandings([...this.racers.values()])
  }
  async getRacer(id: string) {
    await delay(300)
    const racer = this.racers.get(id)
    if (!racer) throw new Error('not found')
    return this.withClock(racer)
  }
  async getStreams(): Promise<StreamInfo[]> {
    await delay(450)
    return createMockStreams([...this.racers.values()])
  }
  async getActivity() {
    await delay(400)
    return clone(this.activity)
  }
  async getHiveShockStats() {
    await delay(400)
    return { ...this.stats }
  }
  async getRacerDays(): Promise<PublicDay[]> {
    await delay(300)
    return [
      ['2026-10-07', 12_600, 2, 3, 6, 1, 5, 4, 1_800, 300, 0, 18, 24, 310],
      ['2026-10-08', 14_100, 3, 4, 9, 2, 7, 6, 2_700, 0, 1, 24, 41, 520],
    ].map(
      ([
        day,
        played,
        sessions,
        objectives,
        items,
        bosses,
        areas,
        donations,
        added,
        removed,
        exhausted,
        from,
        to,
        viewers,
      ]) => ({
        day: day as string,
        playedSeconds: played as number,
        sessions: sessions as number,
        objectives: objectives as number,
        items: items as number,
        bosses: bosses as number,
        areas: areas as number,
        donations: donations as number,
        donationAddedSeconds: added as number,
        donationRemovedSeconds: removed as number,
        exhausted: exhausted as number,
        progressStart: from as number,
        progressEnd: to as number,
        peakViewers: viewers as number,
        partial: false,
      }),
    )
  }
  async getDonors(): Promise<DonorsBoard> {
    await delay(300)
    const donors: DonorsBoard['donors'] = [
      ['FanDeLink', 'tiktok', 'diamonds', 7, 1200, 5400, 300],
      ['navi_fan', 'twitch', 'bits', 4, 800, 3600, 0],
      ['Epona_Rider', 'tiktok', 'diamonds', 5, 650, 2700, 600],
      ['deku_nut', 'twitch', 'bits', 3, 300, 1200, 0],
      ['Zelda_Lover', 'tiktok', 'diamonds', 2, 150, 600, 120],
    ].map(([viewer, platform, currency, donations, amount, addedSeconds, removedSeconds], i) => ({
      rank: i + 1,
      viewer: viewer as string,
      platform: platform as 'tiktok' | 'twitch',
      currency: currency as 'diamonds' | 'bits',
      donations: donations as number,
      amount: amount as number,
      addedSeconds: addedSeconds as number,
      removedSeconds: removedSeconds as number,
    }))
    return {
      enabled: true,
      totals: { donations: 21, addedSeconds: 13500, removedSeconds: 1020 },
      donors,
    }
  }
  async getCatalog() {
    await delay(150)
    return clone(DEFAULT_CATALOG)
  }
  async getClocks() {
    await delay(200)
    return this.clockSnapshot()
  }

  // ---- Clocks -----------------------------------------------------------
  private remainingMs(id: string, now = Date.now()): number {
    const ref = this.clocks.get(id)
    const racer = this.racers.get(id)
    if (!ref || !racer) return 0
    if (racer.status !== 'live') return ref.remainingMs
    return Math.max(0, ref.remainingMs - (now - ref.refAt))
  }

  /** Freeze the current remaining value so a status change starts a clean interval. */
  private freeze(id: string, now = Date.now()) {
    const ref = this.clocks.get(id)
    if (ref) {
      ref.remainingMs = this.remainingMs(id, now)
      ref.refAt = now
    }
  }

  private withClock(r: Racer): Racer {
    const remaining = Math.round(this.remainingMs(r.id) / 1000)
    return {
      ...clone(r),
      remainingSeconds: remaining,
      elapsedSeconds: this.event.dailyBudgetSeconds - remaining,
    }
  }

  private clockOf(id: string, now = Date.now()): ClockState {
    const racer = this.racers.get(id)!
    return {
      racerId: id,
      serverTimeUtc: new Date(now).toISOString(),
      remainingMs: this.remainingMs(id, now),
      status: racer.status,
      resetAtUtc: new Date(
        nextResetUtc(now, racer.timezone, this.event.dailyResetLocalTime),
      ).toISOString(),
    }
  }

  clockSnapshot(): ClockState[] {
    const now = Date.now()
    return [...this.racers.keys()].map((id) => this.clockOf(id, now))
  }

  // ---- Simulation -------------------------------------------------------
  private note(
    kind: ActivityKind,
    racer: Racer | undefined,
    message: string,
    code: string,
    detail?: string,
    subject?: string,
  ): WsMessage {
    const activity: ActivityItem = {
      id: nextActivityId(),
      timestampUtc: new Date().toISOString(),
      kind,
      racerId: racer?.id,
      racerName: racer?.displayName,
      message,
      code,
      detail,
      subject,
    }
    this.activity = [activity, ...this.activity].slice(0, 60)
    return { type: 'LIVE_ACTIVITY', activity }
  }

  private setStatus(racer: Racer, status: Racer['status']): WsMessage[] {
    this.freeze(racer.id)
    racer.status = status
    if (racer.stream) racer.stream.isLive = status === 'live' || status === 'paused'
    this.clocks.get(racer.id)!.refAt = Date.now()
    return [
      { type: 'RACER_STATUS_CHANGED', racerId: racer.id, status },
      { type: 'CLOCK_SYNC', clock: this.clockOf(racer.id) },
    ]
  }

  /** Called every second: detects time exhaustion and (demo only) fast-forwards resets. */
  tickClocks(demo: boolean): WsMessage[] {
    const out: WsMessage[] = []
    const now = Date.now()
    for (const racer of this.racers.values()) {
      const ref = this.clocks.get(racer.id)!
      if (racer.status === 'live' && this.remainingMs(racer.id, now) <= 0) {
        out.push({
          type: 'SESSION_EXHAUSTED',
          racerId: racer.id,
          serverTimeUtc: new Date(now).toISOString(),
        })
        out.push({ type: 'GAME_FORCE_CLOSE', racerId: racer.id })
        out.push(...this.setStatus(racer, 'exhausted'))
        out.push(
          this.note('status', racer, `${racer.displayName} ran out of time`, 'SESSION_EXHAUSTED'),
        )
        ref.exhaustedAt = now
      } else if (
        demo &&
        racer.status === 'exhausted' &&
        ref.exhaustedAt &&
        now - ref.exhaustedAt > 35_000
      ) {
        // Demo shortcut: a real reset only happens at 06:00 local time.
        ref.remainingMs = this.event.dailyBudgetSeconds * 1000
        ref.refAt = now
        ref.exhaustedAt = undefined
        racer.status = 'online'
        out.push({ type: 'DAILY_RESET', racerId: racer.id, clock: this.clockOf(racer.id, now) })
        out.push(
          this.note(
            'reset',
            racer,
            `Daily reset completed for ${racer.displayName}`,
            'DAILY_RESET',
            '04:00:00 AVAILABLE',
          ),
        )
      }
    }
    return out
  }

  private objectiveCount(pct: number) {
    return Math.min(OBJECTIVES.length, Math.floor(pct / 10))
  }

  private setProgress(racer: Racer, pct: number): WsMessage[] {
    const out: WsMessage[] = []
    const before = racer.completedObjectives.length
    racer.progressPercentage = Math.min(100, pct)
    const count = this.objectiveCount(racer.progressPercentage)
    racer.completedObjectives = OBJECTIVES.slice(0, count).map((o) => o.id)
    const next = OBJECTIVES[count]
    racer.currentObjective = next?.id ?? 'ganons-castle'
    if (count > before) racer.currentArea = MOCK_AREAS[Math.min(count, MOCK_AREAS.length - 1)]
    out.push({
      type: 'GAME_PROGRESS',
      racerId: racer.id,
      progress: {
        percentage: racer.progressPercentage,
        currentArea: racer.currentArea,
        currentObjective: racer.currentObjective,
        completedObjectives: [...racer.completedObjectives],
      },
    })
    if (count > before) {
      out.push(
        this.note(
          'area',
          racer,
          `${racer.displayName} entered ${racer.currentArea}`,
          'AREA_CHANGED',
          upper(racer.currentArea ?? ''),
          racer.currentArea,
        ),
      )
    }
    if (racer.progressPercentage >= 100) {
      const now = Date.now()
      racer.finishedAtUtc = new Date(now).toISOString()
      racer.finalTimeSeconds = 8 * 3600 + Math.floor(Math.random() * 3000)
      out.push({
        type: 'GAME_FINISHED',
        racerId: racer.id,
        finalTimeSeconds: racer.finalTimeSeconds,
        finishedAtUtc: racer.finishedAtUtc,
      })
      out.push(...this.setStatus(racer, 'finished'))
      out.push(
        this.note(
          'finish',
          racer,
          `${racer.displayName} completed Ocarina of Time`,
          'GAME_FINISHED',
        ),
      )
    }
    return out
  }

  /** One demo beat: a random, coherent change of race state. */
  demoStep(): WsMessage[] {
    const all = [...this.racers.values()]
    const active = all.filter((r) => r.status === 'live')
    const out: WsMessage[] = []
    const scenario = pick([
      'progress',
      'progress',
      'progress',
      'item',
      'area',
      'boss',
      'pause',
      'overtake',
      'stats',
    ])

    if (scenario === 'progress' && active.length) {
      const r = pick(active)
      out.push(...this.setProgress(r, r.progressPercentage + pick([1, 1, 2])))
    } else if (scenario === 'overtake') {
      const ranked = computeStandings(all).map((s) => this.racers.get(s.racerId)!)
      const idx = ranked.findIndex(
        (r, i) => i > 0 && r.status === 'live' && ranked[i - 1].status !== 'finished',
      )
      if (idx > 0)
        out.push(...this.setProgress(ranked[idx], ranked[idx - 1].progressPercentage + 1))
    } else if (scenario === 'item' && active.length) {
      const r = pick(active)
      const missing = ITEMS.filter((i) => !r.items?.[i.id])
      if (missing.length) {
        const item = pick(missing)
        r.items = { ...r.items, [item.id]: true }
        this.stats.itemEvents += 1
        out.push({ type: 'ITEM_ACQUIRED', racerId: r.id, item: item.id })
        out.push(
          this.note(
            'item',
            r,
            `${r.displayName} acquired ${item.id}`,
            'ITEM_ACQUIRED',
            upper(item.id),
            item.id,
          ),
        )
      }
    } else if (scenario === 'area' && active.length) {
      const r = pick(active)
      const pool = MOCK_AREAS.slice(0, Math.max(2, r.completedObjectives.length + 1))
      const area = pick(pool.filter((a) => a !== r.currentArea))
      r.currentArea = area
      out.push({ type: 'AREA_CHANGED', racerId: r.id, area })
      out.push(
        this.note('area', r, `${r.displayName} entered ${area}`, 'AREA_CHANGED', upper(area), area),
      )
    } else if (scenario === 'boss' && active.length) {
      const r = pick(active)
      const boss = pick(MOCK_BOSSES)
      const count = Math.min(10, (r.stats?.bossesDefeated ?? 0) + 1)
      r.stats = { ...r.stats, bossesDefeated: count }
      out.push({ type: 'BOSS_DEFEATED', racerId: r.id, boss, bossesDefeated: count })
      out.push(
        this.note(
          'boss',
          r,
          `${r.displayName} defeated ${boss}`,
          'BOSS_DEFEATED',
          upper(boss),
          boss,
        ),
      )
    } else if (scenario === 'stats' && active.length) {
      const r = pick(active)
      const rupees = (r.stats?.rupees ?? 0) + pick([5, 20, 50])
      r.stats = { ...r.stats, rupees }
      out.push({ type: 'STATS_UPDATED', racerId: r.id, stats: { rupees } })
    } else if (scenario === 'pause') {
      const paused = all.find((r) => r.status === 'paused' || r.status === 'online')
      const target = paused ?? pick(active)
      if (target) {
        const resume = target.status === 'paused' || target.status === 'online'
        out.push(...this.setStatus(target, resume ? 'live' : 'paused'))
        out.push(
          this.note(
            'status',
            target,
            `${target.displayName} ${resume ? 'resumed' : 'paused'} the game`,
            resume ? 'SESSION_RESUMED' : 'SESSION_PAUSED',
          ),
        )
      }
    }

    this.stats.gameEvents += 8 + Math.floor(Math.random() * 30)
    this.stats.progressEvents += out.some((m) => m.type === 'GAME_PROGRESS') ? 1 : 0
    this.stats.chatEvents += Math.floor(Math.random() * 6)
    this.stats.connectedRacers = all.filter((r) => r.status !== 'offline').length
    out.push({ type: 'HIVESHOCK_STATS_UPDATED', stats: { ...this.stats } })
    return out
  }
}
