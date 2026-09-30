<script setup lang="ts">
import { AlertTriangle, Ghost, Radio, WifiOff } from 'lucide-vue-next'
import { computed } from 'vue'
import { t } from '@/i18n'

/** Consistent empty / error / offline block. */
const props = defineProps<{
  variant: 'connection-lost' | 'backend' | 'no-data' | 'racer-offline'
  title?: string
  text?: string
  actionLabel?: string
}>()
defineEmits<{ action: [] }>()

const presets = {
  'connection-lost': { icon: WifiOff, key: 'states.connectionLost' },
  backend: { icon: AlertTriangle, key: 'states.backend' },
  'no-data': { icon: Radio, key: 'states.noData' },
  'racer-offline': { icon: Ghost, key: 'states.racerOffline' },
} as const
const preset = computed(() => presets[props.variant])
</script>

<template>
  <div class="panel panel-sm" role="status">
    <div class="flex flex-col items-center gap-3 px-6 py-10 text-center">
      <component :is="preset.icon" class="size-8 text-primary" aria-hidden="true" />
      <p class="display text-2xl text-white">{{ title ?? t(`${preset.key}.title`) }}</p>
      <p class="max-w-sm text-sm text-muted">{{ text ?? t(`${preset.key}.text`) }}</p>
      <button
        v-if="actionLabel"
        class="btn btn-ghost btn-sm mt-2"
        type="button"
        @click="$emit('action')"
      >
        {{ actionLabel }}
      </button>
    </div>
  </div>
</template>
