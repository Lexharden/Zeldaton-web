<script setup lang="ts">
import { t, tx } from '@/i18n'
import { computed } from 'vue'
import { Check, Circle } from 'lucide-vue-next'
import AgeBadge from '@/components/common/AgeBadge.vue'
import { useCatalogStore } from '@/stores/catalog'
import type { CatalogAge } from '@/types/catalog'
import type { GameProgress } from '@/types/game'
import { AGE_ORDER } from '@/utils/catalog'
import { formatPercent } from '@/utils/format'
import ProgressBar from '@/components/common/ProgressBar.vue'

/**
 * Full progress panel: percentage, current area/objective and the objectives checklist.
 * Data: a normalized GameProgress (see utils/progress.buildGameProgress).
 */
const props = defineProps<{ progress: GameProgress }>()
const catalog = useCatalogStore()

/** Objectives split by Link (child dungeons / adult temples), each with its own counter. */
const sections = computed(() =>
  AGE_ORDER.flatMap((age: CatalogAge) => {
    const list = props.progress.objectives.filter((o) => o.age === age)
    return list.length ? [{ age, list, done: list.filter((o) => o.completed).length }] : []
  }),
)
</script>

<template>
  <div class="space-y-6">
    <div>
      <div class="mb-2 flex items-end justify-between">
        <span class="hud-label">{{ t('racers.gameProgress') }}</span>
        <span class="num text-4xl font-bold text-white">{{
          formatPercent(progress.percentage)
        }}</span>
      </div>
      <ProgressBar
        :value="progress.percentage"
        tone="gold"
        :segments="30"
        :label="t('racers.gameProgress')"
      />
    </div>

    <dl class="grid gap-4 sm:grid-cols-2">
      <div>
        <dt class="hud-label">{{ t('game.currentArea') }}</dt>
        <dd class="mt-1 font-display text-2xl font-bold uppercase text-secondary">
          {{ progress.currentArea ? tx('areas', progress.currentArea) : t('game.unknown') }}
        </dd>
      </div>
      <div>
        <dt class="hud-label">{{ t('game.currentObjective') }}</dt>
        <dd class="mt-1 text-sm text-white">
          {{
            progress.currentObjective
              ? catalog.objectiveName(progress.currentObjective)
              : t('game.noObjective')
          }}
        </dd>
      </div>
    </dl>

    <div>
      <p class="hud-label mb-3">{{ t('game.objectives') }}</p>
      <div class="grid gap-6 sm:grid-cols-2">
        <section v-for="s in sections" :key="s.age" :aria-label="t(`age.${s.age}`)">
          <div class="mb-2 flex items-center justify-between">
            <AgeBadge :age="s.age" />
            <span class="num text-xs text-muted">{{ s.done }} / {{ s.list.length }}</span>
          </div>
          <ul class="space-y-2">
            <li v-for="o in s.list" :key="o.id" class="flex items-center gap-3 text-sm">
              <Check v-if="o.completed" class="size-4 shrink-0 text-success" aria-hidden="true" />
              <Circle v-else class="size-4 shrink-0 text-white/20" aria-hidden="true" />
              <span :class="o.completed ? 'text-white' : 'text-muted'">{{ o.label }}</span>
              <span class="sr-only">{{
                o.completed ? t('game.completed') : t('game.pending')
              }}</span>
            </li>
          </ul>
        </section>
      </div>
    </div>
  </div>
</template>
