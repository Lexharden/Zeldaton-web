<script setup lang="ts">
import { computed } from 'vue'
import { t } from '@/i18n'
import PlatformIcon from '@/components/racers/PlatformIcon.vue'
import SkeletonBlock from '@/components/common/SkeletonBlock.vue'
import { useDonorsStore } from '@/stores/donors'
import type { PublicDonor } from '@/types/donors'
import { formatCount, formatSpan } from '@/utils/format'

/**
 * Podium (top 3) and list (4 to 10) of the viewers whose donations moved the race clock the most.
 * Ranked by time, the one measure comparable between TikTok diamonds and Twitch bits.
 */
const donors = useDonorsStore()
const board = computed(() => donors.board)
const podium = computed(() => donors.donors.slice(0, 3))
const rest = computed(() => donors.donors.slice(3))

const MEDALS = ['🥇', '🥈', '🥉']
const SKIN = [
  'border-accent/60 bg-accent/10 sm:order-2 sm:-mt-6',
  'border-white/30 bg-white/[0.06] sm:order-1',
  'border-[#cd7f32]/50 bg-[#cd7f32]/10 sm:order-3',
]
const moved = (d: PublicDonor) => d.addedSeconds + d.removedSeconds
const paid = (d: PublicDonor) =>
  `${formatCount(d.amount)} ${d.currency === 'bits' ? t('donors.bits') : t('donors.diamonds')}`
</script>

<template>
  <div>
    <div v-if="donors.loading && !board" class="grid gap-4 sm:grid-cols-3">
      <SkeletonBlock v-for="i in 3" :key="i" class="h-40" />
    </div>

    <p v-else-if="!donors.donors.length" class="panel p-8 text-center text-sm text-muted">
      {{ t('donors.empty') }}
    </p>

    <template v-else>
      <ol class="grid items-end gap-4 sm:grid-cols-3" :aria-label="t('donors.aria')">
        <li
          v-for="(d, i) in podium"
          :key="`${d.platform}-${d.viewer}`"
          class="relative border p-5 text-center [clip-path:polygon(0_0,calc(100%-14px)_0,100%_14px,100%_100%,14px_100%,0_calc(100%-14px))]"
          :class="SKIN[i]"
        >
          <p class="text-4xl" aria-hidden="true">{{ MEDALS[i] }}</p>
          <p class="sr-only">{{ t('donors.place', { n: d.rank }) }}</p>
          <p
            class="display mt-2 flex items-center justify-center gap-2 text-3xl text-white"
            :title="d.viewer"
          >
            <PlatformIcon :platform="d.platform" class="size-5 shrink-0 text-muted" />
            <span class="truncate">{{ d.viewer }}</span>
          </p>
          <p class="num mt-3 text-2xl font-bold text-success">+{{ formatSpan(d.addedSeconds) }}</p>
          <p v-if="d.removedSeconds" class="num text-sm font-semibold text-[#ff8aa0]">
            −{{ formatSpan(d.removedSeconds) }}
          </p>
          <p class="mt-2 text-xs text-muted">
            {{ t('donors.count', { n: d.donations }) }} · {{ paid(d) }}
          </p>
        </li>
      </ol>

      <ol v-if="rest.length" class="mt-6 space-y-2" start="4">
        <li
          v-for="d in rest"
          :key="`${d.platform}-${d.viewer}`"
          class="panel flex items-center gap-3 px-4 py-3"
        >
          <span class="num w-6 text-sm font-bold text-muted">{{ d.rank }}</span>
          <PlatformIcon :platform="d.platform" class="size-4 shrink-0 text-muted" />
          <span class="min-w-0 flex-1 truncate font-semibold text-white" :title="d.viewer">{{
            d.viewer
          }}</span>
          <span class="hidden text-xs text-muted sm:inline">{{
            t('donors.count', { n: d.donations })
          }}</span>
          <span class="num text-sm font-bold" :class="moved(d) ? 'text-success' : 'text-muted'"
            >+{{ formatSpan(d.addedSeconds) }}</span
          >
          <span v-if="d.removedSeconds" class="num text-sm font-semibold text-[#ff8aa0]"
            >−{{ formatSpan(d.removedSeconds) }}</span
          >
        </li>
      </ol>

      <p v-if="board" class="mt-6 text-center text-xs text-muted">
        {{
          t('donors.totals', {
            n: formatCount(board.totals.donations),
            added: formatSpan(board.totals.addedSeconds),
            removed: formatSpan(board.totals.removedSeconds),
          })
        }}
      </p>
    </template>
  </div>
</template>
