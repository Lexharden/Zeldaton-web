<script setup lang="ts">
import { t } from '@/i18n'
import { computed } from 'vue'
import { Gem, Heart, Skull, Sword, Swords } from 'lucide-vue-next'
import { ITEMS, OBJECTIVES } from '@/config/event'
import type { Racer } from '@/types/racer'
import { countItems } from '@/utils/progress'

/**
 * Hearts / rupees / skulltulas / items / bosses.
 * Every field is optional in telemetry: missing values render as "—" without breaking layout.
 */
const props = defineProps<{ racer: Racer }>()
const s = computed(() => props.racer.stats)
const hearts = computed(() => {
  const max = s.value?.maxHearts
  const cur = s.value?.hearts
  return { cur, max, slots: max ? Math.min(20, Math.ceil(max)) : 0 }
})
const items = computed(() =>
  countItems(
    props.racer,
    ITEMS.map((i) => i.id),
  ),
)
const dash = (v: number | undefined) => (v === undefined ? '—' : String(v))
</script>

<template>
  <div class="grid gap-3 sm:grid-cols-2">
    <div class="panel panel-sm sm:col-span-2">
      <div class="p-5">
        <div class="mb-3 flex items-center justify-between">
          <span class="hud-label flex items-center gap-2"
            ><Heart class="size-3.5 text-danger" aria-hidden="true" />{{ t('game.hearts') }}</span
          >
          <span class="num text-lg font-bold text-white"
            >{{ dash(hearts.cur) }} / {{ dash(hearts.max) }}</span
          >
        </div>
        <div
          v-if="hearts.slots"
          class="flex flex-wrap gap-1"
          :aria-label="t('game.heartsAria', { cur: hearts.cur ?? 0, max: hearts.max ?? 0 })"
          role="img"
        >
          <Heart
            v-for="i in hearts.slots"
            :key="i"
            class="size-5 transition-colors"
            :class="i <= (hearts.cur ?? 0) ? 'fill-danger text-danger' : 'text-white/15'"
            aria-hidden="true"
          />
        </div>
        <p v-else class="text-sm text-muted">{{ t('game.noHearts') }}</p>
      </div>
    </div>
    <div
      v-for="stat in [
        { label: t('game.rupees'), icon: Gem, value: dash(s?.rupees), tone: 'text-success' },
        {
          label: t('game.skulltulas'),
          icon: Skull,
          value: s?.skulltulas === undefined ? '—' : `${s.skulltulas} / 100`,
          tone: 'text-accent',
        },
        {
          label: t('game.items'),
          icon: Sword,
          value: `${items.owned} / ${items.total}`,
          tone: 'text-secondary',
        },
        {
          label: t('game.bosses'),
          icon: Swords,
          value:
            s?.bossesDefeated === undefined ? '—' : `${s.bossesDefeated} / ${OBJECTIVES.length}`,
          tone: 'text-magenta',
        },
      ]"
      :key="stat.label"
      class="panel panel-sm"
    >
      <div class="p-5">
        <p class="hud-label flex items-center gap-2">
          <component :is="stat.icon" class="size-3.5" :class="stat.tone" aria-hidden="true" />{{
            stat.label
          }}
        </p>
        <p class="num mt-2 text-3xl font-bold text-white">{{ stat.value }}</p>
      </div>
    </div>
  </div>
</template>
