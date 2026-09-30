<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { X } from 'lucide-vue-next'

/** Accessible dialog: Escape and a click outside close it; focus moves into it and returns after. */
const props = defineProps<{ open: boolean; title: string; wide?: boolean }>()
const emit = defineEmits<{ close: [] }>()
const panel = ref<HTMLElement | null>(null)
let previous: HTMLElement | null = null

function onKey(e: KeyboardEvent) {
  if (e.key === 'Escape') emit('close')
}

watch(
  () => props.open,
  async (open) => {
    if (open) {
      previous = document.activeElement as HTMLElement | null
      window.addEventListener('keydown', onKey)
      await nextTick()
      const first = panel.value?.querySelector<HTMLElement>(
        'input:not([disabled]), select, textarea, button:not([data-close])',
      )
      ;(first ?? panel.value)?.focus()
    } else {
      window.removeEventListener('keydown', onKey)
      previous?.focus()
    }
  },
  { immediate: true },
)
onBeforeUnmount(() => window.removeEventListener('keydown', onKey))
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="fixed inset-0 z-[80] flex items-start justify-center overflow-y-auto bg-black/70 p-4 backdrop-blur-sm sm:items-center"
      @mousedown.self="emit('close')"
    >
      <div
        ref="panel"
        role="dialog"
        aria-modal="true"
        :aria-label="title"
        tabindex="-1"
        class="panel my-8 w-full outline-none"
        :class="wide ? 'max-w-3xl' : 'max-w-lg'"
      >
        <div class="p-6">
          <header class="mb-5 flex items-start justify-between gap-4">
            <h2 class="display text-3xl text-white">{{ title }}</h2>
            <button
              type="button"
              data-close
              class="rounded p-1 text-muted hover:text-white"
              aria-label="Cerrar"
              @click="emit('close')"
            >
              <X class="size-5" />
            </button>
          </header>
          <slot />
        </div>
      </div>
    </div>
  </Teleport>
</template>
