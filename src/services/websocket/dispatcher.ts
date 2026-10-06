import { loadAllData } from '@/services/loadAll'
import { useCatalogStore } from '@/stores/catalog'
import { useDonorsStore } from '@/stores/donors'
import { useConnectionStore } from '@/stores/connection'
import { useHiveShockStore } from '@/stores/hiveshock'
import { useRaceStore } from '@/stores/race'
import { useRacersStore } from '@/stores/racers'
import type { WsMessage } from '@/types/websocket'

/**
 * The only place where realtime messages mutate state. Each case touches just the
 * racer/slice it refers to; nothing here reloads or resets the application.
 */
export function dispatchMessage(message: WsMessage, serverTimeUtc?: string): void {
  const racers = useRacersStore()
  const race = useRaceStore()
  const hive = useHiveShockStore()
  useConnectionStore().recordMessage(message, serverTimeUtc)

  switch (message.type) {
    case 'CLOCK_SNAPSHOT':
      message.clocks.forEach((c) => racers.setClock(c))
      break
    case 'CLOCK_SYNC':
      racers.setClock(message.clock)
      break
    case 'DAILY_RESET':
      racers.setClock(message.clock)
      break
    case 'SESSION_STARTED':
    case 'SESSION_RESUMED':
      racers.patch(message.racerId, { status: 'live' })
      break
    case 'SESSION_PAUSED':
      racers.patch(message.racerId, { status: 'paused' })
      break
    case 'SESSION_EXHAUSTED':
    case 'GAME_FORCE_CLOSE':
      racers.patch(message.racerId, { status: 'exhausted', remainingSeconds: 0 })
      break
    case 'GAME_PROGRESS': {
      const { percentage, currentArea, currentObjective, completedObjectives } = message.progress
      const changes: Parameters<typeof racers.patch>[1] = {}
      if (percentage !== undefined) changes.progressPercentage = percentage
      if (currentArea !== undefined) changes.currentArea = currentArea
      if (currentObjective !== undefined) changes.currentObjective = currentObjective
      if (completedObjectives) changes.completedObjectives = completedObjectives
      racers.patch(message.racerId, changes)
      break
    }
    case 'ITEM_ACQUIRED': {
      const racer = racers.getById(message.racerId)
      if (racer) racer.items = { ...racer.items, [message.item]: true }
      break
    }
    case 'AREA_CHANGED':
      racers.patch(message.racerId, { currentArea: message.area })
      break
    case 'BOSS_DEFEATED': {
      const racer = racers.getById(message.racerId)
      if (racer) {
        const count = message.bossesDefeated ?? (racer.stats?.bossesDefeated ?? 0) + 1
        racer.stats = { ...racer.stats, bossesDefeated: count }
      }
      break
    }
    case 'STATS_UPDATED': {
      const racer = racers.getById(message.racerId)
      if (racer) racer.stats = { ...racer.stats, ...message.stats }
      break
    }
    case 'GAME_FINISHED':
      racers.patch(message.racerId, {
        status: 'finished',
        progressPercentage: 100,
        finalTimeSeconds: message.finalTimeSeconds,
        finishedAtUtc: message.finishedAtUtc ?? new Date().toISOString(),
      })
      race.declareWinner({
        racerId: message.racerId,
        finalTimeSeconds: message.finalTimeSeconds,
        finishedAtUtc: message.finishedAtUtc,
      })
      break
    case 'RACER_STATUS_CHANGED':
      racers.patch(message.racerId, { status: message.status })
      race.setStreamLive(message.racerId, message.status === 'live' || message.status === 'paused')
      break
    case 'LIVE_ACTIVITY':
      hive.pushActivity(message.activity)
      // Donation time landed: the donors board changed too.
      if (message.activity.kind === 'time') useDonorsStore().refreshSoon()
      break
    case 'CATALOG_UPDATED':
      void useCatalogStore().onUpdated(message.version)
      break
    case 'EVENT_UPDATED':
      // A reset or a rehearsal switch: what we hold is stale in many places at once.
      race.clearWinner()
      void loadAllData()
      break
    case 'STREAM_UPDATED':
      racers.patch(message.racerId, { stream: message.stream })
      break
    case 'HIVESHOCK_STATS_UPDATED':
      hive.updateStats(message.stats)
      break
    default: {
      const unhandled: never = message
      console.warn('[realtime] unhandled message', unhandled)
    }
  }
}
