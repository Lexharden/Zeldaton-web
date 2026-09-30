<script setup lang="ts">
import { t } from '@/i18n'
import { computed } from 'vue'
import type { EventStatus } from '@/types/event'
import type { RacerStatus } from '@/types/racer'
import { EVENT_STATUS, RACER_STATUS } from '@/utils/status'

/** Status pill with a status light. Pulse is intentionally subtle. */
const props = withDefaults(
  defineProps<{ status: RacerStatus | EventStatus; label?: string; size?: 'sm' | 'md' }>(),
  { size: 'md' },
)

const meta = computed(
  () => ({ ...EVENT_STATUS, ...RACER_STATUS })[props.status as RacerStatus] ?? RACER_STATUS.offline,
)
const tones = {
  live: 'text-[#ff6b86] bg-[#ff4d6d]/12 ring-[#ff4d6d]/40',
  info: 'text-secondary bg-secondary/10 ring-secondary/35',
  warning: 'text-warning bg-warning/10 ring-warning/35',
  danger: 'text-[#fb923c] bg-[#fb923c]/10 ring-[#fb923c]/40',
  gold: 'text-accent bg-accent/10 ring-accent/40',
  muted: 'text-muted bg-white/5 ring-white/12',
} as const
</script>

<template>
  <span
    class="inline-flex items-center gap-1.5 rounded-[3px] font-mono font-semibold tracking-[0.16em] ring-1 ring-inset"
    :class="[
      tones[meta.tone],
      size === 'sm' ? 'px-1.5 py-0.5 text-[10px]' : 'px-2.5 py-1 text-[11px]',
    ]"
    :aria-label="t('status.statusLabel', { status: label ?? t(meta.label) })"
  >
    <span
      class="size-1.5 rounded-full bg-current"
      :class="meta.pulse && 'motion-safe:animate-[pulse-dot_2.2s_ease-in-out_infinite]'"
      aria-hidden="true"
    />
    {{ label ?? t(meta.label) }}
  </span>
</template>
