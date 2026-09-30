<script setup lang="ts">
/** Angled esports-style separator between sections. */
defineProps<{ flip?: boolean }>()
</script>

<template>
  <div class="divider-diag" :class="flip && 'scale-x-[-1]'" aria-hidden="true" />
</template>
