<script setup lang="ts">
import { RouterLink } from 'vue-router'
import ArtImage from '@/components/common/ArtImage.vue'
import { ART } from '@/config/artwork'
import { t } from '@/i18n'

/** "Powered by HiveShock" badge. Free to use, proprietary; never claims otherwise. */
defineProps<{ link?: boolean }>()
</script>

<template>
  <component
    :is="link ? RouterLink : 'div'"
    v-bind="link ? { to: '/hiveshock' } : {}"
    class="inline-block"
  >
    <div class="flex items-center gap-4">
      <ArtImage
        :src="ART.hiveshockLogo"
        alt="HiveShock"
        class="size-16 shrink-0 object-contain drop-shadow-[0_0_14px_rgb(245_196_81/0.35)]"
      >
        <span
          class="grid size-11 rotate-45 place-items-center border border-primary/60 bg-primary/15"
        >
          <span class="size-3 bg-secondary" />
        </span>
      </ArtImage>
      <div>
        <p class="hud-label text-[10px]">{{ t('hiveshock.poweredBy') }}</p>
        <p class="display text-3xl text-white">HIVESHOCK</p>
        <p class="text-xs text-muted">{{ t('hiveshock.free') }}</p>
      </div>
    </div>
  </component>
</template>
