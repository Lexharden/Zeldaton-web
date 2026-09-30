<script setup lang="ts">
import { watch } from 'vue'
import { RouterView, useRoute, useRouter } from 'vue-router'
import { useRaceData } from '@/composables/useRaceData'
import DefaultLayout from '@/layouts/DefaultLayout.vue'

// Data flow starts once at the root: REST -> stores, socket -> dispatcher -> stores. The organizer
// panel (`bare` routes) has its own chrome and its own data, so it does not open the public socket.
const route = useRoute()
const router = useRouter()
const { start } = useRaceData()
// Wait for the first navigation: until then the route has no meta and /admin would look public.
void router.isReady().then(() =>
  watch(
    () => route.meta.bare,
    (bare) => {
      if (!bare) start()
    },
    { immediate: true },
  ),
)
</script>

<template>
  <RouterView v-if="route.meta.bare" />
  <DefaultLayout v-else />
</template>
