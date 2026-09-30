import type { Racer, RacerChannel, StreamPlatform } from '@/types/racer'

export const PLATFORM_LABEL: Record<StreamPlatform, string> = {
  twitch: 'Twitch',
  tiktok: 'TikTok',
  youtube: 'YouTube',
}

const URL_BUILDERS: Record<StreamPlatform, (handle: string) => string> = {
  twitch: (h) => `https://twitch.tv/${h}`,
  tiktok: (h) => `https://tiktok.com/@${h}`,
  youtube: (h) => `https://youtube.com/@${h}`,
}

/** Builds channel links from handles; platforms without a handle are simply omitted. */
export function buildChannels(handles: Partial<Record<StreamPlatform, string>>): RacerChannel[] {
  return (['twitch', 'tiktok', 'youtube'] as const).flatMap((platform) => {
    const handle = handles[platform]?.replace(/^@/, '').trim()
    return handle ? [{ platform, handle, url: URL_BUILDERS[platform](handle) }] : []
  })
}

/** Channel used for embeds and single "watch" links: Twitch first, then whatever exists. */
export function primaryChannel(racer: Pick<Racer, 'channels'>): RacerChannel | undefined {
  return racer.channels.find((c) => c.platform === 'twitch') ?? racer.channels[0]
}
