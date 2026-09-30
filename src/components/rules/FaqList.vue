<script setup lang="ts">
import { computed, ref } from 'vue'
import Accordion from '@/components/ui/Accordion.vue'
import { tm } from '@/i18n'
import { useEventStore } from '@/stores/event'
import { fillTemplate } from '@/utils/template'
import { formatResetClock } from '@/utils/time'

const event = useEventStore()
const open = ref<number | null>(null)
const items = computed(() => {
  const values = {
    budget: event.budgetHours || 4,
    reset: formatResetClock(event.info?.dailyResetLocalTime ?? '06:00'),
  }
  return tm<{ q: string; a: string }[]>('rulesPage.faq').map((f) => ({
    q: fillTemplate(f.q, values),
    a: fillTemplate(f.a, values),
  }))
})
</script>

<template>
  <div class="space-y-2">
    <Accordion
      v-for="(f, i) in items"
      :id="`faq-${i}`"
      :key="i"
      :title="f.q"
      :model-value="open === i"
      @update:model-value="open = $event ? i : null"
    >
      {{ f.a }}
    </Accordion>
  </div>
</template>
