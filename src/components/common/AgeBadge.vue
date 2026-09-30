<script setup lang="ts">
import { t } from '@/i18n'
import { computed } from 'vue'
import type { CatalogAge } from '@/types/catalog'

/**
 * Which Link something belongs to (or is being played as). Green = child (Kokiri), blue = adult
 * (Hylian), neutral = both. `short` shows CHILD / ADULT instead of the full name.
 */
const props = defineProps<{ age: CatalogAge; short?: boolean; prefix?: string }>()

const label = computed(() =>
  props.short
    ? t(
        props.age === 'child'
          ? 'age.childShort'
          : props.age === 'adult'
            ? 'age.adultShort'
            : 'age.both',
      )
    : t(`age.${props.age}`),
)
const tone = computed(
  () =>
    ({
      child: 'bg-success/15 text-success ring-success/40',
      adult: 'bg-secondary/15 text-secondary ring-secondary/40',
      both: 'bg-white/5 text-muted ring-white/15',
    })[props.age],
)
</script>

<template>
  <span
    class="inline-flex items-center gap-1.5 rounded-sm px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wider ring-1"
    :class="tone"
  >
    <span v-if="prefix" class="opacity-70">{{ prefix }}</span>
    {{ label }}
  </span>
</template>
