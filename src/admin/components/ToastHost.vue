<script setup lang="ts">
import { CheckCircle2, Info, XCircle } from 'lucide-vue-next'
import { useToasts } from '../composables/useToasts'

const { toasts, dismiss } = useToasts()
const styles = {
  success: 'border-success/50 text-success',
  error: 'border-danger/60 text-[#ff8aa0]',
  info: 'border-secondary/50 text-secondary',
} as const
</script>

<template>
  <Teleport to="body">
    <div
      class="pointer-events-none fixed bottom-4 right-4 z-[90] flex w-[min(24rem,calc(100vw-2rem))] flex-col gap-2"
      role="status"
      aria-live="polite"
    >
      <button
        v-for="t in toasts"
        :key="t.id"
        type="button"
        class="pointer-events-auto flex items-start gap-3 rounded border bg-[#0b1419] p-3 text-left text-sm shadow-lg"
        :class="styles[t.kind]"
        @click="dismiss(t.id)"
      >
        <CheckCircle2 v-if="t.kind === 'success'" class="mt-0.5 size-4 shrink-0" />
        <XCircle v-else-if="t.kind === 'error'" class="mt-0.5 size-4 shrink-0" />
        <Info v-else class="mt-0.5 size-4 shrink-0" />
        <span class="text-white">{{ t.message }}</span>
      </button>
    </div>
  </Teleport>
</template>
