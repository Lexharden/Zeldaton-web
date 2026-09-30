<script setup lang="ts">
import { useCountdown } from '@/composables/useCountdown'
import { useEventStore } from '@/stores/event'
import { formatDuration } from '@/utils/format'
import LiveBadge from '@/components/common/LiveBadge.vue'
import ConnectionIndicator from '@/components/common/ConnectionIndicator.vue'

/** Slim context strip on inner pages: event name, game, status and race clock. */
const event = useEventStore()
const { totalSeconds: elapsed } = useCountdown(() => event.startMs, 'up')
const { totalSeconds: until } = useCountdown(() => event.startMs, 'down')
</script>

<template>
  <div class="border-b border-line bg-surface/70 pt-16">
    <div class="container-x flex flex-wrap items-center gap-x-6 gap-y-2 py-2.5">
      <span class="display text-lg tracking-wider text-white">ZELDATHON</span>
      <span class="hud-label hidden sm:inline">{{
        event.info?.game?.toUpperCase() ?? 'OCARINA OF TIME'
      }}</span>
      <LiveBadge :status="event.status" size="sm" />
      <span v-if="event.status === 'live'" class="num text-sm font-semibold text-white">{{
        formatDuration(elapsed)
      }}</span>
      <span v-else-if="event.status === 'upcoming'" class="num text-sm font-semibold text-white">{{
        formatDuration(until)
      }}</span>
      <span class="ml-auto hidden md:block"><ConnectionIndicator /></span>
    </div>
  </div>
</template>
