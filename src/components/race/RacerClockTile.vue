<script setup lang="ts">
import { t } from '@/i18n'
import { RouterLink } from 'vue-router'
import LiveBadge from '@/components/common/LiveBadge.vue'
import ProgressBar from '@/components/common/ProgressBar.vue'
import ClockDisplay from '@/components/countdown/ClockDisplay.vue'
import { useRaceClock } from '@/composables/useRaceClock'
import { useRacer } from '@/composables/useRacer'
import { formatTimezone } from '@/utils/time'

/** Compact clock for the race dashboard's clock wall. */
const props = defineProps<{ racerId: string }>()
const { racer } = useRacer(() => props.racerId)
const clock = useRaceClock(() => props.racerId)
</script>

<template>
  <RouterLink v-if="racer" :to="`/racer/${racer.id}`" class="panel panel-sm panel-hover block">
    <div class="space-y-3 p-4">
      <div class="flex items-center justify-between gap-2">
        <span class="truncate font-display text-lg font-bold uppercase text-white">{{
          racer.displayName
        }}</span>
        <LiveBadge :status="clock.status.value" size="sm" />
      </div>
      <ClockDisplay
        :seconds="clock.remainingSeconds.value"
        size="lg"
        :tone="
          clock.status.value === 'exhausted'
            ? 'danger'
            : clock.status.value === 'live'
              ? 'default'
              : 'muted'
        "
      />
      <ProgressBar
        :value="clock.usedRatio.value * 100"
        tone="cyan"
        thin
        :segments="24"
        :label="t('racers.dailyTimeUsed')"
      />
      <p class="font-mono text-[10px] uppercase tracking-wider text-muted">
        {{ formatTimezone(racer.timezone) }}
      </p>
    </div>
  </RouterLink>
</template>
