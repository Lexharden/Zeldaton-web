<script setup lang="ts">
import { t, tx } from '@/i18n'
import { computed } from 'vue'
import { RouterLink } from 'vue-router'
import Avatar from '@/components/common/Avatar.vue'
import LiveBadge from '@/components/common/LiveBadge.vue'
import ProgressBar from '@/components/common/ProgressBar.vue'
import ClockDisplay from '@/components/countdown/ClockDisplay.vue'
import { useRaceClock } from '@/composables/useRaceClock'
import { useRacer } from '@/composables/useRacer'
import { formatPercent } from '@/utils/format'
import { formatTimezone } from '@/utils/time'
import ChannelButtons from './ChannelButtons.vue'
import PlatformIcon from './PlatformIcon.vue'

/**
 * "Fighter select" style racer card.
 * Data: racerId -> racer/rank from stores, clock from useRaceClock.
 * Navigates to /racer/:id; the stream button is a separate external link.
 */
const props = defineProps<{ racerId: string }>()
const { racer, rank } = useRacer(() => props.racerId)
const clock = useRaceClock(() => props.racerId)
const status = computed(() => clock.status.value)
const dim = computed(() => status.value === 'offline')
</script>

<template>
  <article
    v-if="racer"
    class="panel panel-hover group relative h-full"
    :class="rank === 1 && 'panel-gold'"
  >
    <div class="relative overflow-hidden">
      <!-- art zone -->
      <div
        class="relative h-36 overflow-hidden bg-gradient-to-br from-primary/25 via-surface-elevated to-surface"
      >
        <div
          class="absolute -right-6 -top-8 h-56 w-32 skew-x-[-18deg] bg-secondary/10 transition-transform duration-500 group-hover:translate-x-3"
        />
        <div class="absolute -left-4 bottom-0 h-full w-16 skew-x-[-18deg] bg-accent/8" />
        <span
          class="display absolute -bottom-6 right-3 select-none text-[9rem] leading-none text-white/6 transition-transform duration-500 group-hover:scale-105"
          aria-hidden="true"
          >{{ rank }}</span
        >
        <div
          class="absolute left-5 top-5 flex items-center gap-4 transition-transform duration-500 group-hover:scale-[1.04]"
          :class="dim && 'opacity-60'"
        >
          <Avatar :name="racer.displayName" :src="racer.avatarUrl" :seed="racer.id" :size="72" />
        </div>
        <div class="absolute right-4 top-4"><LiveBadge :status="status" size="sm" /></div>
        <div class="absolute bottom-3 left-5 font-mono text-[11px] tracking-widest text-muted">
          {{ t('racers.rank') }}
          <span class="text-base font-bold" :class="rank === 1 ? 'text-accent' : 'text-white'"
            >#{{ rank }}</span
          >
        </div>
      </div>

      <div class="space-y-4 p-5">
        <div>
          <h3 class="display text-3xl text-white">
            <RouterLink
              :to="`/racer/${racer.id}`"
              class="after:absolute after:inset-0 after:content-['']"
            >
              {{ racer.displayName }}
            </RouterLink>
          </h3>
          <p class="mt-1 flex items-center gap-2 text-xs text-muted">
            <PlatformIcon v-for="c in racer.channels" :key="c.platform" :platform="c.platform" />
            <span
              >{{ racer.country ? `${racer.country} · ` : ''
              }}{{ formatTimezone(racer.timezone) }}</span
            >
          </p>
        </div>

        <div class="border-t border-line pt-3">
          <p class="hud-label text-[10px]">{{ t('racers.location') }}</p>
          <p
            class="mt-0.5 truncate font-display text-xl font-semibold uppercase tracking-wide text-secondary"
          >
            {{ racer.currentArea ? tx('areas', racer.currentArea) : t('racers.notStarted') }}
          </p>
        </div>

        <div>
          <div class="mb-1.5 flex items-end justify-between">
            <span class="hud-label text-[10px]">{{ t('racers.gameProgress') }}</span>
            <span class="num text-2xl font-bold text-white">{{
              formatPercent(racer.progressPercentage)
            }}</span>
          </div>
          <ProgressBar
            :value="racer.progressPercentage"
            :tone="rank === 1 ? 'gold' : 'primary'"
            :label="t('racers.gameProgress')"
          />
        </div>

        <div class="grid grid-cols-2 gap-3">
          <div>
            <p class="hud-label text-[10px]">{{ t('racers.used') }}</p>
            <ClockDisplay :seconds="clock.usedSeconds.value" size="sm" tone="muted" />
          </div>
          <div>
            <p class="hud-label text-[10px]">{{ t('racers.left') }}</p>
            <ClockDisplay
              :seconds="clock.remainingSeconds.value"
              size="sm"
              :tone="status === 'exhausted' ? 'danger' : 'default'"
            />
          </div>
        </div>

        <div class="relative z-10">
          <ChannelButtons :channels="racer.channels" :live="racer.stream?.isLive" />
        </div>
      </div>
    </div>
  </article>
</template>
