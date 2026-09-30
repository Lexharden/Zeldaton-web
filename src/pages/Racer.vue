<script setup lang="ts">
import { t } from '@/i18n'
import { computed, watch } from 'vue'
import { ArrowLeft } from 'lucide-vue-next'
import { RouterLink, useRoute } from 'vue-router'
import Avatar from '@/components/common/Avatar.vue'
import GamingBackground from '@/components/common/GamingBackground.vue'
import LiveBadge from '@/components/common/LiveBadge.vue'
import ProgressBar from '@/components/common/ProgressBar.vue'
import SkeletonBlock from '@/components/common/SkeletonBlock.vue'
import StateMessage from '@/components/common/StateMessage.vue'
import ClockDisplay from '@/components/countdown/ClockDisplay.vue'
import LiveRaceClock from '@/components/countdown/LiveRaceClock.vue'
import GameProgress from '@/components/game/GameProgress.vue'
import RacerItems from '@/components/game/RacerItems.vue'
import PlatformIcon from '@/components/racers/PlatformIcon.vue'
import RacerStats from '@/components/stats/RacerStats.vue'
import StreamEmbed from '@/components/streams/StreamEmbed.vue'
import { useRaceClock } from '@/composables/useRaceClock'
import { useRacer } from '@/composables/useRacer'
import { useSeo } from '@/composables/useSeo'
import { useEventStore } from '@/stores/event'
import { useRacersStore } from '@/stores/racers'
import { primaryChannel } from '@/utils/channels'
import { formatPercent } from '@/utils/format'
import { formatDateTime, formatLocalTime, formatResetTime, formatTimezone } from '@/utils/time'

/** Player profile: hero, live clock, game progress, items, stats and stream. */
const route = useRoute()
const id = computed(() => String(route.params.id))
const store = useRacersStore()
const event = useEventStore()
const { racer, rank, progress } = useRacer(id)
const clock = useRaceClock(id)

watch(id, (v) => store.loadOne(v), { immediate: true })
useSeo(() => racer.value?.displayName ?? t('meta.pages.racer'))

const loading = computed(() => !racer.value && !store.loaded)
const facts = computed(() => {
  const r = racer.value
  if (!r) return []
  const reset = clock.resetAtUtc.value
  return [
    {
      id: 'daily',
      label: t('racerPage.dailyTime'),
      kind: 'clock' as const,
      seconds: clock.remainingSeconds.value,
    },
    {
      id: 'used',
      label: t('racerPage.used'),
      kind: 'clock' as const,
      seconds: clock.usedSeconds.value,
    },
    {
      id: 'timezone',
      label: t('racerPage.timezone'),
      kind: 'text' as const,
      value: r.timezone,
      sub: formatTimezone(r.timezone),
    },
    {
      id: 'reset',
      label: t('racerPage.nextReset'),
      kind: 'text' as const,
      value: formatResetTime(event.info?.dailyResetLocalTime ?? '06:00'),
      sub: reset ? formatDateTime(reset, r.timezone) : '—',
    },
    {
      id: 'local',
      label: t('racerPage.localTime'),
      kind: 'text' as const,
      value: formatLocalTime(Date.now(), r.timezone),
      sub: formatTimezone(r.timezone),
    },
  ]
})
</script>

