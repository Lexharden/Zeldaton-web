<script setup lang="ts">
import { t } from '@/i18n'
import { useEventStore } from '@/stores/event'
import ResetFlow from './ResetFlow.vue'

/** THE RACE: explains the daily budget and the "use it or lose it" rule. */
const event = useEventStore()
</script>

<template>
  <section id="race" class="section" aria-labelledby="race-title">
    <div class="container-x">
      <div class="grid gap-10 lg:grid-cols-[0.9fr_1.1fr] lg:items-end">
        <header v-reveal>
          <p class="hud-label mb-4 flex items-center gap-3">
            <span class="text-accent">01</span><span class="h-px w-8 bg-primary/60" />{{
              t('race.eyebrow')
            }}
          </p>
          <h2 id="race-title" class="display text-[clamp(2.6rem,7vw,5.2rem)] text-white">
            {{ t('race.title1') }}<br /><span class="text-gradient">{{ t('race.title2') }}</span
            ><br />{{ t('race.title3') }}
          </h2>
        </header>
        <div v-reveal="1" class="space-y-6">
          <p class="max-w-xl text-base leading-relaxed text-muted md:text-lg">
            {{ t('race.body') }}
          </p>
          <div class="flex flex-wrap items-end gap-x-8 gap-y-4">
            <p class="display text-[clamp(5rem,16vw,9rem)] leading-[0.8] text-gold">
              {{ event.budgetHours || 4
              }}<span class="ml-2 text-[0.35em] text-white">{{ t('race.hours') }}</span>
            </p>
            <p
              class="max-w-[14rem] pb-1 font-display text-3xl font-bold uppercase leading-none tracking-wide text-white"
            >
              {{ t('race.useIt1') }}<br /><span class="text-accent">{{ t('race.useIt2') }}</span>
            </p>
          </div>
        </div>
      </div>
      <div class="mt-14"><ResetFlow /></div>
    </div>
  </section>
</template>
