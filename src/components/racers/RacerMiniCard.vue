<script setup lang="ts">
import { RouterLink } from 'vue-router'
import Avatar from '@/components/common/Avatar.vue'
import LiveBadge from '@/components/common/LiveBadge.vue'
import { useRacer } from '@/composables/useRacer'
import { formatPercent } from '@/utils/format'

/** Small pill-style racer reference (podium, header chips). */
const props = defineProps<{ racerId: string }>()
const { racer, rank } = useRacer(() => props.racerId)
</script>

<template>
  <RouterLink
    v-if="racer"
    :to="`/racer/${racer.id}`"
    class="panel panel-sm panel-hover flex items-center gap-3 p-3"
  >
    <span class="display w-6 text-center text-2xl text-accent">{{ rank }}</span>
    <Avatar :name="racer.displayName" :src="racer.avatarUrl" :seed="racer.id" :size="36" />
    <span class="min-w-0 flex-1">
      <span class="block truncate font-display text-lg font-bold uppercase text-white">{{
        racer.displayName
      }}</span>
      <span class="num text-xs text-muted">{{ formatPercent(racer.progressPercentage) }}</span>
    </span>
    <LiveBadge :status="racer.status" size="sm" />
  </RouterLink>
</template>
