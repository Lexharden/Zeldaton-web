<script setup lang="ts">
import { t } from '@/i18n'
import { splitLastWord } from '@/utils/template'
import { computed } from 'vue'
import ConnectionIndicator from '@/components/common/ConnectionIndicator.vue'
import GamingBackground from '@/components/common/GamingBackground.vue'
import StateMessage from '@/components/common/StateMessage.vue'
import SkeletonBlock from '@/components/common/SkeletonBlock.vue'
import ActivityFeed from '@/components/hiveshock/ActivityFeed.vue'
import EventTerminal from '@/components/hiveshock/EventTerminal.vue'
import HiveShockStats from '@/components/hiveshock/HiveShockStats.vue'
import RaceTrack from '@/components/race/RaceTrack.vue'
import RacerClockTile from '@/components/race/RacerClockTile.vue'
import StandingsTable from '@/components/standings/StandingsTable.vue'
import StreamGrid from '@/components/streams/StreamGrid.vue'
import Button from '@/components/ui/Button.vue'
import { useSeo } from '@/composables/useSeo'
import { useWebSocket } from '@/composables/useWebSocket'
import { useRaceStore } from '@/stores/race'
import { useRacersStore } from '@/stores/racers'

/**
 * Live broadcast dashboard: the screen to keep open during the event.
 * Priority: status -> standings -> progress -> clocks -> streams -> activity -> telemetry.
 */
useSeo(
  () => t('meta.pages.race'),
  () => t('meta.pages.raceDesc'),
)
const race = useRaceStore()
const racers = useRacersStore()
const { status, reconnect } = useWebSocket()
const showBanner = computed(
  () =>
    status.value === 'reconnecting' || status.value === 'error' || status.value === 'disconnected',
)
</script>

<template>
  <div class="relative pb-20">
    <GamingBackground shapes />
    <div class="container-x pt-8 sm:pt-10">
      <!-- 1. status -->
      <div class="mb-8 flex flex-wrap items-end justify-between gap-4 border-b border-line pb-6">
        <div>
          <p class="hud-label mb-2">{{ t('game.broadcast') }}</p>
          <h1 class="display text-[clamp(2.8rem,8vw,5.5rem)] text-white">
            {{ splitLastWord(t('game.liveRace'))[0] }}
            <span class="text-gradient">{{ splitLastWord(t('game.liveRace'))[1] }}</span>
          </h1>
        </div>
        <div class="space-y-3 text-right">
          <div class="flex justify-end md:hidden"><ConnectionIndicator /></div>
        </div>
      </div>

      <div v-if="showBanner" class="mb-8">
        <StateMessage
          :variant="status === 'reconnecting' ? 'connection-lost' : 'backend'"
          :title="
            status === 'reconnecting' ? t('states.connectionLost.title') : t('states.unavailable')
          "
          :text="
            status === 'reconnecting' ? t('states.connectionLost.text') : t('states.lastKnown')
          "
          :action-label="t('states.reconnectNow')"
          @action="reconnect()"
        />
      </div>

      <!-- 2. standings -->
      <section aria-labelledby="st-title" class="mb-12">
        <h2 id="st-title" class="display mb-1 text-4xl text-white">{{ t('standings.title') }}</h2>
        <p class="mb-5 text-sm text-muted">{{ t('standings.subtitle') }}</p>
        <StandingsTable />
      </section>

      <!-- 3. progress -->
      <section aria-labelledby="tr-title" class="mb-12">
        <h2 id="tr-title" class="display mb-5 text-4xl text-white">{{ t('game.trackTitle') }}</h2>
        <RaceTrack />
      </section>

      <!-- 4. clocks + 6. activity -->
      <div class="mb-12 grid gap-8 xl:grid-cols-[1.5fr_1fr]">
        <section aria-labelledby="ck-title">
          <h2 id="ck-title" class="display mb-5 text-4xl text-white">
            {{ t('game.dailyClocks') }}
          </h2>
          <div v-if="!racers.loaded" class="grid gap-3 sm:grid-cols-2">
            <SkeletonBlock v-for="i in 4" :key="i" class="h-32" />
          </div>
          <div v-else class="grid gap-3 sm:grid-cols-2 2xl:grid-cols-3">
            <RacerClockTile v-for="s in race.standings" :key="s.racerId" :racer-id="s.racerId" />
          </div>
        </section>
        <section aria-labelledby="ac-title">
          <h2 id="ac-title" class="display mb-5 text-4xl text-white">{{ t('activity.title') }}</h2>
          <ActivityFeed :limit="9" />
        </section>
      </div>

      <!-- 5. streams -->
      <section aria-labelledby="sm-title" class="mb-12">
        <div class="mb-5 flex items-end justify-between gap-4">
          <h2 id="sm-title" class="display text-4xl text-white">{{ t('game.watchLive') }}</h2>
          <Button to="/streams" variant="ghost" size="sm">{{ t('game.allStreams') }}</Button>
        </div>
        <StreamGrid :limit="4" live-only />
      </section>

      <!-- 7. telemetry -->
      <section aria-labelledby="hs-title" class="grid gap-8 lg:grid-cols-[1fr_1.1fr]">
        <div>
          <h2 id="hs-title" class="display mb-5 text-4xl text-white">{{ t('hiveshock.live') }}</h2>
          <HiveShockStats />
        </div>
        <EventTerminal :limit="8" />
      </section>
    </div>
  </div>
</template>
