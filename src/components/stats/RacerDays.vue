<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { intlLocale, t } from '@/i18n'
import SkeletonBlock from '@/components/common/SkeletonBlock.vue'
import { raceApi } from '@/services/api'
import type { PublicDay } from '@/types/stats'
import { formatCount, formatSpan } from '@/utils/format'

/**
 * A racer's game days, one row per day: time really played, progress, objectives, items, bosses and
 * what donations did to the clock. A "day" is the stretch between two of the racer's daily resets.
 */
const props = defineProps<{ racerId: string }>()
const days = ref<PublicDay[] | null>(null)
let timer: ReturnType<typeof setInterval> | null = null

async function load() {
  try {
    days.value = await raceApi.getRacerDays(props.racerId)
  } catch {
    // A failed refresh keeps what is on screen; the first failure just hides the section.
    days.value ??= []
  }
}
watch(
  () => props.racerId,
  () => {
    days.value = null
    void load()
    if (timer) clearInterval(timer)
    timer = setInterval(() => !document.hidden && void load(), 60_000)
  },
  { immediate: true },
)
onBeforeUnmount(() => timer && clearInterval(timer))

/** Most recent first: what happened today matters most. */
const rows = computed(() => [...(days.value ?? [])].reverse())
const anyPartial = computed(() => rows.value.some((d) => d.partial))

function label(day: string): string {
  const [y, m, d] = day.split('-').map(Number)
  if (!y || !m || !d) return day
  return new Intl.DateTimeFormat(intlLocale.value, {
    weekday: 'short',
    day: 'numeric',
    month: 'short',
    timeZone: 'UTC',
  }).format(new Date(Date.UTC(y, m - 1, d)))
}
const progress = (d: PublicDay) =>
  d.progressStart === null || d.progressEnd === null
    ? '—'
    : `${Math.round(d.progressStart)}% → ${Math.round(d.progressEnd)}%`
</script>

<template>
  <section v-if="days === null || days.length" aria-labelledby="days-title">
    <h2 id="days-title" class="display mb-4 text-3xl text-white">{{ t('racerDays.title') }}</h2>
    <SkeletonBlock v-if="days === null" class="h-32" />
    <div v-else class="panel">
      <div class="overflow-x-auto p-2">
        <table class="w-full min-w-[34rem] text-left text-sm">
          <thead>
            <tr class="hud-label text-[10px]">
              <th class="p-2 font-normal">{{ t('racerDays.day') }}</th>
              <th class="p-2 font-normal">{{ t('racerDays.played') }}</th>
              <th class="p-2 font-normal">{{ t('racerDays.progress') }}</th>
              <th class="p-2 text-right font-normal">{{ t('racerDays.objectives') }}</th>
              <th class="p-2 text-right font-normal">{{ t('racerDays.items') }}</th>
              <th class="p-2 text-right font-normal">{{ t('racerDays.bosses') }}</th>
              <th class="p-2 font-normal">{{ t('racerDays.donations') }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="d in rows" :key="d.day" class="border-t border-line">
              <td class="p-2 font-semibold capitalize text-white">
                {{ label(d.day) }}
                <span
                  v-if="d.partial"
                  class="cursor-help text-warning"
                  :title="t('racerDays.partial')"
                  >~</span
                >
              </td>
              <td class="num p-2 text-white">{{ formatSpan(d.playedSeconds) }}</td>
              <td class="num whitespace-nowrap p-2">{{ progress(d) }}</td>
              <td class="num p-2 text-right">{{ formatCount(d.objectives) }}</td>
              <td class="num p-2 text-right">{{ formatCount(d.items) }}</td>
              <td class="num p-2 text-right">{{ formatCount(d.bosses) }}</td>
              <td class="num whitespace-nowrap p-2">
                <template v-if="d.donations">
                  {{ formatCount(d.donations) }}
                  <span v-if="d.donationAddedSeconds" class="text-success"
                    >+{{ formatSpan(d.donationAddedSeconds) }}</span
                  >
                  <span v-if="d.donationRemovedSeconds" class="ml-1 text-[#ff8aa0]"
                    >−{{ formatSpan(d.donationRemovedSeconds) }}</span
                  >
                </template>
                <template v-else>—</template>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
      <p v-if="anyPartial" class="px-4 pb-3 text-xs text-muted">
        <span class="text-warning">~</span> {{ t('racerDays.partial') }}
      </p>
    </div>
  </section>
</template>
