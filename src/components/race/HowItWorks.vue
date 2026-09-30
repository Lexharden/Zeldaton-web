<script setup lang="ts">
import { t } from '@/i18n'
import { RACE_STEPS } from '@/config/event'
</script>

<template>
  <ol class="grid gap-4 md:grid-cols-2 lg:grid-cols-5">
    <li v-for="(s, i) in RACE_STEPS" :key="s.id" v-reveal="i" class="panel panel-hover">
      <div class="flex h-full flex-col gap-3 p-6">
        <span class="num text-sm font-bold text-accent">0{{ i + 1 }}</span>
        <h3 class="display text-3xl text-white">{{ t(`how.steps.${s.id}.title`) }}</h3>
        <p class="text-sm text-muted">{{ t(`how.steps.${s.id}.text`) }}</p>
        <span
          class="mt-auto h-0.5 w-10 bg-gradient-to-r from-primary to-secondary"
          aria-hidden="true"
        />
      </div>
    </li>
  </ol>
</template>
