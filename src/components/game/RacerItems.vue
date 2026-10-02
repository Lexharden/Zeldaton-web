<script setup lang="ts">
import { t } from '@/i18n'
import { Check } from 'lucide-vue-next'
import AgeBadge from '@/components/common/AgeBadge.vue'
import ArtImage from '@/components/common/ArtImage.vue'
import { ART } from '@/config/artwork'
import { useCatalogStore } from '@/stores/catalog'
import type { Racer } from '@/types/racer'
import { countOwned } from '@/utils/catalog'
import { itemIconUrl } from '@/utils/media'

/**
 * Items organised by Link: child, adult and what both share, each split by category (weapons,
 * boots, songs...). Driven by the catalog store, so an organizer's changes appear live. Items missing
 * from telemetry count as "not acquired"; ids the catalog does not know are ignored.
 */
const props = defineProps<{ racer: Racer }>()
const catalog = useCatalogStore()
const has = (id: string) => Boolean(props.racer.items?.[id])
const icon = (id: string, custom?: string) => (custom ? itemIconUrl(custom) : ART.item(id))
</script>

<template>
  <div class="space-y-6">
    <section
      v-for="section in catalog.itemSections"
      :key="section.age"
      :aria-label="t(`age.${section.age}`)"
      class="rounded-sm border-l-2 pl-4"
      :class="{
        'border-success/60': section.age === 'child',
        'border-secondary/60': section.age === 'adult',
        'border-white/15': section.age === 'both',
      }"
    >
      <header class="mb-3 flex items-center justify-between gap-3">
        <AgeBadge :age="section.age" />
        <span class="num text-sm font-semibold text-white"
          >{{ countOwned(section.items, racer.items).owned }}
          <span class="text-muted">/ {{ section.items.length }}</span></span
        >
      </header>

      <div v-for="g in section.groups" :key="g.group" class="mb-4 last:mb-0">
        <h3 class="hud-label mb-2 text-[10px]">{{ t(`itemGroups.${g.group}`) }}</h3>
        <ul class="grid grid-cols-4 gap-2 sm:grid-cols-6 lg:grid-cols-4 xl:grid-cols-6">
          <li
            v-for="item in g.items"
            :key="item.id"
            class="relative aspect-square [clip-path:polygon(10%_0,100%_0,100%_90%,90%_100%,0_100%,0_10%)] transition-all duration-300"
            :class="has(item.id) ? 'bg-primary/20 ring-1 ring-primary/50' : 'bg-white/[0.03]'"
            :title="`${catalog.label(item)}: ${has(item.id) ? t('game.acquired') : t('game.notAcquired')}`"
          >
            <div class="flex h-full flex-col items-center justify-center gap-0.5 p-1 text-center">
              <ArtImage
                :src="icon(item.id, item.icon)"
                :alt="catalog.label(item)"
                class="size-8 object-contain"
                :class="has(item.id) ? '' : 'opacity-25 grayscale'"
              >
                <span
                  class="num text-lg font-bold"
                  :class="has(item.id) ? 'text-white' : 'text-white/20'"
                  >{{ item.short }}</span
                >
              </ArtImage>
              <span
                class="line-clamp-2 text-[8px] leading-tight uppercase tracking-wider"
                :class="has(item.id) ? 'text-muted' : 'text-white/20'"
                >{{ catalog.label(item) }}</span
              >
            </div>
            <Check
              v-if="has(item.id)"
              class="absolute right-1 top-1 size-3 text-success"
              aria-hidden="true"
            />
            <span class="sr-only">{{
              has(item.id) ? t('game.acquired') : t('game.notAcquired')
            }}</span>
          </li>
        </ul>
      </div>
    </section>
  </div>
</template>
