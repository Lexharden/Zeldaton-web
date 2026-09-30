<script setup lang="ts">
import { computed, ref } from 'vue'
import Accordion from '@/components/ui/Accordion.vue'
import { t } from '@/i18n'
import { useEventStore } from '@/stores/event'
import { formatResetClock } from '@/utils/time'

/**
 * Rules from i18n `rulesPage.rules`, interpolated with live event values (budget, reset, win
 * condition). Numbered grid on desktop, accordion on mobile.
 */
const IDS = [
  'daily',
  'use',
  'reset',
  'tz',
  'pause',
  'disconnect',
  'versions',
  'win',
  'prohibited',
  'technical',
  'force',
]
const event = useEventStore()
const open = ref<string | null>('daily')
const rules = computed(() => {
  const values = {
    budget: event.budgetHours || 4,
    reset: formatResetClock(event.info?.dailyResetLocalTime ?? '06:00'),
    win: event.info?.rules.winCondition ?? t('rulesPage.fallbackWin'),
  }
  return IDS.map((id) => ({
    id,
    title: t(`rulesPage.rules.${id}.title`, values),
    body: t(`rulesPage.rules.${id}.body`, values),
  }))
})
</script>

<template>
  <div>
    <div class="space-y-2 md:hidden">
      <Accordion
        v-for="r in rules"
        :id="`rule-${r.id}`"
        :key="r.id"
        :title="r.title"
        :model-value="open === r.id"
        @update:model-value="open = $event ? r.id : null"
      >
        {{ r.body }}
      </Accordion>
    </div>
    <ol class="hidden gap-4 md:grid md:grid-cols-2">
      <li
        v-for="(r, i) in rules"
        :key="r.id"
        v-reveal="i % 2"
        class="panel panel-sm"
        :class="r.id === 'win' && 'panel-gold md:col-span-2'"
      >
        <div class="flex gap-5 p-6">
          <span class="num text-2xl font-bold text-accent">{{
            String(i + 1).padStart(2, '0')
          }}</span>
          <div>
            <h3 class="display text-3xl text-white">{{ r.title }}</h3>
            <p class="mt-2 text-sm leading-relaxed text-muted">{{ r.body }}</p>
          </div>
        </div>
      </li>
    </ol>
  </div>
</template>
