<script setup lang="ts">
import { FlaskConical } from 'lucide-vue-next'
import { t } from '@/i18n'
import { useEventStore } from '@/stores/event'

/** Shown on every public page while the organizer has the event in rehearsal mode. */
const event = useEventStore()
</script>

<template>
  <div
    v-if="event.info?.rehearsal"
    role="status"
    class="pointer-events-none fixed inset-x-0 bottom-0 z-[70] flex justify-center px-3 pb-3"
  >
    <p
      class="pointer-events-auto flex max-w-xl items-center gap-2 border border-warning/60 bg-black/85 px-3 py-2 text-xs text-secondary shadow-lg backdrop-blur-sm"
    >
      <FlaskConical class="size-4 shrink-0 text-warning" aria-hidden="true" />
      <span
        ><strong class="hud-label mr-1.5 text-warning">{{ t('rehearsal.banner') }}</strong
        >{{ t('rehearsal.text') }}</span
      >
    </p>
  </div>
</template>
