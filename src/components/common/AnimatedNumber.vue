<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from 'vue'
import { formatCount } from '@/utils/format'

/** Counts smoothly from the previous value to the new one. Respects reduced motion. */
const props = withDefaults(defineProps<{ value: number; duration?: number }>(), { duration: 900 })
const shown = ref(props.value)
let frame = 0

function animateTo(target: number) {
  cancelAnimationFrame(frame)
  const reduce = window.matchMedia('(prefers-reduced-motion: reduce)').matches
  if (reduce || props.duration <= 0) {
    shown.value = target
    return
  }
  const from = shown.value
  const start = performance.now()
  const step = (t: number) => {
    const p = Math.min(1, (t - start) / props.duration)
    shown.value = from + (target - from) * (1 - Math.pow(1 - p, 3))
    if (p < 1) frame = requestAnimationFrame(step)
  }
  frame = requestAnimationFrame(step)
}

watch(() => props.value, animateTo)
onBeforeUnmount(() => cancelAnimationFrame(frame))
</script>

<template>
  <span class="num">{{ formatCount(shown) }}</span>
</template>
