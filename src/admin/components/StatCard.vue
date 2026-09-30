<script setup lang="ts">
defineProps<{
  label: string
  value: string | number
  tone?: 'default' | 'good' | 'warn' | 'bad'
  hint?: string
}>()
</script>

<template>
  <div class="panel panel-sm">
    <div class="p-4">
      <p class="hud-label text-[10px]">{{ label }}</p>
      <p
        class="num mt-1 text-3xl font-bold"
        :class="{
          'text-white': !tone || tone === 'default',
          'text-success': tone === 'good',
          'text-warning': tone === 'warn',
          'text-[#ff8aa0]': tone === 'bad',
        }"
      >
        {{ value }}
      </p>
      <p v-if="hint" class="mt-0.5 text-xs text-muted">{{ hint }}</p>
    </div>
  </div>
</template>
