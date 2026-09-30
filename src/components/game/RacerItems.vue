<script setup lang="ts">
import { t } from '@/i18n'
import { Check } from 'lucide-vue-next'
import ArtImage from '@/components/common/ArtImage.vue'
import { ART } from '@/config/artwork'
import { ITEMS } from '@/config/event'
import type { Racer } from '@/types/racer'

/**
 * Item grid driven by the ITEMS catalog. Items missing from telemetry count as "not acquired";
 * unknown ids from the backend are ignored, so the catalog can grow independently.
 */
const props = defineProps<{ racer: Racer }>()
const has = (id: string) => Boolean(props.racer.items?.[id])
</script>

<template>
  <ul class="grid grid-cols-3 gap-2 sm:grid-cols-5 lg:grid-cols-3 xl:grid-cols-5">
    <li
      v-for="item in ITEMS"
      :key="item.id"
      class="relative aspect-square [clip-path:polygon(10%_0,100%_0,100%_90%,90%_100%,0_100%,0_10%)] transition-all duration-300"
      :class="has(item.id) ? 'bg-primary/20 ring-1 ring-primary/50' : 'bg-white/[0.03]'"
      :title="`${t('items.' + item.id)}: ${has(item.id) ? t('game.acquired') : t('game.notAcquired')}`"
    >
      <div class="flex h-full flex-col items-center justify-center gap-1 p-1 text-center">
        <ArtImage
          :src="ART.item(item.id)"
          :alt="t('items.' + item.id)"
          class="size-9 object-contain"
          :class="has(item.id) ? '' : 'opacity-25 grayscale'"
        >
          <span
            class="num text-xl font-bold"
            :class="has(item.id) ? 'text-white' : 'text-white/20'"
            >{{ item.short }}</span
          >
        </ArtImage>
        <span
          class="text-[9px] leading-tight uppercase tracking-wider"
          :class="has(item.id) ? 'text-muted' : 'text-white/20'"
          >{{ t('items.' + item.id) }}</span
        >
      </div>
      <Check
        v-if="has(item.id)"
        class="absolute right-1 top-1 size-3 text-success"
        aria-hidden="true"
      />
      <span class="sr-only">{{ has(item.id) ? t('game.acquired') : t('game.notAcquired') }}</span>
    </li>
  </ul>
</template>
