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

/** One standings entry: table row on md+, card on mobile. Same data, two layouts. */
const props = defineProps<{ racerId: string; rank: number }>()
const { racer } = useRacer(() => props.racerId)
const clock = useRaceClock(() => props.racerId)
const exhausted = computed(() => clock.status.value === 'exhausted')
const medal = computed(() =>
  props.rank === 1
    ? 'text-accent'
    : props.rank === 2
      ? 'text-[#cbd5e1]'
      : props.rank === 3
        ? 'text-[#d6a06b]'
        : 'text-white',
)
</script>

<template>
  <li v-if="racer" class="relative">
    <RouterLink
      :to="`/racer/${racer.id}`"
      class="group block bg-surface/80 ring-1 ring-inset ring-line transition-colors hover:bg-surface-elevated hover:ring-primary/60 [clip-path:polygon(10px_0,100%_0,100%_calc(100%-10px),calc(100%-10px)_100%,0_100%,0_10px)]"
      :class="rank === 1 && 'ring-accent/50'"
    >
      <!-- desktop -->
      <div
        class="hidden items-center gap-4 px-4 py-3 md:grid md:grid-cols-[52px_minmax(150px,1.3fr)_minmax(150px,1.2fr)_minmax(110px,1fr)_92px_92px_104px]"
      >
        <span class="display text-4xl" :class="medal">{{ rank }}</span>
        <span class="flex min-w-0 items-center gap-3">
          <Avatar :name="racer.displayName" :src="racer.avatarUrl" :seed="racer.id" :size="38" />
          <span class="truncate font-display text-xl font-bold uppercase text-white">{{
            racer.displayName
          }}</span>
        </span>
        <span class="flex items-center gap-3">
          <ProgressBar
            :value="racer.progressPercentage"
            :segments="14"
            :tone="rank === 1 ? 'gold' : 'primary'"
            thin
            class="w-24 lg:w-32"
            :label="t('standings.progress')"
          />
          <span class="num text-sm font-semibold text-white">{{
            formatPercent(racer.progressPercentage)
          }}</span>
        </span>
        <span class="truncate text-sm text-secondary">{{
          racer.currentArea ? tx('areas', racer.currentArea) : '—'
        }}</span>
        <ClockDisplay :seconds="clock.usedSeconds.value" size="sm" tone="muted" />
        <ClockDisplay
          :seconds="clock.remainingSeconds.value"
          size="sm"
          :tone="exhausted ? 'danger' : 'default'"
        />
        <LiveBadge :status="clock.status.value" size="sm" />
      </div>

      <!-- mobile -->
      <div class="space-y-3 p-4 md:hidden">
        <div class="flex items-center gap-3">
          <span class="display w-8 text-4xl" :class="medal">{{ rank }}</span>
          <Avatar :name="racer.displayName" :src="racer.avatarUrl" :seed="racer.id" :size="40" />
          <span
            class="min-w-0 flex-1 truncate font-display text-2xl font-bold uppercase text-white"
            >{{ racer.displayName }}</span
          >
          <LiveBadge :status="clock.status.value" size="sm" />
        </div>
        <div class="flex items-center gap-3">
          <ProgressBar
            :value="racer.progressPercentage"
            :segments="18"
            thin
            class="flex-1"
            :label="t('standings.progress')"
          />
          <span class="num text-sm font-bold text-white">{{
            formatPercent(racer.progressPercentage)
          }}</span>
        </div>
        <div class="grid grid-cols-3 gap-2 text-xs">
          <div>
            <p class="hud-label text-[9px]">{{ t('standings.location') }}</p>
            <p class="mt-0.5 truncate text-secondary">
              {{ racer.currentArea ? tx('areas', racer.currentArea) : '—' }}
            </p>
          </div>
          <div>
            <p class="hud-label text-[9px]">{{ t('racers.used') }}</p>
            <ClockDisplay :seconds="clock.usedSeconds.value" size="sm" tone="muted" />
          </div>
          <div>
            <p class="hud-label text-[9px]">{{ t('racers.left') }}</p>
            <ClockDisplay
              :seconds="clock.remainingSeconds.value"
              size="sm"
              :tone="exhausted ? 'danger' : 'default'"
            />
          </div>
        </div>
      </div>
    </RouterLink>
  </li>
</template>
