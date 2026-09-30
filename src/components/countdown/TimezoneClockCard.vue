<script setup lang="ts">
import { t } from '@/i18n'
import { computed } from 'vue'
import { Moon, Sun } from 'lucide-vue-next'
import { useNow } from '@/composables/useNow'
import { formatDuration } from '@/utils/format'
import {
  formatLocalTime,
  formatResetTime,
  formatTimezone,
  isDaytime,
  nextResetUtc,
} from '@/utils/time'

/**
 * One timezone: current local time, the reset wall-clock time and how long until that
 * zone's own reset. Shows that the same "06:00" lands at different real moments.
 * Props: IANA `timezone` + `resetLocalTime` from event config (never hardcoded here).
 */
const props = defineProps<{ timezone: string; resetLocalTime: string }>()
const { now } = useNow()

const localTime = computed(() => formatLocalTime(now.value, props.timezone, { seconds: true }))
const resetIn = computed(() =>
  formatDuration(
    (nextResetUtc(now.value, props.timezone, props.resetLocalTime) - now.value) / 1000,
  ),
)
const day = computed(() => isDaytime(now.value, props.timezone))
</script>

<template>
  <article class="panel panel-hover panel-sm h-full">
    <div class="flex h-full flex-col gap-5 p-5">
      <div class="flex items-start justify-between gap-2">
        <div>
          <h3 class="display text-3xl text-white">{{ formatTimezone(timezone) }}</h3>
          <p class="mt-1 font-mono text-[10px] tracking-wider text-muted">{{ timezone }}</p>
        </div>
        <component
          :is="day ? Sun : Moon"
          class="size-5"
          :class="day ? 'text-accent' : 'text-primary'"
          aria-hidden="true"
        />
      </div>
      <div>
        <p class="hud-label">{{ t('daily.localNow') }}</p>
        <p class="num mt-1 text-3xl font-bold text-white">{{ localTime }}</p>
      </div>
      <div class="mt-auto grid grid-cols-2 gap-3 border-t border-line pt-4">
        <div>
          <p class="hud-label text-[10px]">{{ t('daily.resetsAt') }}</p>
          <p class="num mt-1 text-sm font-semibold text-accent">
            {{ formatResetTime(resetLocalTime) }}
          </p>
        </div>
        <div>
          <p class="hud-label text-[10px]">{{ t('daily.resetIn') }}</p>
          <p class="num mt-1 text-sm font-semibold text-secondary">{{ resetIn }}</p>
        </div>
      </div>
    </div>
  </article>
</template>