<template>
  <div class="relative pb-20">
    <div v-if="loading" class="container-x space-y-6 pt-12" aria-busy="true">
      <SkeletonBlock class="h-56" /><SkeletonBlock class="h-72" />
    </div>

    <div v-else-if="!racer" class="container-x pt-16">
      <StateMessage
        variant="no-data"
        :title="t('racerPage.notFoundTitle')"
        :text="t('racerPage.notFoundText')"
        :action-label="t('racerPage.backToRace')"
        @action="$router.push('/race')"
      />
    </div>

    <template v-else>
      <section class="relative overflow-hidden pb-12 pt-8" aria-labelledby="racer-name">
        <GamingBackground shapes particles variant="hero" />
        <div class="container-x">
          <RouterLink
            to="/race"
            class="hud-label mb-8 inline-flex items-center gap-2 hover:text-white"
            ><ArrowLeft class="size-3.5" />{{ t('racerPage.back') }}</RouterLink
          >
          <div class="grid items-end gap-8 lg:grid-cols-[1.4fr_1fr]">
            <div class="flex flex-col gap-6 sm:flex-row sm:items-center">
              <Avatar
                :name="racer.displayName"
                :src="racer.avatarUrl"
                :seed="racer.id"
                :size="128"
                class="enter"
              />
              <div>
                <div class="flex flex-wrap items-center gap-3">
                  <LiveBadge :status="clock.status.value" /><span class="hud-label">{{
                    event.info?.game?.toUpperCase() ?? t('event.game')
                  }}</span>
                </div>
                <h1 id="racer-name" class="display mt-3 text-[clamp(3rem,10vw,7rem)] text-white">
                  {{ racer.displayName }}
                </h1>
                <p
                  class="mt-2 flex flex-wrap items-center gap-x-4 gap-y-1 font-mono text-sm text-muted"
                >
                  <span
                    v-for="c in racer.channels"
                    :key="c.platform"
                    class="flex items-center gap-2"
                    ><PlatformIcon :platform="c.platform" />@{{ c.handle }}</span
                  >
                  <span v-if="racer.country">{{ racer.country }}</span>
                </p>
              </div>
            </div>
            <div class="grid grid-cols-2 gap-3">
              <div class="panel panel-gold">
                <div class="p-5">
                  <p class="hud-label">{{ t('racerPage.currentRank') }}</p>
                  <p class="display mt-1 text-6xl text-gold">#{{ rank }}</p>
                </div>
              </div>
              <div class="panel">
                <div class="p-5">
                  <p class="hud-label">{{ t('racerPage.progress') }}</p>
                  <p class="num mt-2 text-5xl font-bold text-white">
                    {{ formatPercent(racer.progressPercentage) }}
                  </p>
                </div>
              </div>
              <div class="col-span-2">
                <ProgressBar
                  :value="racer.progressPercentage"
                  tone="gold"
                  :segments="30"
                  :label="t('racers.gameProgress')"
                />
              </div>
            </div>
          </div>
        </div>
      </section>

      <StateMessage
        v-if="clock.status.value === 'offline'"
        class="container-x mb-8"
        variant="racer-offline"
      />

      <div class="container-x space-y-8">
        <!-- live clock -->
        <section aria-labelledby="lc-title" class="grid gap-4 lg:grid-cols-[1.1fr_1fr]">
          <h2 id="lc-title" class="sr-only">{{ t('racerPage.liveClock') }}</h2>
          <LiveRaceClock :racer-id="racer.id" />
          <dl class="grid grid-cols-2 gap-3">
            <div
              v-for="f in facts"
              :key="f.label"
              class="panel panel-sm"
              :class="f.id === 'timezone' && 'col-span-2'"
            >
              <div class="p-4">
                <dt class="hud-label text-[10px]">{{ f.label }}</dt>
                <dd class="mt-1.5">
                  <ClockDisplay v-if="f.kind === 'clock'" :seconds="f.seconds" size="md" />
                  <template v-else
                    ><span class="num block truncate text-lg font-bold text-white">{{
                      f.value
                    }}</span
                    ><span class="text-xs text-muted">{{ f.sub }}</span></template
                  >
                </dd>
              </div>
            </div>
          </dl>
        </section>

        <!-- progress + items + stats -->
        <div class="grid gap-8 lg:grid-cols-[1.2fr_1fr]">
          <section class="panel panel-lg" aria-labelledby="gp-title">
            <div class="p-6 sm:p-8">
              <h2 id="gp-title" class="display mb-6 text-4xl text-white">
                {{ t('racerPage.gameProgress') }}
              </h2>
              <GameProgress v-if="progress" :progress="progress" />
            </div>
          </section>
          <div class="space-y-8">
            <section class="panel panel-lg" aria-labelledby="it-title">
              <div class="p-6">
                <h2 id="it-title" class="display mb-5 text-3xl text-white">
                  {{ t('racerPage.items') }}
                </h2>
                <RacerItems :racer="racer" />
              </div>
            </section>
            <section aria-labelledby="stt-title">
              <h2 id="stt-title" class="display mb-4 text-3xl text-white">
                {{ t('racerPage.stats') }}
              </h2>
              <RacerStats :racer="racer" />
            </section>
          </div>
        </div>

        <section aria-labelledby="ls-title">
          <h2 id="ls-title" class="display mb-5 text-4xl text-white">
            {{ t('streams.liveStream') }}
          </h2>
          <StreamEmbed
            :channels="racer.channels"
            :primary="primaryChannel(racer)"
            :name="racer.displayName"
          />
        </section>
      </div>
    </template>
  </div>
</template>
