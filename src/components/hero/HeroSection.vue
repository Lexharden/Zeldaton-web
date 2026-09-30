<script setup lang="ts">
import { t } from '@/i18n'
import { computed } from 'vue'
import { ChevronsDown } from 'lucide-vue-next'
import { ART } from '@/config/artwork'
import GamingBackground from '@/components/common/GamingBackground.vue'
import LiveBadge from '@/components/common/LiveBadge.vue'
import Countdown from '@/components/countdown/Countdown.vue'
import Button from '@/components/ui/Button.vue'
import { useEventStore } from '@/stores/event'
import { useRacersStore } from '@/stores/racers'
import { eventHeadline } from '@/utils/status'
import { formatDateTime, formatTimezone } from '@/utils/time'
import HeroVisual from './HeroVisual.vue'

/** Landing hero. Copy and CTAs react to event state (upcoming / live / paused / finished). */
const event = useEventStore()
const racers = useRacersStore()
const headline = computed(() => eventHeadline(event.status))
const count = computed(() => racers.list.length || 8)
const pillars = computed(() => [
  t('hero.streamers', { n: count.value }),
  t('hero.oneGame'),
  t('hero.hoursDay', { n: event.budgetHours || 4 }),
  t('hero.oneFinish'),
])
</script>

<template>
  <section
    class="relative isolate overflow-hidden pb-16 pt-28 sm:pt-32 lg:min-h-[100svh] lg:pb-20"
    aria-labelledby="hero-title"
  >
    <GamingBackground variant="hero" particles shapes :art="ART.heroBg" />

    <div class="container-x grid items-center gap-10 lg:grid-cols-[1.15fr_0.85fr] lg:gap-6">
      <div>
        <p class="enter mb-5 flex flex-wrap items-center gap-3" style="--d: 0">
          <LiveBadge
            :status="event.status"
            :label="event.status === 'live' ? t('nav.liveNow') : undefined"
          />
          <span class="hud-label !text-white/70"
            >{{ event.info?.name?.toUpperCase() ?? 'ZELDATHON' }} {{ event.info?.edition }}</span
          >
        </p>

        <h1
          id="hero-title"
          class="display enter text-[clamp(3rem,9vw,6.5rem)] text-white"
          style="--d: 1"
        >
          <span class="block">{{ t('hero.title1') }}</span>
          <span class="block">{{ t('hero.title2') }}</span>
          <span class="text-gradient block">{{ t('hero.title3') }}</span>
        </h1>

        <p
          class="enter mt-6 flex flex-wrap gap-x-4 gap-y-1 font-display text-xl font-semibold uppercase tracking-[0.14em] text-white/85 sm:text-2xl"
          style="--d: 2"
        >
          <span v-for="(p, i) in pillars" :key="i" :class="i === 3 && 'text-accent'">{{ p }}</span>
        </p>

        <div class="enter mt-9 max-w-xl" style="--d: 3">
          <p class="hud-label mb-3 !text-secondary">
            {{ event.status === 'live' ? t('hero.raceTime') : headline.eyebrow }}
          </p>
          <template v-if="event.info && event.status !== 'finished'">
            <Countdown
              :target="event.startMs"
              :mode="event.status === 'upcoming' ? 'down' : 'up'"
            />
            <p class="mt-3 font-mono text-[11px] tracking-wider text-muted">
              {{ t('hero.start') }} ·
              {{ formatDateTime(event.info.startAtUtc, event.info.timezone) }} ·
              {{ formatTimezone(event.info.timezone) }}
            </p>
          </template>
          <p v-else-if="event.status === 'finished'" class="display text-5xl text-gold">
            {{ t('hero.eventComplete') }}
          </p>
          <div v-else class="grid grid-cols-4 gap-2">
            <div v-for="i in 4" :key="i" class="skeleton h-20" />
          </div>
        </div>

        <div class="enter mt-10 flex flex-col gap-3 sm:flex-row" style="--d: 4">
          <Button to="/race" variant="primary">{{ headline.cta }}</Button>
          <Button :to="'/#racers'" variant="ghost">{{ t('hero.meetRacers') }}</Button>
        </div>
      </div>

      <div class="enter hidden lg:block" style="--d: 3"><HeroVisual /></div>
    </div>

    <a
      href="#race"
      class="absolute inset-x-0 bottom-5 hidden flex-col items-center gap-1 text-muted transition hover:text-white lg:flex"
      :aria-label="t('hero.scrollAria')"
    >
      <span class="hud-label text-[9px]">{{ t('hero.scroll') }}</span
      ><ChevronsDown class="size-5 motion-safe:animate-bounce" aria-hidden="true" />
    </a>
  </section>
</template>
