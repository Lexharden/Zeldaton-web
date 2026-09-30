import type { Racer } from '@/types/racer'
import type { StreamInfo } from '@/types/race'

export function createMockStreams(racers: Racer[]): StreamInfo[] {
  return racers.map((r) => ({
    racerId: r.id,
    isLive: r.stream?.isLive ?? false,
    thumbnailUrl: r.stream?.thumbnailUrl,
    viewers: r.stream?.viewers,
  }))
}
