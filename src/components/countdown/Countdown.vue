<script setup lang="ts">
import { t } from '@/i18n'
import { computed } from 'vue'
import { useCountdown } from '@/composables/useCountdown'
import { twoDigits } from '@/utils/format'

/** Big DAYS/HOURS/MINUTES/SECONDS HUD countdown (or count-up when `mode="up"`). */
const props = withDefaults(
  defineProps<{ target: number; mode?: 'down' | 'up'; compact?: boolean }>(),
  { mode: 'down' },
)
const { parts } = useCountdown(() => props.target, props.mode)

const cells = computed(() => [
  { key: 'days', label: t('countdown.days'), value: parts.value.days },
  { key: 'hours', label: t('countdown.hours'), value: parts.value.hours },
  { key: 'minutes', label: t('countdown.minutes'), value: parts.value.minutes },
  { key: 'seconds', label: t('countdown.seconds'), value: parts.value.seconds },
])
</script>

<template>
  <div
    class="grid grid-cols-4 items-start gap-x-1.5 sm:gap-x-3"
    role="timer"
    :aria-label="mode === 'down' ? t('countdown.ariaDown') : t('countdown.ariaUp')"
  >
    <template v-for="(cell, i) in cells" :key="cell.key">
      <div class="relative text-center">
        <div class="panel panel-sm" :class="cell.key === 'seconds' && 'panel-gold'">
          <div
            class="num font-bold leading-none text-white"
            :class="compact ? 'py-3 text-3xl' : 'py-4 text-[clamp(2rem,9vw,4.75rem)] sm:py-6'"
          >
            {{ twoDigits(cell.value) }}
          </div>
        </div>
        <span class="hud-label mt-2 block text-[9px] sm:text-[11px]">{{ cell.label }}</span>
        <span
          v-if="i < cells.length - 1"
          class="num pointer-events-none absolute -right-2 top-1/2 hidden -translate-y-[80%] text-2xl text-primary/70 sm:block"
          aria-hidden="true"
          >:</span
        >
      </div>
    </template>
  </div>
</template>
