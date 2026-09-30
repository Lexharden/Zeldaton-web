import { DEFAULT_CATALOG } from '@/config/catalog'
import type { ItemDefinition, ObjectiveDefinition } from '@/types/game'
import type { HiveShockCapability } from '@/types/hiveshock'

/** Brand constants. Translatable copy lives in src/i18n. */
export const SITE = { name: 'ZELDATÓN' } as const

/**
 * Official event dates. 7 Oct 2026 at 06:00 Mexico City (UTC-6, no DST) = the daily reset time,
 * so every racer starts the race with a full budget. Adjust the hour here if it changes.
 */
export const EVENT_SCHEDULE = {
  startAtUtc: '2026-10-07T12:00:00Z',
  timezone: 'America/Mexico_City',
} as const

/** Main event channels: Twitch and TikTok. Replace with the official account URLs. */
/** Who built this site (footer credit, meta author). HiveShock is the technology behind it. */
export const DEVELOPER = { name: 'Yafel GH', url: 'https://www.instagram.com/yaafel/' } as const

export const SOCIAL_LINKS = [
  { id: 'twitch', label: 'Twitch', href: 'https://twitch.tv' },
  { id: 'tiktok', label: 'TikTok', href: 'https://tiktok.com' },
  { id: 'discord', label: 'Discord', href: 'https://discord.com' },
] as const

/** Timezones highlighted in the "Your time. Your timezone." section. */
export const SHOWCASE_TIMEZONES = [
  'America/Mexico_City',
  'Europe/Madrid',
  'America/Argentina/Buenos_Aires',
  'America/New_York',
] as const

/** Ordered race track (factory catalog). Labels come from the catalog store; `objectives.<id>` is the fallback. */
export const OBJECTIVES: ObjectiveDefinition[] = DEFAULT_CATALOG.objectives.map((o) => ({
  id: o.id,
}))

/** Factory items. The live list comes from the catalog store; this feeds mock data and fallbacks. */
export const ITEMS: ItemDefinition[] = DEFAULT_CATALOG.items.map((i) => ({
  id: i.id,
  short: i.short,
}))

/** Capability copy: i18n `hiveshock.cap.<id>`. Experimental features are labelled honestly. */
export const HIVESHOCK_CAPABILITIES: HiveShockCapability[] = [
  { id: 'events', state: 'live' },
  { id: 'telemetry', state: 'live' },
  { id: 'web', state: 'live' },
  { id: 'chat', state: 'experimental' },
  { id: 'interaction', state: 'experimental' },
]

/** Copy: i18n `how.steps.<id>`. */
export const RACE_STEPS = [
  { id: 'connect' },
  { id: 'play' },
  { id: 'track' },
  { id: 'race' },
  { id: 'finish' },
] as const
