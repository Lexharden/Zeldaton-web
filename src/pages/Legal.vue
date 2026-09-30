<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import { t, tm } from '@/i18n'
import GamingBackground from '@/components/common/GamingBackground.vue'
import SectionHeader from '@/components/common/SectionHeader.vue'
import { useSeo } from '@/composables/useSeo'
import { analyticsEnabled, consent, openCookieSettings } from '@/analytics/analytics'

/** Privacy & cookies (/privacy) and terms (/terms): same layout, text from the i18n catalogs. */
interface Section {
  h: string
  p: string
  items?: string[]
}

const route = useRoute()
const doc = computed(() => (route.meta.doc === 'terms' ? 'terms' : 'privacy'))
const sections = computed(() => tm<Section[]>(`legal.${doc.value}.sections`))

useSeo(
  () => t(`legal.${doc.value}.title`),
  () => t(`legal.${doc.value}.intro`),
)

const consentText = computed(() =>
  consent.value === 'granted'
    ? t('consent.statusGranted')
    : consent.value === 'denied'
      ? t('consent.statusDenied')
      : t('consent.statusNone'),
)
</script>

<template>
  <div class="relative pb-20">
    <GamingBackground />
    <div class="container-x pt-10 sm:pt-14">
      <SectionHeader
        :eyebrow="t('legal.eyebrow')"
        :title="t(`legal.${doc}.title`)"
        :subtitle="t(`legal.${doc}.intro`)"
      />
      <p class="hud-label -mt-6 mb-10">{{ t('legal.updated') }}</p>

      <div
        v-if="doc === 'privacy' && analyticsEnabled"
        class="panel mb-10 flex max-w-3xl flex-wrap items-center gap-4 p-5"
      >
        <p class="flex-1 text-sm text-white/85">{{ consentText }}</p>
        <button
          type="button"
          class="border border-accent px-4 py-2 font-display text-lg font-semibold uppercase tracking-wider text-accent hover:bg-accent hover:text-black"
          @click="openCookieSettings"
        >
          {{ t('footer.cookies') }}
        </button>
      </div>

      <article class="max-w-3xl space-y-9">
        <section v-for="(s, i) in sections" :key="i">
          <h2 class="display text-3xl text-white">{{ s.h }}</h2>
          <p class="mt-3 leading-relaxed text-white/80">{{ s.p }}</p>
          <ul v-if="s.items?.length" class="mt-3 list-disc space-y-2 pl-5 text-white/80">
            <li v-for="(item, j) in s.items" :key="j" class="leading-relaxed">{{ item }}</li>
          </ul>
        </section>
      </article>
    </div>
  </div>
</template>
