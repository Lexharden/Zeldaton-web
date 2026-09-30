<script setup lang="ts">
import { RouterLink } from 'vue-router'
import { t } from '@/i18n'
import {
  acceptAnalytics,
  analyticsEnabled,
  bannerOpen,
  rejectAnalytics,
} from '@/analytics/analytics'

/**
 * Cookie notice. Only exists when Google Analytics is configured. "Accept" and "Reject" weigh the
 * same (no pre-ticked boxes, no dark patterns); the choice can be changed from the footer.
 */
</script>

<template>
  <Transition name="cookie">
    <section
      v-if="analyticsEnabled && bannerOpen"
      class="fixed inset-x-3 bottom-3 z-[80] mx-auto max-w-3xl border border-line bg-surface/95 p-5 shadow-2xl backdrop-blur-md sm:inset-x-6 sm:bottom-6"
      role="dialog"
      aria-live="polite"
      aria-labelledby="cookie-title"
      aria-describedby="cookie-text"
    >
      <h2 id="cookie-title" class="hud-label !text-accent">{{ t('consent.title') }}</h2>
      <p id="cookie-text" class="mt-2 text-sm text-white/85">
        {{ t('consent.text') }}
        <RouterLink to="/privacy" class="text-secondary underline hover:text-white">{{
          t('consent.more')
        }}</RouterLink>
      </p>
      <div class="mt-4 flex flex-wrap gap-3">
        <button
          type="button"
          class="border border-accent bg-accent px-5 py-2 font-display text-lg font-semibold uppercase tracking-wider text-black hover:brightness-110"
          @click="acceptAnalytics"
        >
          {{ t('consent.accept') }}
        </button>
        <button
          type="button"
          class="border border-white/40 px-5 py-2 font-display text-lg font-semibold uppercase tracking-wider text-white hover:border-white"
          @click="rejectAnalytics"
        >
          {{ t('consent.reject') }}
        </button>
      </div>
    </section>
  </Transition>
</template>

<style scoped>
.cookie-enter-active,
.cookie-leave-active {
  transition:
    opacity 0.25s ease,
    transform 0.25s ease;
}
.cookie-enter-from,
.cookie-leave-to {
  opacity: 0;
  transform: translateY(12px);
}
</style>
