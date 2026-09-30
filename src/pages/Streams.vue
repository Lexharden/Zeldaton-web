<script setup lang="ts">
import { splitLastWord } from '@/utils/template'
import { t } from '@/i18n'
import { ref } from 'vue'
import { useRoute } from 'vue-router'
import GamingBackground from '@/components/common/GamingBackground.vue'
import StreamGrid from '@/components/streams/StreamGrid.vue'
import { useSeo } from '@/composables/useSeo'
import { useRaceStore } from '@/stores/race'

useSeo(
  () => t('meta.pages.streams'),
  () => t('meta.pages.streamsDesc'),
)
const route = useRoute()
const race = useRaceStore()
const liveOnly = ref(route.query.live === '1')
</script>

<template>
  <div class="relative pb-20">
    <GamingBackground shapes />
    <div class="container-x pt-10 sm:pt-14">
      <header class="mb-10 flex flex-wrap items-end justify-between gap-6">
        <div>
          <p class="hud-label mb-3">{{ t('streams.eyebrow') }}</p>
          <h1 class="display text-[clamp(2.8rem,8vw,5.5rem)] text-white">
            {{ splitLastWord(t('streams.title'))[0] }}
            <span class="text-gradient">{{ splitLastWord(t('streams.title'))[1] }}</span>
          </h1>
        </div>
        <div class="flex gap-2" role="group" :aria-label="t('streams.filter')">
          <button
            type="button"
            class="btn btn-sm"
            :class="!liveOnly ? 'btn-primary' : 'btn-ghost'"
            :aria-pressed="!liveOnly"
            @click="liveOnly = false"
          >
            {{ t('streams.all', { n: race.streams.length }) }}
          </button>
          <button
            type="button"
            class="btn btn-sm"
            :class="liveOnly ? 'btn-primary' : 'btn-ghost'"
            :aria-pressed="liveOnly"
            @click="liveOnly = true"
          >
            {{ t('streams.watchLive', { n: race.streams.filter((s) => s.isLive).length }) }}
          </button>
        </div>
      </header>
      <StreamGrid :live-only="liveOnly" />
    </div>
  </div>
</template>
