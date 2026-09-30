<script setup lang="ts">
import { t } from '@/i18n'
import { tx } from '@/i18n'
import { computed } from 'vue'
import SkeletonBlock from '@/components/common/SkeletonBlock.vue'
import StateMessage from '@/components/common/StateMessage.vue'
import type { ActivityItem } from '@/types/race'
import { useHiveShockStore } from '@/stores/hiveshock'
import { formatLocalTime } from '@/utils/time'

/** Friendly "Live Activity" list. Fed by LIVE_ACTIVITY messages through the store. */
const props = withDefaults(defineProps<{ limit?: number }>(), { limit: 8 })
const hive = useHiveShockStore()
const items = computed(() => hive.activity.slice(0, props.limit))
function describe(a: ActivityItem): string {
  const key = `activity.${a.code}`
  const text = t(key, { racer: a.racerName ?? '', subject: subjectLabel(a) })
  return text === key ? a.message : text
}
function subjectLabel(a: ActivityItem): string {
  if (!a.subject) return ''
  const ns = a.code === 'ITEM_ACQUIRED' ? 'items' : a.code === 'BOSS_DEFEATED' ? 'bosses' : 'areas'
  return tx(ns, a.subject)
}
const dot: Record<string, string> = {
  area: 'bg-secondary',
  item: 'bg-accent',
  boss: 'bg-magenta',
  status: 'bg-warning',
  reset: 'bg-success',
  finish: 'bg-accent',
  system: 'bg-primary',
}
</script>

<template>
  <div class="panel">
    <div class="p-5">
      <div v-if="hive.loading && !items.length" class="space-y-3" aria-busy="true">
        <SkeletonBlock v-for="i in 5" :key="i" class="h-6" />
      </div>
      <StateMessage v-else-if="hive.error && !items.length" variant="backend" />
      <StateMessage v-else-if="!items.length" variant="no-data" />
      <TransitionGroup
        v-else
        name="feed"
        tag="ul"
        class="space-y-3"
        aria-live="polite"
        :aria-label="t('activity.aria')"
      >
        <li v-for="a in items" :key="a.id" class="flex items-start gap-3 text-sm">
          <span
            class="mt-1.5 size-2 shrink-0 rounded-full"
            :class="dot[a.kind] ?? 'bg-primary'"
            aria-hidden="true"
          />
          <span class="flex-1 text-white/90">{{ describe(a) }}</span>
          <time class="num shrink-0 text-[11px] text-muted" :datetime="a.timestampUtc">{{
            formatLocalTime(a.timestampUtc, 'UTC')
          }}</time>
        </li>
      </TransitionGroup>
    </div>
  </div>
</template>
