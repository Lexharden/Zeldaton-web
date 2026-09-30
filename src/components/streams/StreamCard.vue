<script setup lang="ts">
import { t, tx } from '@/i18n'
import { computed } from 'vue'
import { RouterLink } from 'vue-router'
import Avatar from '@/components/common/Avatar.vue'
import LiveBadge from '@/components/common/LiveBadge.vue'
import ProgressBar from '@/components/common/ProgressBar.vue'
import ChannelButtons from '@/components/racers/ChannelButtons.vue'
import PlatformIcon from '@/components/racers/PlatformIcon.vue'
import { useRacer } from '@/composables/useRacer'
import { formatPercent } from '@/utils/format'

/** Stream tile: thumbnail (or generated placeholder), live state, progress and watch link. */
const props = defineProps<{ racerId: string }>()
const { racer, rank } = useRacer(() => props.racerId)
const isLive = computed(() => racer.value?.stream?.isLive ?? false)
const badge = computed(() =>
  racer.value?.status === 'paused'
    ? 'paused'
    : isLive.value
      ? 'live'
      : racer.value?.status === 'exhausted'
        ? 'exhausted'
        : 'offline',
)
</script>

<template>
  <article v-if="racer" class="panel panel-hover h-full">
    <div class="flex h-full flex-col">
      <div
        class="relative aspect-video overflow-hidden bg-gradient-to-br from-primary/30 via-surface-elevated to-background"
      >
        <img
          v-if="racer.stream?.thumbnailUrl"
          :src="racer.stream.thumbnailUrl"
          :alt="t('streams.stream', { name: racer.displayName })"
          loading="lazy"
          class="size-full object-cover"
        />
        <div v-else class="absolute inset-0 grid place-items-center">
          <div
            class="absolute inset-0 opacity-30"
            style="
              background: repeating-linear-gradient(
                135deg,
                transparent 0 14px,
                rgb(76 154 82 / 0.15) 14px 15px
              );
            "
          />
          <Avatar :name="racer.displayName" :src="racer.avatarUrl" :seed="racer.id" :size="64" />
        </div>
        <div class="absolute left-3 top-3"><LiveBadge :status="badge" size="sm" /></div>
        <span
          class="absolute bottom-3 left-3 flex items-center gap-2 rounded bg-black/60 px-2 py-1 text-white"
        >
          <PlatformIcon v-for="c in racer.channels" :key="c.platform" :platform="c.platform" />
        </span>
        <span
          v-if="racer.stream?.viewers && isLive"
          class="absolute bottom-3 right-3 rounded bg-black/60 px-2 py-1 font-mono text-[10px] text-white"
          >{{ t('streams.watching', { n: racer.stream.viewers }) }}</span
        >
      </div>
      <div class="flex flex-1 flex-col gap-4 p-5">
        <div class="flex items-center justify-between gap-2">
          <h3 class="display text-2xl text-white">
            <RouterLink :to="`/racer/${racer.id}`" class="hover:text-secondary">{{
              racer.displayName
            }}</RouterLink>
          </h3>
          <span class="font-mono text-xs text-accent">#{{ rank }}</span>
        </div>
        <div>
          <div class="mb-1.5 flex justify-between">
            <span class="hud-label text-[10px]">{{ t('racers.gameProgress') }}</span
            ><span class="num text-sm font-bold text-white">{{
              formatPercent(racer.progressPercentage)
            }}</span>
          </div>
          <ProgressBar
            :value="racer.progressPercentage"
            thin
            :segments="24"
            :label="t('racers.gameProgress')"
          />
        </div>
        <div>
          <p class="hud-label text-[10px]">{{ t('streams.currentArea') }}</p>
          <p class="mt-0.5 truncate font-display text-lg font-semibold uppercase text-secondary">
            {{ racer.currentArea ? tx('areas', racer.currentArea) : '—' }}
          </p>
        </div>
        <div class="mt-auto"><ChannelButtons :channels="racer.channels" :live="isLive" /></div>
      </div>
    </div>
  </article>
</template>
