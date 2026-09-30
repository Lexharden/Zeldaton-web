import { OBJECTIVES } from '@/config/event'
import type { Racer, RacerChannel, RacerStatus } from '@/types/racer'
import { MOCK_AREAS } from './areas'

const BUDGET = 4 * 3600

interface Seed {
  id: string
  name: string
  timezone: string
  country: string
  status: RacerStatus
  /** Race progress, 0-100. Completed objectives follow it (one per 10 %). */
  progress: number
  usedSeconds: number
  items: string[]
  channels: Array<'twitch' | 'tiktok'>
  viewers?: number
  age?: 'child' | 'adult'
}

const SEEDS: Seed[] = [
  {
    id: 'ralbat',
    name: 'Ralbat',
    timezone: 'America/Mexico_City',
    country: 'MX',
    status: 'live',
    progress: 76,
    usedSeconds: 7_800,
    items: [
      'master-sword',
      'hookshot',
      'longshot',
      'bow',
      'bombs',
      'iron-boots',
      'megaton-hammer',
      'kokiri-sword',
      'deku-shield',
      'boomerang',
      'fire-arrows',
      'hover-boots',
      'goron-tunic',
      'zora-tunic',
      'forest-medallion',
      'fire-medallion',
      'water-medallion',
      'kokiri-emerald',
      'goron-ruby',
      'zora-sapphire',
      'ocarina-of-time',
      'zeldas-lullaby',
      'sarias-song',
      'song-of-time',
      'minuet-of-forest',
      'bolero-of-fire',
      'serenade-of-water',
      'bottle',
      'lens-of-truth',
      'dins-fire',
    ],
    channels: ['twitch', 'tiktok'],
    viewers: 1240,
  },
  {
    id: 'pinchiviejo',
    name: 'Pinchiviejo',
    timezone: 'America/Argentina/Buenos_Aires',
    country: 'AR',
    status: 'live',
    progress: 64,
    usedSeconds: 6_900,
    items: ['master-sword', 'hookshot', 'bow', 'bombs', 'boomerang'],
    channels: ['twitch'],
    viewers: 860,
  },
  {
    id: 'xime',
    name: 'Xime',
    timezone: 'Europe/Madrid',
    country: 'ES',
    status: 'live',
    progress: 52,
    usedSeconds: 8_400,
    items: ['master-sword', 'hookshot', 'bombs', 'boomerang'],
    channels: ['tiktok'],
    viewers: 512,
  },
  {
    id: 'cuaco',
    name: 'Cuaco',
    timezone: 'America/Mexico_City',
    country: 'MX',
    status: 'paused',
    progress: 41,
    usedSeconds: 5_100,
    items: ['master-sword', 'bow', 'bombs'],
    channels: ['twitch', 'tiktok'],
    viewers: 230,
  },
  {
    id: 'kahrilys',
    name: 'Kahrilys',
    timezone: 'America/New_York',
    country: 'US',
    status: 'live',
    progress: 33,
    usedSeconds: 4_200,
    items: ['master-sword', 'bombs', 'boomerang'],
    channels: ['twitch'],
    viewers: 175,
  },
  {
    id: 'pupperina',
    name: 'Pupperina',
    timezone: 'America/Sao_Paulo',
    country: 'BR',
    status: 'online',
    progress: 27,
    usedSeconds: 3_000,
    items: ['master-sword', 'bombs'],
    channels: ['tiktok'],
  },
  {
    id: 'speaksins',
    name: 'Speaksins',
    timezone: 'Europe/London',
    country: 'GB',
    status: 'live',
    progress: 18,
    usedSeconds: 2_600,
    items: ['bombs'],
    channels: ['twitch'],
    viewers: 96,
  },
  {
    id: 'perlarev',
    name: 'Perlarev',
    timezone: 'Asia/Tokyo',
    country: 'JP',
    status: 'exhausted',
    progress: 12,
    usedSeconds: BUDGET,
    items: ['master-sword'],
    channels: ['twitch'],
  },
  {
    id: 'insomnia',
    name: 'Insomnia',
    timezone: 'America/Los_Angeles',
    country: 'US',
    status: 'offline',
    progress: 0,
    usedSeconds: 0,
    items: [],
    channels: [],
  },
]

const URLS = { twitch: 'https://twitch.tv/', tiktok: 'https://tiktok.com/@' } as const

function channelsFor(seed: Seed): RacerChannel[] {
  return seed.channels.map((platform) => ({
    platform,
    handle: seed.id,
    url: `${URLS[platform]}${seed.id}`,
  }))
}

/** Nine simulated racers in different states, with coherent progress, items and clocks. */
export function createMockRacers(): Racer[] {
  return SEEDS.map((seed): Racer => {
    const done = Math.min(OBJECTIVES.length, Math.floor(seed.progress / 10))
    const playing = seed.status === 'live' || seed.status === 'paused'
    const remaining = Math.max(0, BUDGET - seed.usedSeconds)
    const hearts = 3 + Math.min(17, done * 2)
    return {
      id: seed.id,
      displayName: seed.name,
      slug: seed.id,
      country: seed.country,
      timezone: seed.timezone,
      status: seed.status,
      elapsedSeconds: BUDGET - remaining,
      remainingSeconds: remaining,
      progressPercentage: seed.progress,
      currentArea:
        seed.progress > 0 ? MOCK_AREAS[Math.min(done, MOCK_AREAS.length - 1)] : undefined,
      currentObjective: OBJECTIVES[done]?.id,
      completedObjectives: OBJECTIVES.slice(0, done).map((o) => o.id),
      channels: channelsFor(seed),
      stream: {
        isLive: playing && seed.channels.length > 0,
        viewers: playing ? seed.viewers : undefined,
      },
      stats: {
        hearts: Math.max(3, hearts - 1),
        age: seed.age,
        maxHearts: hearts,
        rupees: done * 40,
        skulltulas: done * 2,
        bossesDefeated: Math.min(6, Math.floor(done / 1.5)),
      },
      items: Object.fromEntries(seed.items.map((id) => [id, true])),
    }
  })
}
