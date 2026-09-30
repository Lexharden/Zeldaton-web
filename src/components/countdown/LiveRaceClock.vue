<script setup lang="ts">
import { t } from '@/i18n'
import { computed } from 'vue'
import { useRaceClock } from '@/composables/useRaceClock'
import { useEventStore } from '@/stores/event'
import { formatResetTime } from '@/utils/time'
import LiveBadge from '@/components/common/LiveBadge.vue'
import ProgressBar from '@/components/common/ProgressBar.vue'
import ClockDisplay from './ClockDisplay.vue'

/**
 * Daily play clock for one racer.
 * Data: racerId -> clock snapshot from the racers store (server-authoritative, interpolated).
 * States: LIVE (counting), PAUSED (held), EXHAUSTED (time expired), FINISHED, OFFLINE/UPCOMING.
 */
const props = defineProps<{ racerId: string }>()
const event = useEventStore()
const clock = useRaceClock(() => props.racerId)

const view = computed(() => {
  switch (clock.status.value) {
    case 'live':
      return { note: t('racerPage.clockRunning'), tone: 'default' as const }
    case 'paused':
      return { note: t('racerPage.clockPaused'), tone: 'muted' as const }
    case 'exhausted':
      return { note: t('racerPage.forcedClose'), tone: 'danger' as const }
    case 'finished':
      return { note: t('racerPage.raceComplete'), tone: 'gold' as const }
    case 'online':
      return { note: t('racerPage.readyToPlay'), tone: 'default' as const }
    default:
      return { note: t('racerPage.offlineHeld'), tone: 'muted' as const }
  }
})
const reset = computed(() => formatResetTime(event.info?.dailyResetLocalTime ?? '06:00'))
</script>

<template>
  <section class="panel panel-lg" :aria-label="t('racerPage.clockAria')">
    <div class="p-6 sm:p-8">
      <div class="mb-6 flex items-center justify-between gap-3">
        <span class="hud-label">{{ t('racerPage.daily') }}</span>
        <LiveBadge :status="clock.status.value" />
      </div>

      <template v-if="clock.status.value === 'exhausted'">
        <p class="display text-2xl text-[#fb923c] sm:text-3xl">{{ t('racerPage.timeExpired') }}</p>
        <div class="my-2"><ClockDisplay :seconds="0" size="xl" tone="danger" /></div>
      </template>
      <div v-else class="my-2">
        <ClockDisplay :seconds="clock.remainingSeconds.value" size="xl" :tone="view.tone" />
      </div>

      <p
        class="mt-3 font-mono text-xs uppercase tracking-widest"
        :class="clock.status.value === 'exhausted' ? 'text-[#fb923c]' : 'text-muted'"
      >
        {{ view.note }}
      </p>
      <div class="mt-6">
        <ProgressBar
          :value="clock.usedRatio.value * 100"
          tone="cyan"
          :segments="30"
          :label="t('racers.dailyTimeUsed')"
          thin
        />
        <div class="mt-2 flex justify-between font-mono text-[11px] text-muted">
          <span>{{
            t('racerPage.usedPct', { pct: Math.round(clock.usedRatio.value * 100) })
          }}</span>
          <span>{{ t('racerPage.nextResetShort', { time: reset }) }}</span>
        </div>
      </div>
    </div>
  </section>
</template>
