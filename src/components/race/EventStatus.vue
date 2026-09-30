<script setup lang="ts">
import { t } from '@/i18n'
import { useEventStore } from '@/stores/event'
import { useCountdown } from '@/composables/useCountdown'
import { eventHeadline } from '@/utils/status'
import { formatDuration } from '@/utils/format'
import LiveBadge from '@/components/common/LiveBadge.vue'

/** Compact global event state: LIVE NOW / EVENT STARTS IN / EVENT COMPLETE. */
const event = useEventStore()
const { totalSeconds } = useCountdown(() => event.startMs, 'down')
const { totalSeconds: elapsed } = useCountdown(() => event.startMs, 'up')
</script>

<template>
  <div class="flex items-center gap-3">
    <LiveBadge
      :status="event.status"
      :label="event.status === 'live' ? t('nav.liveNow') : undefined"
    />
    <span class="hud-label">{{ eventHeadline(event.status).eyebrow }}</span>
    <span v-if="event.status === 'upcoming'" class="num text-sm font-semibold text-white">{{
      formatDuration(totalSeconds)
    }}</span>
    <span v-else-if="event.status === 'live'" class="num text-sm font-semibold text-white">{{
      formatDuration(elapsed)
    }}</span>
  </div>
</template>
