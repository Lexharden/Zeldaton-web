import { t } from '@/i18n'
import type { EventStatus } from '@/types/event'
import type { RacerStatus } from '@/types/racer'

export interface StatusMeta {
  /** Translation key under `status.` */
  label: string
  tone: 'live' | 'muted' | 'warning' | 'danger' | 'gold' | 'info'
  pulse: boolean
}

export const RACER_STATUS: Record<RacerStatus, StatusMeta> = {
  live: { label: 'status.live', tone: 'live', pulse: true },
  online: { label: 'status.online', tone: 'info', pulse: false },
  paused: { label: 'status.paused', tone: 'warning', pulse: false },
  offline: { label: 'status.offline', tone: 'muted', pulse: false },
  exhausted: { label: 'status.exhausted', tone: 'danger', pulse: false },
  finished: { label: 'status.finished', tone: 'gold', pulse: false },
}

export const EVENT_STATUS: Record<EventStatus, StatusMeta> = {
  upcoming: { label: 'status.upcoming', tone: 'info', pulse: false },
  live: { label: 'status.live', tone: 'live', pulse: true },
  paused: { label: 'status.paused', tone: 'warning', pulse: false },
  finished: { label: 'status.finished', tone: 'gold', pulse: false },
}

export interface EventHeadline {
  eyebrow: string
  cta: string
}

/** Maps global event state to hero copy. */
export function eventHeadline(status: EventStatus): EventHeadline {
  switch (status) {
    case 'upcoming':
      return { eyebrow: t('event.upcomingEyebrow'), cta: t('event.ctaUpcoming') }
    case 'live':
      return { eyebrow: t('event.liveEyebrow'), cta: t('event.ctaLive') }
    case 'paused':
      return { eyebrow: t('event.pausedEyebrow'), cta: t('event.ctaUpcoming') }
    case 'finished':
      return { eyebrow: t('event.finishedEyebrow'), cta: t('event.ctaFinished') }
  }
}

/** Position (0..1) of a racer on the track, driven by objectives when available. */
export function trackPosition(
  completed: number,
  totalObjectives: number,
  percentage: number,
): number {
  if (totalObjectives > 0 && completed > 0) return Math.min(1, completed / totalObjectives)
  return Math.min(1, Math.max(0, percentage / 100))
}
