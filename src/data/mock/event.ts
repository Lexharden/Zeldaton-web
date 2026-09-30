import { EVENT_SCHEDULE, OBJECTIVES } from '@/config/event'
import type { EventInfo, EventStatus } from '@/types/event'

/**
 * Simulated event. Without a forced state the status follows the real schedule
 * (`upcoming` until the start, `live` after); `?state=` forces one for previews.
 */
export function createMockEvent(forced: EventStatus | null): EventInfo {
  const started = Date.now() >= Date.parse(EVENT_SCHEDULE.startAtUtc)
  return {
    id: 'zeldathon-2026',
    name: 'Zeldathon',
    game: 'Ocarina of Time',
    edition: '2026',
    status: forced ?? (started ? 'live' : 'upcoming'),
    startAtUtc: EVENT_SCHEDULE.startAtUtc,
    timezone: EVENT_SCHEDULE.timezone,
    dailyBudgetSeconds: 4 * 3600,
    dailyResetLocalTime: '06:00',
    rules: {
      winCondition: 'First racer to complete all required objectives crosses the finish line.',
      requiredObjectiveIds: OBJECTIVES.map((o) => o.id),
    },
  }
}
