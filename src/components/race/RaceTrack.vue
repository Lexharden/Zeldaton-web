<script setup lang="ts">
import { t } from '@/i18n'
import { computed } from 'vue'
import { OBJECTIVES } from '@/config/event'
import { useRaceStore } from '@/stores/race'
import { useRacersStore } from '@/stores/racers'
import { trackPosition } from '@/utils/status'
import { RouterLink } from 'vue-router'
import Avatar from '@/components/common/Avatar.vue'

/**
 * START -> objectives -> FINISH track. Racer markers sit at completedObjectives / total
 * (falling back to percentage only when no objective data exists). Markers on the same
 * node are stacked so nobody is hidden.
 */
const racers = useRacersStore()
const race = useRaceStore()

const nodes = computed(() => [
  { id: 'start', label: t('game.startEdge'), kind: 'edge' as const },
  ...OBJECTIVES.map((o) => ({ id: o.id, label: t(`objectives.${o.id}`), kind: 'node' as const })),
  { id: 'finish', label: t('game.finishEdge'), kind: 'edge' as const },
])

const markers = computed(() => {
  const stackAt: Record<number, number> = {}
  return race.standings.flatMap((s) => {
    const r = racers.getById(s.racerId)
    if (!r) return []
    const fraction = trackPosition(
      r.completedObjectives.length,
      OBJECTIVES.length,
      r.progressPercentage,
    )
    // node index 0..total+1 -> fraction along the track
    const slot =
      r.status === 'finished' ? OBJECTIVES.length + 1 : Math.round(fraction * OBJECTIVES.length)
    const stack = (stackAt[slot] = (stackAt[slot] ?? -1) + 1)
    return [{ racer: r, rank: s.rank, slot, stack }]
  })
})
const trackY = computed(() => 64 + Math.max(0, ...markers.value.map((m) => m.stack)) * 46)
const leadFraction = computed(() =>
  markers.value.length
    ? Math.max(...markers.value.map((m) => m.slot)) / (nodes.value.length - 1)
    : 0,
)
const pct = (slot: number) => `${(slot / (nodes.value.length - 1)) * 100}%`
</script>

<template>
  <div class="panel panel-lg" role="group" :aria-label="t('game.track')">
    <div class="overflow-x-auto p-6 sm:p-8">
      <!-- desktop: horizontal. Stacked markers rise above the track so nobody is hidden. -->
      <div class="relative hidden min-w-[900px] lg:block" :style="{ height: `${trackY + 72}px` }">
        <div class="absolute inset-x-6 h-1 bg-white/10" :style="{ top: `${trackY}px` }" />
        <div
          class="absolute left-6 h-1 bg-gradient-to-r from-primary to-secondary transition-[width] duration-700"
          :style="{ top: `${trackY}px`, width: `calc((100% - 48px) * ${leadFraction})` }"
        />
        <div class="absolute inset-x-6 top-0 h-full">
          <div
            v-for="(n, i) in nodes"
            :key="n.id"
            class="absolute -translate-x-1/2"
            :style="{ left: pct(i), top: `${trackY - 6}px` }"
          >
            <span
              class="mx-auto block size-4 rotate-45 border-2"
              :class="
                n.kind === 'edge' ? 'border-accent bg-accent/30' : 'border-primary bg-background'
              "
            />
            <span
              class="hud-label mt-4 block w-20 text-center text-[9px] leading-tight"
              :class="n.kind === 'edge' && '!text-accent'"
              >{{ n.label }}</span
            >
          </div>
          <RouterLink
            v-for="m in markers"
            :key="m.racer.id"
            :to="`/racer/${m.racer.id}`"
            class="group absolute flex -translate-x-1/2 flex-col items-center transition-[left,top] duration-700 ease-out"
            :style="{ left: pct(m.slot), top: `${trackY - 56 - m.stack * 46}px` }"
            :title="`${m.racer.displayName} — #${m.rank}`"
          >
            <Avatar
              :name="m.racer.displayName"
              :src="m.racer.avatarUrl"
              :seed="m.racer.id"
              :size="40"
              class="transition group-hover:scale-110"
            />
            <span class="mt-0.5 h-2 w-px bg-secondary" />
          </RouterLink>
        </div>
      </div>

      <!-- mobile / tablet: vertical -->
      <ol class="relative space-y-0 lg:hidden">
        <li v-for="(n, i) in nodes" :key="n.id" class="relative flex gap-4 pb-6 last:pb-0">
          <span
            v-if="i < nodes.length - 1"
            class="absolute left-[7px] top-4 h-full w-px bg-white/12"
            aria-hidden="true"
          />
          <span
            class="relative z-10 mt-1 size-4 shrink-0 rotate-45 border-2"
            :class="
              n.kind === 'edge' ? 'border-accent bg-accent/40' : 'border-primary bg-background'
            "
          />
          <div class="min-w-0 flex-1">
            <p
              class="font-display text-lg font-semibold uppercase tracking-wide"
              :class="n.kind === 'edge' ? 'text-accent' : 'text-white'"
            >
              {{ n.label }}
            </p>
            <div class="mt-1 flex flex-wrap gap-2">
              <RouterLink
                v-for="m in markers.filter((x) => x.slot === i)"
                :key="m.racer.id"
                :to="`/racer/${m.racer.id}`"
                class="flex items-center gap-1.5 rounded-full bg-primary/15 py-0.5 pl-0.5 pr-2.5 text-xs text-white ring-1 ring-primary/40"
              >
                <Avatar :name="m.racer.displayName" :seed="m.racer.id" :size="20" />{{
                  m.racer.displayName
                }}
              </RouterLink>
            </div>
          </div>
        </li>
      </ol>
    </div>
  </div>
</template>
