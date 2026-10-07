<script setup lang="ts">
import { ref, useId } from 'vue'
import { Info } from 'lucide-vue-next'

/**
 * A small "i" that explains what the thing next to it means. Opens on hover, keyboard focus or tap
 * and closes with Escape. `align` keeps the popup inside the screen near the edges.
 */
withDefaults(defineProps<{ text: string; label?: string; align?: 'start' | 'center' | 'end' }>(), {
  label: 'Más información',
  align: 'start',
})

const open = ref(false)
const id = useId()
</script>

<template>
  <span
    class="relative inline-flex align-middle normal-case tracking-normal"
    @mouseenter="open = true"
    @mouseleave="open = false"
  >
    <button
      type="button"
      class="rounded-full p-0.5 text-muted hover:text-white focus-visible:text-white"
      :aria-label="label"
      :aria-expanded="open"
      :aria-describedby="open ? id : undefined"
      @focus="open = true"
      @blur="open = false"
      @click.prevent="open = !open"
      @keydown.esc="open = false"
    >
      <Info class="size-3.5" aria-hidden="true" />
    </button>
    <span
      v-if="open"
      :id="id"
      role="tooltip"
      class="absolute top-full z-30 mt-1 w-60 rounded border border-line bg-surface-elevated p-2.5 text-left text-xs font-normal leading-snug text-text shadow-lg"
      :class="{
        'left-0': align === 'start',
        'left-1/2 -translate-x-1/2': align === 'center',
        'right-0': align === 'end',
      }"
      >{{ text }}</span
    >
  </span>
</template>
