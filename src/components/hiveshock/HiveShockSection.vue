<script setup lang="ts">
import { t } from '@/i18n'
import { RouterLink } from 'vue-router'
import { ART } from '@/config/artwork'
import GamingBackground from '@/components/common/GamingBackground.vue'
import EventTerminal from './EventTerminal.vue'
import HiveFlow from './HiveFlow.vue'
import HiveShockStats from './HiveShockStats.vue'
import PoweredBy from './PoweredBy.vue'

/** Home teaser: THE GAME IS CONNECTED + flow + live counters + event feed. HiveShock supports the race, it does not lead it. */
</script>

<template>
  <section id="hiveshock" class="section overflow-hidden" aria-labelledby="hs-title">
    <GamingBackground shapes :art="ART.bgTemple" />
    <div class="container-x">
      <div class="grid items-end gap-8 lg:grid-cols-[1.1fr_0.9fr]">
        <header v-reveal>
          <p class="hud-label mb-4 flex items-center gap-3">
            <span class="text-accent">06</span><span class="h-px w-8 bg-primary/60" />{{
              t('hiveshock.eyebrow')
            }}
          </p>
          <h2 id="hs-title" class="display text-[clamp(2.6rem,7vw,5.2rem)] text-white">
            <span class="block">{{ t('hiveshock.title1') }}</span
            ><span class="block">{{ t('hiveshock.title2') }}</span
            ><span class="text-gradient block">{{ t('hiveshock.title3') }}</span>
          </h2>
        </header>
        <div v-reveal="1" class="pb-2">
          <p class="text-lg text-white/85">
            {{ t('hiveshock.tagline') }}
          </p>
          <div class="mt-6 flex flex-wrap items-center gap-6">
            <PoweredBy />
            <RouterLink to="/hiveshock" class="btn btn-ghost btn-sm">{{
              t('hiveshock.learnMore')
            }}</RouterLink>
          </div>
        </div>
      </div>

      <div class="mt-14"><HiveFlow /></div>

      <div class="mt-14 grid gap-8 lg:grid-cols-[1fr_1.1fr] lg:items-start">
        <div>
          <p class="hud-label mb-4">{{ t('hiveshock.live') }}</p>
          <HiveShockStats />
        </div>
        <EventTerminal />
      </div>
    </div>
  </section>
</template>
