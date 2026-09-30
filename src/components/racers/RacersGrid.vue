<script setup lang="ts">
import { t } from '@/i18n'
import SkeletonBlock from '@/components/common/SkeletonBlock.vue'
import StateMessage from '@/components/common/StateMessage.vue'
import { useRaceStore } from '@/stores/race'
import { useRacersStore } from '@/stores/racers'
import RacerCard from './RacerCard.vue'

/** All racers, ordered by current rank. Skeletons while loading, error state on failure. */
const racers = useRacersStore()
const race = useRaceStore()
</script>

<template>
  <div>
    <div
      v-if="racers.loading && !racers.loaded"
      class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4"
      aria-busy="true"
      :aria-label="t('standings.loading')"
    >
      <SkeletonBlock v-for="i in 8" :key="i" class="h-[27rem]" />
    </div>
    <StateMessage
      v-else-if="racers.error && !racers.list.length"
      variant="backend"
      :action-label="t('standings.retry')"
      @action="racers.load()"
    />
    <StateMessage v-else-if="!racers.list.length" variant="no-data" />
    <ul v-else class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
      <li v-for="(s, i) in race.standings" :key="s.racerId" v-reveal="i % 4">
        <RacerCard :racer-id="s.racerId" />
      </li>
    </ul>
  </div>
</template>
