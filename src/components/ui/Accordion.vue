<script setup lang="ts">
import { ChevronDown } from 'lucide-vue-next'

/** Accessible single-item disclosure. Group several for a FAQ. */
defineProps<{ title: string; id: string }>()
const open = defineModel<boolean>({ default: false })
</script>

<template>
  <div class="panel panel-sm">
    <h3>
      <button
        type="button"
        class="flex w-full items-center justify-between gap-4 px-5 py-4 text-left"
        :aria-expanded="open"
        :aria-controls="`acc-${id}`"
        @click="open = !open"
      >
        <span class="font-display text-xl font-bold uppercase tracking-wide text-white">{{
          title
        }}</span>
        <ChevronDown
          class="size-5 shrink-0 text-primary transition-transform duration-300"
          :class="open && 'rotate-180'"
          aria-hidden="true"
        />
      </button>
    </h3>
    <div
      :id="`acc-${id}`"
      class="grid transition-[grid-template-rows] duration-300"
      :class="open ? 'grid-rows-[1fr]' : 'grid-rows-[0fr]'"
    >
      <div class="overflow-hidden">
        <div class="px-5 pb-5 text-sm leading-relaxed text-muted"><slot /></div>
      </div>
    </div>
  </div>
</template>
