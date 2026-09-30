<script setup lang="ts">
import { t } from '@/i18n'
import type { RacerChannel } from '@/types/racer'
import { PLATFORM_LABEL } from '@/utils/channels'
import PlatformIcon from './PlatformIcon.vue'

/**
 * One link per channel (Twitch and/or TikTok). Racers with a single platform show a single
 * button; racers without channels show a neutral note instead of a dead link.
 */
withDefaults(defineProps<{ channels: RacerChannel[]; live?: boolean; block?: boolean }>(), {
  live: false,
  block: true,
})
</script>

<template>
  <div v-if="channels.length" class="flex flex-wrap gap-2" :class="block && '[&>a]:flex-1'">
    <a
      v-for="c in channels"
      :key="c.platform"
      :href="c.url"
      target="_blank"
      rel="noopener noreferrer"
      class="btn btn-sm min-w-0"
      :class="live && c.platform === 'twitch' ? 'btn-primary' : 'btn-ghost'"
      :aria-label="t('streams.watchOn', { platform: PLATFORM_LABEL[c.platform] })"
    >
      <PlatformIcon :platform="c.platform" />{{ PLATFORM_LABEL[c.platform] }}
    </a>
  </div>
  <p v-else class="text-center font-mono text-[11px] uppercase tracking-wider text-muted">
    {{ t('streams.noChannel') }}
  </p>
</template>
