<script setup lang="ts">
import { t } from '@/i18n'
import ArtImage from '@/components/common/ArtImage.vue'
import { ART } from '@/config/artwork'
import { useRaceStore } from '@/stores/race'
import RacerMiniCard from '@/components/racers/RacerMiniCard.vue'
import { useEventStore } from '@/stores/event'
import ClockDisplay from '@/components/countdown/ClockDisplay.vue'

/**
 * Artwork slot for the hero. Renders abstract original geometry (rings, diamonds, faceted gem). To use real artwork, drop an <img> into the `art` slot.
 */
const race = useRaceStore()
const event = useEventStore()
</script>

<template>
  <div class="relative mx-auto aspect-square w-full max-w-[560px]" aria-hidden="false">
    <slot name="art">
      <svg
        viewBox="0 0 600 600"
        class="absolute inset-0 size-full"
        role="presentation"
        aria-hidden="true"
      >
        <defs>
          <linearGradient id="gold" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0" stop-color="#fde68a" />
            <stop offset="1" stop-color="#d99a1f" />
          </linearGradient>
          <radialGradient id="core" cx="50%" cy="50%" r="50%">
            <stop offset="0" stop-color="#4c9a52" stop-opacity=".55" />
            <stop offset="1" stop-color="#4c9a52" stop-opacity="0" />
          </radialGradient>
          <filter id="blur"><feGaussianBlur stdDeviation="14" /></filter>
        </defs>
        <circle cx="300" cy="300" r="260" fill="url(#core)" />
        <g
          class="origin-center motion-safe:animate-[spin-slow_90s_linear_infinite]"
          style="transform-origin: 300px 300px"
        >
          <circle
            cx="300"
            cy="300"
            r="250"
            fill="none"
            stroke="#4c9a52"
            stroke-opacity=".45"
            stroke-dasharray="2 14"
            stroke-width="2"
          />
          <circle cx="300" cy="50" r="6" fill="#4a90d9" />
        </g>
        <g
          class="motion-safe:animate-[spin-slow_140s_linear_infinite_reverse]"
          style="transform-origin: 300px 300px"
        >
          <circle
            cx="300"
            cy="300"
            r="200"
            fill="none"
            stroke="#4a90d9"
            stroke-opacity=".3"
            stroke-dasharray="60 24"
            stroke-width="1.5"
          />
        </g>
        <path
          d="M300 60 540 300 300 540 60 300Z"
          fill="none"
          stroke="#f5c451"
          stroke-opacity=".55"
          stroke-width="2"
        />
        <path
          d="M300 110 490 300 300 490 110 300Z"
          fill="rgb(76 154 82 / 0.08)"
          stroke="#4c9a52"
          stroke-opacity=".6"
          stroke-width="1.5"
        />
        <g stroke="#4a90d9" stroke-opacity=".7" stroke-width="2" fill="none">
          <path d="M20 80V20H80" />
          <path d="M580 80V20H520" />
          <path d="M20 520V580H80" />
          <path d="M580 520V580H520" />
        </g>
      </svg>
    </slot>

    <div class="absolute inset-0 grid place-items-center">
      <ArtImage
        :src="ART.heroCharacter"
        eager
        class="absolute bottom-0 right-[-8%] h-[105%] max-w-none object-contain drop-shadow-[0_0_40px_rgb(0_0_0/0.6)]"
      />
      <ArtImage
        :src="ART.logo"
        alt="The Legend of Zelda: Ocarina of Time"
        eager
        class="relative mt-16 w-[68%] motion-safe:animate-[float_8s_ease-in-out_infinite] drop-shadow-[0_0_38px_rgb(245_196_81/0.35)]"
      />
    </div>

    <div
      class="absolute -left-2 -top-[4%] w-64 motion-safe:animate-[float_7s_ease-in-out_infinite] sm:-left-8"
    >
      <div class="panel panel-sm">
        <div class="p-3">
          <p class="hud-label mb-2 text-[9px]">{{ t('hero.currentLeader') }}</p>
          <RacerMiniCard v-if="race.leader" :racer-id="race.leader.id" />
          <p v-else class="text-xs text-muted">{{ t('hero.waiting') }}</p>
        </div>
      </div>
    </div>
    <div
      class="absolute -right-2 bottom-[12%] w-48 motion-safe:animate-[float_9s_ease-in-out_infinite_1s] sm:-right-6"
      style="--r: 0deg"
    >
      <div class="panel panel-sm panel-gold">
        <div class="p-3">
          <p class="hud-label mb-1 text-[9px]">{{ t('hero.dailyPlay') }}</p>
          <ClockDisplay :seconds="event.budgetSeconds" size="md" tone="gold" />
        </div>
      </div>
    </div>
  </div>
</template>
