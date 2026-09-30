<script setup lang="ts">
import { t } from '@/i18n'
import { computed } from 'vue'
import { Trophy, X } from 'lucide-vue-next'
import { useRaceStore } from '@/stores/race'
import { useRacersStore } from '@/stores/racers'
import { primaryChannel } from '@/utils/channels'
import { formatDuration } from '@/utils/format'

/** Special FINISH LINE moment shown when GAME_FINISHED arrives. Dismissible. */
const race = useRaceStore()
const racers = useRacersStore()
const racer = computed(() => racers.getById(race.winner?.racerId ?? ''))
const show = computed(() => !!race.winner && !race.winnerDismissed && !!racer.value)
</script>

<template>
  <Transition name="page">
    <div
      v-if="show && racer"
      class="fixed inset-0 z-[70] grid place-items-center bg-background/90 p-4 backdrop-blur-sm"
      role="dialog"
      aria-modal="true"
      :aria-label="t('finish.aria')"
    >
      <div class="panel panel-gold panel-lg w-full max-w-2xl">
        <div class="relative p-8 text-center sm:p-14">
          <button
            type="button"
            class="absolute right-4 top-4 text-muted hover:text-white"
            :aria-label="t('finish.close')"
            @click="race.winnerDismissed = true"
          >
            <X class="size-6" />
          </button>
          <Trophy class="mx-auto mb-4 size-14 text-accent" aria-hidden="true" />
          <p class="hud-label !text-accent">{{ t('finish.line') }}</p>
          <p class="display mt-3 text-6xl sm:text-8xl">
            <span class="text-gold">{{ racer.displayName }}</span>
          </p>
          <p class="mt-4 font-display text-2xl font-semibold uppercase tracking-widest text-white">
            {{ t('finish.completed') }}
          </p>
          <div v-if="race.winner?.finalTimeSeconds" class="mt-8">
            <p class="hud-label">{{ t('finish.finalTime') }}</p>
            <p class="num mt-1 text-5xl font-bold text-white">
              {{ formatDuration(race.winner.finalTimeSeconds) }}
            </p>
          </div>
          <a
            v-if="primaryChannel(racer)"
            :href="primaryChannel(racer)?.url"
            target="_blank"
            rel="noopener noreferrer"
            class="btn btn-gold mt-10"
            >{{ t('finish.celebration') }}</a
          >
        </div>
      </div>
    </div>
  </Transition>
</template>
