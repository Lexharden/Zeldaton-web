<script setup lang="ts">
import { t } from '@/i18n'
import { computed } from 'vue'

/** Segmented HUD bar. `value` is 0-100. */
const props = withDefaults(
  defineProps<{
    value: number
    segments?: number
    tone?: 'primary' | 'gold' | 'cyan' | 'danger'
    label?: string
    thin?: boolean
  }>(),
  { segments: 20, tone: 'primary' },
)
const clamped = computed(() => Math.min(100, Math.max(0, props.value)))
const filled = computed(() => Math.round((clamped.value / 100) * props.segments))
const colors = {
  primary: 'bg-primary',
  gold: 'bg-accent',
  cyan: 'bg-secondary',
  danger: 'bg-[#fb923c]',
}
</script>

<template>
  <div
    class="flex gap-[3px]"
    role="progressbar"
    :aria-valuenow="Math.round(clamped)"
    aria-valuemin="0"
    aria-valuemax="100"
    :aria-label="label ?? t('standings.progress')"
  >
    <span
      v-for="i in segments"
      :key="i"
      class="skew-x-[-18deg] flex-1 transition-colors duration-500"
      :class="[
        thin ? 'h-1.5' : 'h-2.5',
        i <= filled ? colors[tone] : 'bg-white/8',
        i === filled && 'shadow-[0_0_10px_currentColor]',
      ]"
    />
  </div>
</template>
