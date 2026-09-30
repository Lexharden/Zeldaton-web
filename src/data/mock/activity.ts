import type { ActivityItem, ActivityKind } from '@/types/race'

let counter = 0

/** Ids for activity created while the simulation runs (`mock-1`, `mock-2`, ...). */
export function nextActivityId(): string {
  counter += 1
  return `mock-${counter}`
}

interface Seed {
  minutesAgo: number
  kind: ActivityKind
  racerId: string
  racerName: string
  code: string
  message: string
  subject?: string
}

const SEEDS: Seed[] = [
  {
    minutesAgo: 1,
    kind: 'item',
    racerId: 'ralbat',
    racerName: 'Ralbat',
    code: 'ITEM_ACQUIRED',
    message: 'Ralbat acquired longshot',
    subject: 'longshot',
  },
  {
    minutesAgo: 3,
    kind: 'boss',
    racerId: 'pinchiviejo',
    racerName: 'Pinchiviejo',
    code: 'BOSS_DEFEATED',
    message: 'Pinchiviejo defeated barinade',
    subject: 'barinade',
  },
  {
    minutesAgo: 5,
    kind: 'area',
    racerId: 'xime',
    racerName: 'Xime',
    code: 'AREA_CHANGED',
    message: 'Xime entered lake-hylia',
    subject: 'lake-hylia',
  },
  {
    minutesAgo: 8,
    kind: 'status',
    racerId: 'cuaco',
    racerName: 'Cuaco',
    code: 'SESSION_PAUSED',
    message: 'Cuaco paused the game',
  },
  {
    minutesAgo: 11,
    kind: 'item',
    racerId: 'kahrilys',
    racerName: 'Kahrilys',
    code: 'ITEM_ACQUIRED',
    message: 'Kahrilys acquired boomerang',
    subject: 'boomerang',
  },
  {
    minutesAgo: 14,
    kind: 'area',
    racerId: 'ralbat',
    racerName: 'Ralbat',
    code: 'AREA_CHANGED',
    message: 'Ralbat entered water-temple',
    subject: 'water-temple',
  },
  {
    minutesAgo: 18,
    kind: 'status',
    racerId: 'perlarev',
    racerName: 'Perlarev',
    code: 'SESSION_EXHAUSTED',
    message: 'Perlarev ran out of time',
  },
  {
    minutesAgo: 22,
    kind: 'boss',
    racerId: 'ralbat',
    racerName: 'Ralbat',
    code: 'BOSS_DEFEATED',
    message: 'Ralbat defeated morpha',
    subject: 'morpha',
  },
  {
    minutesAgo: 30,
    kind: 'item',
    racerId: 'pinchiviejo',
    racerName: 'Pinchiviejo',
    code: 'ITEM_ACQUIRED',
    message: 'Pinchiviejo acquired hookshot',
    subject: 'hookshot',
  },
  {
    minutesAgo: 41,
    kind: 'reset',
    racerId: 'speaksins',
    racerName: 'Speaksins',
    code: 'DAILY_RESET',
    message: 'Daily reset completed for Speaksins',
  },
]

/** Recent activity, newest first. */
export function createMockActivity(): ActivityItem[] {
  const now = Date.now()
  return SEEDS.map((s) => ({
    id: nextActivityId(),
    timestampUtc: new Date(now - s.minutesAgo * 60_000).toISOString(),
    kind: s.kind,
    racerId: s.racerId,
    racerName: s.racerName,
    message: s.message,
    code: s.code,
    detail: s.subject?.toUpperCase().replace(/[^A-Z0-9]+/g, '_'),
    subject: s.subject,
  }))
}
