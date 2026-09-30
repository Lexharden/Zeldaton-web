<script setup lang="ts">
import { t } from '@/i18n'
import { HIVESHOCK_CAPABILITIES } from '@/config/event'
import type { CapabilityState } from '@/types/hiveshock'

/** Capabilities are labelled honestly: live, experimental or planned. */
const label: Record<CapabilityState, string> = {
  live: 'hiveshock.active',
  experimental: 'hiveshock.experimental',
  planned: 'hiveshock.planned',
}
const tone: Record<CapabilityState, string> = {
  live: 'text-success bg-success/10 ring-success/30',
  experimental: 'text-warning bg-warning/10 ring-warning/30',
  planned: 'text-muted bg-white/5 ring-white/10',
}
</script>

<template>
  <ul class="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
    <li v-for="(c, i) in HIVESHOCK_CAPABILITIES" :key="c.id" v-reveal="i" class="panel panel-sm">
      <div class="flex h-full flex-col gap-3 p-5">
        <span
          class="w-fit rounded-[3px] px-2 py-0.5 font-mono text-[10px] font-semibold tracking-[0.16em] ring-1 ring-inset"
          :class="tone[c.state]"
          >{{ t(label[c.state]) }}</span
        >
        <h3 class="display text-2xl text-white">{{ t(`hiveshock.cap.${c.id}.title`) }}</h3>
        <p class="text-sm text-muted">{{ t(`hiveshock.cap.${c.id}.description`) }}</p>
      </div>
    </li>
  </ul>
</template>
