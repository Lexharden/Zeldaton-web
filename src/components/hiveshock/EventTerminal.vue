<script setup lang="ts">
import { t } from '@/i18n'
import { computed } from 'vue'
import { useHiveShockStore } from '@/stores/hiveshock'
import { formatLocalTime } from '@/utils/time'

/** Terminal-style LIVE EVENT FEED: [time] RACER / CODE / DETAIL. Timestamps in UTC. */
const props = withDefaults(defineProps<{ limit?: number }>(), { limit: 7 })
const hive = useHiveShockStore()
const rows = computed(() => hive.activity.slice(0, props.limit))
</script>

<template>
  <div
    class="overflow-hidden rounded-md bg-[#03080a] ring-1 ring-secondary/25 shadow-glow-secondary"
    role="log"
    :aria-label="t('activity.eventFeed')"
  >
    <div class="flex items-center gap-2 border-b border-secondary/15 bg-white/[0.03] px-4 py-2.5">
      <span class="size-2.5 rounded-full bg-[#ff4d6d]/80" /><span
        class="size-2.5 rounded-full bg-warning/80"
      /><span class="size-2.5 rounded-full bg-success/80" />
      <span class="hud-label ml-2 !text-secondary">{{ t('activity.terminal') }}</span>
      <span class="ml-auto font-mono text-[10px] text-muted">UTC</span>
    </div>
    <TransitionGroup name="feed" tag="ol" class="space-y-2.5 p-4 font-mono text-xs leading-relaxed">
      <li
        v-for="e in rows"
        :key="e.id"
        class="grid grid-cols-[auto_1fr] gap-x-3 sm:flex sm:flex-wrap sm:gap-x-3"
      >
        <span class="text-muted"
          >[{{ formatLocalTime(e.timestampUtc, 'UTC', { seconds: true }) }}]</span
        >
        <span class="font-semibold text-white">{{
          (e.racerName ?? t('activity.system')).toUpperCase()
        }}</span>
        <span class="col-start-2 text-secondary">{{ e.code }}</span>
        <span v-if="e.detail" class="col-start-2 text-accent">{{ e.detail }}</span>
      </li>
      <li v-if="!rows.length" key="empty" class="text-muted">
        $ {{ t('activity.waiting') }}<span class="motion-safe:animate-pulse">_</span>
      </li>
    </TransitionGroup>
  </div>
</template>
