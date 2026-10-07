<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, useId } from 'vue'
import { Info } from 'lucide-vue-next'

/**
 * A small "i" that explains what the thing next to it means. Opens on hover, keyboard focus or tap
 * and closes with Escape or a tap elsewhere.
 *
 * The popup is rendered in `<body>` with a fixed position computed from the button, so no
 * `overflow` or stacking context of the page (tables, panels, dialogs) can clip or cover it. It
 * stays inside the screen, goes above the button when there is no room below, and follows scroll
 * and resize while open.
 */
withDefaults(defineProps<{ text: string; label?: string }>(), { label: 'Más información' })

const GAP = 6
const MARGIN = 8

const open = ref(false)
const id = useId()
const trigger = ref<HTMLElement | null>(null)
const tip = ref<HTMLElement | null>(null)
const pos = ref({ top: 0, left: 0 })

function place() {
  const button = trigger.value
  if (!button) return
  const r = button.getBoundingClientRect()
  const w = tip.value?.offsetWidth ?? 240
  const h = tip.value?.offsetHeight ?? 80
  const left = Math.min(
    Math.max(MARGIN, r.left + r.width / 2 - w / 2),
    Math.max(MARGIN, window.innerWidth - w - MARGIN),
  )
  let top = r.bottom + GAP
  if (top + h > window.innerHeight - MARGIN && r.top - GAP - h >= MARGIN) top = r.top - GAP - h
  pos.value = { top, left }
}

function onOutside(e: Event) {
  if (!trigger.value?.contains(e.target as Node)) hide()
}

async function show() {
  if (open.value) return
  open.value = true
  await nextTick() // the popup now exists and can be measured
  place()
  window.addEventListener('scroll', place, true)
  window.addEventListener('resize', place)
  document.addEventListener('pointerdown', onOutside)
}

function hide() {
  open.value = false
  window.removeEventListener('scroll', place, true)
  window.removeEventListener('resize', place)
  document.removeEventListener('pointerdown', onOutside)
}

const toggle = () => (open.value ? hide() : void show())

onBeforeUnmount(hide)
</script>

<template>
  <span class="inline-flex align-middle normal-case tracking-normal">
    <button
      ref="trigger"
      type="button"
      class="rounded-full p-0.5 text-muted hover:text-white focus-visible:text-white"
      :aria-label="label"
      :aria-expanded="open"
      :aria-describedby="open ? id : undefined"
      @mouseenter="show"
      @mouseleave="hide"
      @focus="show"
      @blur="hide"
      @click.prevent="toggle"
      @keydown.esc="hide"
    >
      <Info class="size-3.5" aria-hidden="true" />
    </button>
    <Teleport to="body">
      <span
        v-if="open"
        :id="id"
        ref="tip"
        role="tooltip"
        class="pointer-events-none fixed z-[1000] w-60 max-w-[calc(100vw-1rem)] rounded border border-line bg-surface-elevated p-2.5 text-left text-xs font-normal normal-case leading-snug tracking-normal text-text shadow-lg"
        :style="{ top: `${pos.top}px`, left: `${pos.left}px` }"
        >{{ text }}</span
      >
    </Teleport>
  </span>
</template>
