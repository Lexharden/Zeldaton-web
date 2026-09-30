<script setup lang="ts">
import { onMounted } from 'vue'
import { useRaceData } from '@/composables/useRaceData'
import DefaultLayout from '@/layouts/DefaultLayout.vue'

// Data flow starts once at the root: REST -> stores, socket -> dispatcher -> stores.
const { start } = useRaceData()
onMounted(start)
</script>

<template>
  <DefaultLayout />
</template>
