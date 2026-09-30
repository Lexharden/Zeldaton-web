<script setup lang="ts">
import { t } from '@/i18n'
import { computed } from 'vue'
import {
  ArrowDown,
  ArrowRight,
  CalendarClock,
  Gamepad2,
  Hourglass,
  Play,
  Power,
  RotateCcw,
} from 'lucide-vue-next'
import { useEventStore } from '@/stores/event'
import { formatDuration } from '@/utils/format'
import { formatResetClock } from '@/utils/time'

/** Visual loop: reset -> full budget -> play -> time out -> force close -> next reset. */
const event = useEventStore()
const steps = computed(() => [
  {
    icon: CalendarClock,
    label: t('race.resetLocal', {
      time: formatResetClock(event.info?.dailyResetLocalTime ?? '06:00'),
    }),
    tone: 'text-accent',
  },
  {
    icon: Hourglass,
    label: formatDuration(event.budgetSeconds),
    tone: 'text-secondary',
    mono: true,
  },
  { icon: Play, label: t('race.play'), tone: 'text-success' },
  { icon: Power, label: t('race.runsOut'), tone: 'text-warning' },
  { icon: Gamepad2, label: t('race.forcedClose'), tone: 'text-[#ff6b86]' },
  { icon: RotateCcw, label: t('race.nextReset'), tone: 'text-primary' },
])
</script>

<template>
  <ol
    class="grid gap-1 lg:grid-cols-[repeat(11,auto)] lg:items-stretch lg:gap-2"
    :aria-label="t('race.flowAria')"
  >
    <template v-for="(s, i) in steps" :key="s.label">
      <li v-reveal="i" class="panel panel-sm lg:min-w-0">
        <div
          class="flex items-center gap-4 p-4 lg:h-full lg:flex-col lg:justify-center lg:gap-3 lg:px-3 lg:py-6 lg:text-center"
        >
          <span class="num hidden text-[10px] text-muted lg:block">0{{ i + 1 }}</span>
          <component
            :is="s.icon"
            class="size-6 shrink-0 lg:size-8"
            :class="s.tone"
            aria-hidden="true"
          />
          <span
            class="text-sm font-semibold uppercase tracking-wider text-white"
            :class="s.mono ? 'num !text-2xl font-bold' : 'font-display text-lg'"
            >{{ s.label }}</span
          >
        </div>
      </li>
      <li
        v-if="i < steps.length - 1"
        class="grid place-items-center py-0.5 text-primary/70"
        aria-hidden="true"
      >
        <ArrowDown class="size-4 lg:hidden" /><ArrowRight class="hidden size-4 lg:block" />
      </li>
    </template>
  </ol>
</template>
