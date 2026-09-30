<script setup lang="ts">
import { computed } from 'vue'
import { formatDuration } from '@/utils/format'

/** HH:MM:SS readout in the HUD monospace font. */
const props = withDefaults(
  defineProps<{
    seconds: number
    size?: 'sm' | 'md' | 'lg' | 'xl'
    tone?: 'default' | 'gold' | 'danger' | 'muted'
  }>(),
  { size: 'md', tone: 'default' },
)
const text = computed(() => formatDuration(props.seconds))
const sizes = {
  sm: 'text-base',
  md: 'text-2xl',
  lg: 'text-4xl sm:text-5xl',
  xl: 'text-5xl sm:text-7xl',
}
const tones = {
  default: 'text-white',
  gold: 'text-accent',
  danger: 'text-[#fb923c]',
  muted: 'text-muted',
}
</script>

<template>
  <span class="num font-bold leading-none" :class="[sizes[size], tones[tone]]">{{ text }}</span>
</template>
