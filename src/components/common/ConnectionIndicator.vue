<script setup lang="ts">
import { computed } from 'vue'
import { useWebSocket } from '@/composables/useWebSocket'
import { t } from '@/i18n'

/** Never hides a socket failure: shows connected / reconnecting / offline explicitly. */
const { status, quality } = useWebSocket()

const view = computed(() => {
  switch (status.value) {
    case 'connected':
      return {
        label: t('connection.liveData'),
        text: quality.value === 'poor' ? t('connection.stale') : t('connection.connectedText'),
        tone: 'text-success',
      }
    case 'connecting':
      return {
        label: t('connection.connecting'),
        text: t('connection.connectingText'),
        tone: 'text-secondary',
      }
    case 'reconnecting':
      return {
        label: t('connection.reconnecting'),
        text: t('connection.reconnectingText'),
        tone: 'text-warning',
      }
    default:
      return {
        label: t('connection.offline'),
        text: t('connection.offlineText'),
        tone: 'text-[#ff6b86]',
      }
  }
})
</script>

<template>
  <div class="flex items-center gap-3" role="status" aria-live="polite">
    <span class="relative flex size-2.5">
      <span
        v-if="status === 'connected'"
        class="absolute inline-flex size-full rounded-full bg-success opacity-60 motion-safe:animate-ping"
      />
      <span class="relative inline-flex size-2.5 rounded-full bg-current" :class="view.tone" />
    </span>
    <span class="font-mono text-[11px] font-semibold tracking-[0.16em]" :class="view.tone"
      >● {{ view.label }}</span
    >
    <span class="hidden text-xs text-muted sm:inline">{{ view.text }}</span>
  </div>
</template>
