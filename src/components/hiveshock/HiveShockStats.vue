<script setup lang="ts">
import { t } from '@/i18n'
import { computed } from 'vue'
import AnimatedNumber from '@/components/common/AnimatedNumber.vue'
import SkeletonBlock from '@/components/common/SkeletonBlock.vue'
import { useHiveShockStore } from '@/stores/hiveshock'

/** HIVESHOCK LIVE counters. Values come from HIVESHOCK_STATS_UPDATED / REST. */
const hive = useHiveShockStore()
const cells = computed(() => {
  const s = hive.stats
  return [
    { label: t('hiveshock.connected'), value: s?.connectedRacers },
    { label: t('hiveshock.gameEvents'), value: s?.gameEvents },
    { label: t('hiveshock.itemEvents'), value: s?.itemEvents },
    { label: t('hiveshock.progressEvents'), value: s?.progressEvents },
    { label: t('hiveshock.chatEvents'), value: s?.chatEvents },
  ]
})
</script>

<template>
  <div
    class="grid grid-cols-2 gap-3 sm:grid-cols-3 2xl:grid-cols-5"
    :aria-label="t('hiveshock.statsAria')"
  >
    <div
      v-for="(c, i) in cells"
      :key="c.label"
      class="panel panel-sm"
      :class="i === 0 && 'col-span-2 sm:col-span-1'"
    >
      <div class="p-4 sm:p-5">
        <p class="hud-label text-[10px]">{{ c.label }}</p>
        <SkeletonBlock v-if="c.value === undefined" class="mt-3 h-8 w-24" />
        <p v-else class="mt-2 text-3xl font-bold text-white sm:text-4xl">
          <AnimatedNumber :value="c.value" />
        </p>
      </div>
    </div>
  </div>
</template>
