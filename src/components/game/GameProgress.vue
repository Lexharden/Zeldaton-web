<script setup lang="ts">
import { t, tx } from '@/i18n'
import { Check, Circle } from 'lucide-vue-next'
import type { GameProgress } from '@/types/game'
import { formatPercent } from '@/utils/format'
import ProgressBar from '@/components/common/ProgressBar.vue'

/**
 * Full progress panel: percentage, current area/objective and the objectives checklist.
 * Data: a normalized GameProgress (see utils/progress.buildGameProgress).
 */
defineProps<{ progress: GameProgress }>()
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
              ? tx('objectives', progress.currentObjective)
              : t('game.noObjective')
          }}
        </dd>
      </div>
    </dl>

    <div>
      <p class="hud-label mb-3">{{ t('game.objectives') }}</p>
      <ul class="grid gap-x-6 gap-y-2 sm:grid-cols-2">
        <li v-for="o in progress.objectives" :key="o.id" class="flex items-center gap-3 text-sm">
          <Check v-if="o.completed" class="size-4 shrink-0 text-success" aria-hidden="true" />
          <Circle v-else class="size-4 shrink-0 text-white/20" aria-hidden="true" />
          <span :class="o.completed ? 'text-white' : 'text-muted'">{{ o.label }}</span>
          <span class="sr-only">{{ o.completed ? t('game.completed') : t('game.pending') }}</span>
        </li>
      </ul>
    </div>
  </div>
</template>
