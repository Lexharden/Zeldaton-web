<script setup lang="ts">
import { RouterLink } from 'vue-router'

/** Router link, external link or button with the shared chamfered style. */
withDefaults(
  defineProps<{
    to?: string
    href?: string
    variant?: 'primary' | 'ghost' | 'gold'
    size?: 'md' | 'sm'
  }>(),
  { variant: 'primary', size: 'md' },
)
</script>

<template>
  <RouterLink v-if="to" :to="to" class="btn" :class="[`btn-${variant}`, size === 'sm' && 'btn-sm']">
    <slot />
  </RouterLink>
  <a
    v-else-if="href"
    :href="href"
    target="_blank"
    rel="noopener noreferrer"
    class="btn"
    :class="[`btn-${variant}`, size === 'sm' && 'btn-sm']"
  >
    <slot />
  </a>
  <button v-else class="btn" type="button" :class="[`btn-${variant}`, size === 'sm' && 'btn-sm']">
    <slot />
  </button>
</template>
