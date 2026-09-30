<script setup lang="ts">
import { t } from '@/i18n'
import { computed } from 'vue'
import SkeletonBlock from '@/components/common/SkeletonBlock.vue'
import StateMessage from '@/components/common/StateMessage.vue'
import { useRaceStore } from '@/stores/race'
import { useRacersStore } from '@/stores/racers'
import StreamCard from './StreamCard.vue'

/** Grid of stream cards. Live streams first; `limit` for teasers. Set `liveOnly` to filter. */
const props = defineProps<{ limit?: number; liveOnly?: boolean }>()
const race = useRaceStore()
const racers = useRacersStore()

const ids = computed(() => {
  const live = new Set(race.streams.filter((s) => s.isLive).map((s) => s.racerId))
  const ordered = [...race.standings].sort(
    (a, b) => Number(live.has(b.racerId)) - Number(live.has(a.racerId)) || a.rank - b.rank,
  )
  const filtered = props.liveOnly ? ordered.filter((s) => live.has(s.racerId)) : ordered
  return (props.limit ? filtered.slice(0, props.limit) : filtered).map((s) => s.racerId)
})
</script>

<template>
  <div>
    <div
      v-if="(racers.loading || race.loading) && !ids.length"
      class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4"
      aria-busy="true"
      :aria-label="t('streams.loading')"
    >
      <SkeletonBlock v-for="i in limit ?? 8" :key="i" class="h-[24rem]" />
    </div>
    <StateMessage v-else-if="race.error && !ids.length" variant="backend" />
    <StateMessage
      v-else-if="!ids.length"
      variant="no-data"
      :title="t('streams.noneTitle')"
      :text="t('streams.noneText')"
    />
    <ul v-else class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
      <li v-for="(id, i) in ids" :key="id" v-reveal="i % 4"><StreamCard :racer-id="id" /></li>
    </ul>
  </div>
</template>
