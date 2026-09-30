<script setup lang="ts">
import { t } from '@/i18n'
import { computed } from 'vue'
import SkeletonBlock from '@/components/common/SkeletonBlock.vue'
import StateMessage from '@/components/common/StateMessage.vue'
import { useRaceStore } from '@/stores/race'
import { useRacersStore } from '@/stores/racers'
import StandingRow from './StandingRow.vue'

/**
 * Live leaderboard. Ranking comes from raceStore.standings (derived from racer progress).
 * TransitionGroup animates position changes; the list is never replaced wholesale.
 * `limit` renders a compact top-N for the home page.
 */
const props = defineProps<{ limit?: number }>()
const race = useRaceStore()
const racers = useRacersStore()
const rows = computed(() => (props.limit ? race.standings.slice(0, props.limit) : race.standings))
</script>

<template>
  <div>
    <div
      class="mb-2 hidden grid-cols-[52px_minmax(150px,1.3fr)_minmax(150px,1.2fr)_minmax(110px,1fr)_92px_92px_104px] gap-4 px-4 md:grid"
    >
      <span class="hud-label">{{ t('standings.rank') }}</span>
      <span class="hud-label">{{ t('standings.racer') }}</span>
      <span class="hud-label">{{ t('standings.progress') }}</span>
      <span class="hud-label">{{ t('standings.location') }}</span>
      <span class="hud-label">{{ t('standings.timeUsed') }}</span>
      <span class="hud-label">{{ t('standings.timeLeft') }}</span>
      <span class="hud-label">{{ t('standings.status') }}</span>
    </div>

    <div
      v-if="racers.loading && !racers.loaded"
      class="space-y-2"
      aria-busy="true"
      :aria-label="t('standings.loading')"
    >
      <SkeletonBlock v-for="i in limit ?? 8" :key="i" class="h-[62px]" />
    </div>
    <StateMessage
      v-else-if="racers.error && !racers.list.length"
      variant="backend"
      :action-label="t('standings.retry')"
      @action="racers.load()"
    />
    <StateMessage v-else-if="!rows.length" variant="no-data" />
    <TransitionGroup
      v-else
      name="rank"
      tag="ol"
      class="relative space-y-2"
      :aria-label="t('standings.aria')"
    >
      <StandingRow
        v-for="entry in rows"
        :key="entry.racerId"
        :racer-id="entry.racerId"
        :rank="entry.rank"
      />
    </TransitionGroup>
  </div>
</template>
