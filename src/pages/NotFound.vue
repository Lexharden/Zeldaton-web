<script setup lang="ts">
import { t } from '@/i18n'
import GamingBackground from '@/components/common/GamingBackground.vue'
import Button from '@/components/ui/Button.vue'
import { useSeo } from '@/composables/useSeo'

useSeo(() => t('meta.pages.notFound'))
</script>

<template>
  <section
    class="relative grid min-h-[70vh] place-items-center overflow-hidden px-4 pt-16 text-center"
  >
    <GamingBackground particles />
    <div>
      <p class="hud-label !text-accent">{{ t('notFound.code') }}</p>
      <p class="display mt-2 text-[clamp(6rem,24vw,14rem)] text-gradient">
        {{ t('notFound.title') }}
      </p>
      <p class="mb-8 font-display text-2xl uppercase tracking-widest text-white">
        {{ t('notFound.text') }}
      </p>
      <Button to="/">{{ t('notFound.back') }}</Button>
    </div>
  </section>
</template>
