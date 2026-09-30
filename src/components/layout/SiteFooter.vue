<script setup lang="ts">
import { t } from '@/i18n'
import { RouterLink } from 'vue-router'
import { SITE, SOCIAL_LINKS } from '@/config/event'
import ArtImage from '@/components/common/ArtImage.vue'
import { ART } from '@/config/artwork'
import PoweredBy from '@/components/hiveshock/PoweredBy.vue'

const links = [
  { label: 'footer.race', to: '/race' },
  { label: 'footer.racers', to: { path: '/', hash: '#racers' } },
  { label: 'footer.streams', to: '/streams' },
  { label: 'footer.rules', to: '/rules' },
  { label: 'footer.hiveshock', to: '/hiveshock' },
]
</script>

<template>
  <footer class="relative mt-16 overflow-hidden border-t border-line bg-surface/60">
    <div
      class="pointer-events-none absolute -bottom-10 left-1/2 -translate-x-1/2 select-none whitespace-nowrap font-display text-[clamp(6rem,24vw,22rem)] font-extrabold leading-none text-white/[0.035]"
      aria-hidden="true"
    >
      ZELDATHON
    </div>
    <div class="container-x relative grid gap-12 py-16 md:grid-cols-[1.4fr_1fr_1fr]">
      <div>
        <ArtImage
          :src="ART.logo"
          alt="The Legend of Zelda: Ocarina of Time"
          class="h-40 w-auto drop-shadow-[0_0_24px_rgb(245_196_81/0.25)]"
        >
          <p class="display text-6xl text-white">{{ SITE.name }}</p>
        </ArtImage>
        <p class="hud-label mt-3 !text-accent">{{ t('meta.tagline') }}</p>
        <div class="mt-8"><PoweredBy link /></div>
      </div>
      <nav :aria-label="t('footer.aria')">
        <p class="hud-label mb-4">{{ t('footer.explore') }}</p>
        <ul class="space-y-2.5">
          <li v-for="l in links" :key="l.label">
            <RouterLink
              :to="l.to"
              class="font-display text-xl font-semibold uppercase tracking-wider text-white/80 hover:text-accent"
              >{{ t(l.label) }}</RouterLink
            >
          </li>
        </ul>
      </nav>
      <div>
        <p class="hud-label mb-4">{{ t('footer.follow') }}</p>
        <ul class="space-y-2.5">
          <li v-for="s in SOCIAL_LINKS" :key="s.id">
            <a
              :href="s.href"
              target="_blank"
              rel="noopener noreferrer"
              class="font-display text-xl font-semibold uppercase tracking-wider text-white/80 hover:text-secondary"
              >{{ s.label }}</a
            >
          </li>
        </ul>
      </div>
    </div>
    <div class="relative border-t border-line">
      <div
        class="container-x flex flex-col gap-3 py-5 text-xs text-muted md:flex-row md:items-center md:justify-between"
      >
        <p class="max-w-2xl">{{ t('meta.disclaimer') }}</p>
        <p class="flex gap-5">
          <a href="#" class="hover:text-white">{{ t('footer.privacy') }}</a
          ><a href="#" class="hover:text-white">{{ t('footer.terms') }}</a>
        </p>
      </div>
    </div>
  </footer>
</template>
