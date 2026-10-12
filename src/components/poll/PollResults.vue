<script setup lang="ts">
import { computed } from 'vue'
import { t } from '@/i18n'
import Avatar from '@/components/common/Avatar.vue'
import type { Poll } from '@/types/poll'
import { formatCount } from '@/utils/format'

/** Bars with each racer's share of the votes; the visitor's own pick is marked. */
const props = defineProps<{ poll: Poll }>()
const rows = computed(() => {
  const byId = new Map(props.poll.options.map((o) => [o.racerId, o]))
  return (props.poll.results ?? []).flatMap((r) => {
    const o = byId.get(r.racerId)
    return o ? [{ ...r, name: o.name, avatarUrl: o.avatarUrl }] : []
  })
})
const top = computed(() => Math.max(1, ...rows.value.map((r) => r.percent)))
</script>

<template>
  <div>
    <ol class="space-y-3">
      <li
        v-for="(r, i) in rows"
        :key="r.racerId"
        class="border border-line bg-white/[0.03] p-3"
        :class="poll.voted === r.racerId && 'border-accent/60 bg-accent/10'"
      >
        <div class="flex items-center gap-3">
          <Avatar :name="r.name" :src="r.avatarUrl" :size="44" />
          <div class="min-w-0 flex-1">
            <div class="flex items-baseline justify-between gap-3">
              <p class="truncate font-display text-lg font-semibold uppercase text-white">
                <span class="mr-2 text-white/50">{{ i + 1 }}</span
                >{{ r.name }}
                <span v-if="poll.voted === r.racerId" class="ml-2 hud-label !text-accent">{{
                  t('poll.yours')
                }}</span>
              </p>
              <p class="num shrink-0 text-white">
                {{ r.percent.toFixed(1) }}%
                <span class="text-xs text-white/60"
                  >· {{ t('poll.votes', { n: formatCount(r.votes) }) }}</span
                >
              </p>
            </div>
            <div class="mt-2 h-2 bg-white/10" aria-hidden="true">
              <div
                class="h-full bg-gradient-to-r from-primary to-accent transition-[width] duration-700"
                :style="{ width: `${(r.percent / top) * 100}%` }"
              />
            </div>
          </div>
        </div>
      </li>
    </ol>
    <p class="mt-4 text-sm text-white/70">
      {{ t('poll.total', { n: formatCount(poll.totalVotes ?? 0) }) }}
    </p>
  </div>
</template>
