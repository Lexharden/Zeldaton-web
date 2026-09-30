<script setup lang="ts">
import { t } from '@/i18n'
import { computed } from 'vue'

/** Image avatar, or a generated sigil placeholder (replaceable by real avatarUrl). */
const props = withDefaults(
  defineProps<{ name: string; src?: string; seed?: string; size?: number }>(),
  { size: 56 },
)
const hue = computed(() => {
  let h = 0
  for (const c of props.seed ?? props.name) h = (h * 31 + c.charCodeAt(0)) % 360
  return h
})
const initials = computed(() =>
  props.name
    .split(/\s+/)
    .map((p) => p[0])
    .join('')
    .slice(0, 2)
    .toUpperCase(),
)
</script>

<template>
  <div
    class="relative shrink-0 overflow-hidden [clip-path:polygon(50%_0,100%_25%,100%_75%,50%_100%,0_75%,0_25%)]"
    :style="{ width: `${size}px`, height: `${size}px` }"
    role="img"
    :aria-label="t('racers.avatar', { name })"
  >
    <img v-if="src" :src="src" :alt="name" loading="lazy" class="size-full object-cover" />
    <div
      v-else
      class="grid size-full place-items-center font-display font-bold text-white"
      :style="{
        background: `linear-gradient(135deg, hsl(${hue} 70% 42%), hsl(${(hue + 60) % 360} 70% 28%))`,
        fontSize: `${size * 0.36}px`,
      }"
    >
      {{ initials }}
    </div>
  </div>
</template>
