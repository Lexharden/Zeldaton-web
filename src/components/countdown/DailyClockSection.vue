<script setup lang="ts">
import { t } from '@/i18n'
import { computed } from 'vue'
import SectionHeader from '@/components/common/SectionHeader.vue'
import { SHOWCASE_TIMEZONES } from '@/config/event'
import { useEventStore } from '@/stores/event'
import { formatResetClock } from '@/utils/time'
import TimezoneClockCard from './TimezoneClockCard.vue'

/** "YOUR TIME. YOUR TIMEZONE." Values come from the event; nothing about the schedule is hardcoded. */
const event = useEventStore()
const reset = computed(() => event.info?.dailyResetLocalTime ?? '06:00')
</script>

<template>
  <section class="section" aria-labelledby="tz-title">
    <div class="container-x">
      <SectionHeader
        id="tz-title"
        index="02"
        :eyebrow="t('daily.eyebrow')"
        :title="[t('daily.title1'), t('daily.title2')]"
      >
        <p class="mt-4 max-w-2xl text-base text-muted md:text-lg">
          {{ t('daily.body', { hours: event.budgetHours, reset: formatResetClock(reset) }) }}
        </p>
      </SectionHeader>
      <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <div v-for="(tz, i) in SHOWCASE_TIMEZONES" :key="tz" v-reveal="i">
          <TimezoneClockCard :timezone="tz" :reset-local-time="reset" />
        </div>
      </div>
      <p class="mt-6 flex items-center gap-3 font-mono text-xs uppercase tracking-wider text-muted">
        <span class="h-px w-8 bg-accent" aria-hidden="true" />
        {{ t('daily.note') }}
      </p>
    </div>
  </section>
</template>
