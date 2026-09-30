import type { ItemDefinition, ObjectiveDefinition } from '@/types/game'
import type { HiveShockCapability } from '@/types/hiveshock'

/** Brand constants. Translatable copy lives in src/i18n. */
export const SITE = { name: 'ZELDATHON' } as const

/**
 * Official event dates. 7 Oct 2026 at 06:00 Mexico City (UTC-6, no DST) = the daily reset time,
 * so every racer starts the race with a full budget. Adjust the hour here if it changes.
 */
export const EVENT_SCHEDULE = {
  startAtUtc: '2026-10-07T12:00:00Z',
  timezone: 'America/Mexico_City',
} as const

/** Main event channels: Twitch and TikTok. Replace with the official account URLs. */
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

/** Ordered race track. Ids are what telemetry reports; labels come from i18n `objectives.<id>`. */
export const OBJECTIVES: ObjectiveDefinition[] = [
  { id: 'kokiri-forest' },
  { id: 'deku-tree' },
  { id: 'dodongos-cavern' },
  { id: 'jabu-jabu' },
  { id: 'forest-temple' },
  { id: 'fire-temple' },
  { id: 'water-temple' },
  { id: 'shadow-temple' },
  { id: 'spirit-temple' },
  { id: 'ganons-castle' },
]

/** Extensible: unknown item ids coming from telemetry are ignored. Labels: i18n `items.<id>`. */
export const ITEMS: ItemDefinition[] = [
  { id: 'master-sword', short: 'ME' },
  { id: 'hookshot', short: 'GA' },
  { id: 'longshot', short: 'GL' },
  { id: 'bow', short: 'AR' },
  { id: 'bombs', short: 'BO' },
  { id: 'boomerang', short: 'BU' },
  { id: 'megaton-hammer', short: 'MM' },
  { id: 'iron-boots', short: 'BH' },
  { id: 'mirror-shield', short: 'EE' },
]

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
