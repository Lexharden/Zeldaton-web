<script setup lang="ts">
import { t } from '@/i18n'
import { computed } from 'vue'
import { RouterView, useRoute } from 'vue-router'
import EventHeader from '@/components/layout/EventHeader.vue'
import NavBar from '@/components/layout/NavBar.vue'
import SiteFooter from '@/components/layout/SiteFooter.vue'
import NaviCursor from '@/components/common/NaviCursor.vue'
import CookieBanner from '@/components/common/CookieBanner.vue'
import FinishOverlay from '@/components/race/FinishOverlay.vue'

const route = useRoute()
const showHeader = computed(() => route.meta.eventHeader !== false)
</script>

<template>
  <div class="relative min-h-screen">
    <a
      href="#main"
      class="sr-only z-[60] bg-accent px-4 py-2 text-black focus:not-sr-only focus:fixed focus:left-3 focus:top-3"
      >{{ t('nav.skip') }}</a
    >
    <NavBar />
    <EventHeader v-if="showHeader" />
    <main id="main">
      <RouterView v-slot="{ Component, route: r }">
        <Transition name="page" mode="out-in">
          <component :is="Component" :key="r.path" />
        </Transition>
      </RouterView>
    </main>
    <SiteFooter />
    <FinishOverlay />
    <NaviCursor />
    <CookieBanner />
  </div>
</template>
