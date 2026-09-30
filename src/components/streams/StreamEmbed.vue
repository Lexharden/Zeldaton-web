<script setup lang="ts">
import { Tv } from 'lucide-vue-next'
import { t } from '@/i18n'
import type { RacerChannel } from '@/types/racer'
import { PLATFORM_LABEL } from '@/utils/channels'
import ChannelButtons from '@/components/racers/ChannelButtons.vue'

/**
 * Placeholder for the embedded player. Twitch supports embeds; TikTok live is link-only.
 * Swap the body for a real Twitch iframe once the parent domain is known; props stay the same.
 */
const props = defineProps<{ channels: RacerChannel[]; primary?: RacerChannel; name: string }>()
</script>

<template>
  <div class="panel panel-lg">
    <div
      class="relative grid aspect-video max-h-[420px] w-full place-items-center bg-gradient-to-br from-primary/20 via-surface to-background"
    >
      <div
        class="absolute inset-0 opacity-25"
        style="
          background: repeating-linear-gradient(
            135deg,
            transparent 0 18px,
            rgb(76 154 82 / 0.2) 18px 19px
          );
        "
      />
      <div v-if="props.primary" class="relative z-10 px-4 text-center">
        <Tv class="mx-auto mb-3 size-10 text-secondary" aria-hidden="true" />
        <p class="display text-3xl text-white">
          {{ t('streams.embedOn', { name, platform: PLATFORM_LABEL[props.primary.platform] }) }}
        </p>
        <p class="mx-auto mt-2 max-w-sm text-sm text-muted">{{ t('streams.embedPlaceholder') }}</p>
        <div class="mt-5 flex justify-center">
          <ChannelButtons :channels="channels" :block="false" live />
        </div>
      </div>
      <p v-else class="relative z-10 text-muted">{{ t('streams.noStream') }}</p>
    </div>
  </div>
</template>
