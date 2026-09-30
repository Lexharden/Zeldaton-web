<script setup lang="ts">
import { ref, watch } from 'vue'

/**
 * Optional artwork: renders nothing (or the fallback slot) if the file is missing, so the
 * layout never shows a broken image icon.
 */
defineOptions({ inheritAttrs: false })
const props = withDefaults(defineProps<{ src: string; alt?: string; eager?: boolean }>(), {
  alt: '',
})
const missing = ref(false)
watch(
  () => props.src,
  () => (missing.value = false),
)
</script>

<template>
  <img
    v-if="!missing"
    v-bind="$attrs"
    :src="src"
    :alt="alt"
    :aria-hidden="alt ? undefined : true"
    :loading="eager ? 'eager' : 'lazy'"
    decoding="async"
    draggable="false"
    @error="missing = true"
  />
  <slot v-else />
</template>
