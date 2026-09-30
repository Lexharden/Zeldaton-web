<script setup lang="ts">
import { computed } from 'vue'
import { t } from '@/i18n'
import { ArrowDown, ArrowRight, Cpu, Gamepad2, Globe, MessageSquare } from 'lucide-vue-next'

/** CHAT -> HIVESHOCK -> GAME -> WEB, with a concrete !bomb example. */
const steps = computed(() => [
  { icon: MessageSquare, title: t('hiveshock.flowChat'), line: '!bomb', tone: 'text-secondary' },
  { icon: Cpu, title: t('hiveshock.flowHive'), line: 'TRIGGER_GAME_EVENT', tone: 'text-primary' },
  {
    icon: Gamepad2,
    title: t('hiveshock.flowGame'),
    line: t('hiveshock.flowBomb'),
    tone: 'text-accent',
  },
  {
    icon: Globe,
    title: t('hiveshock.flowWeb'),
    line: t('hiveshock.flowLive'),
    tone: 'text-magenta',
  },
])
</script>

<template>
  <ol
    class="flex flex-col items-stretch gap-2 lg:flex-row lg:items-center lg:gap-3"
    :aria-label="t('hiveshock.flowAria')"
  >
    <template v-for="(s, i) in steps" :key="s.title">
      <li v-reveal="i" class="panel panel-sm flex-1">
        <div class="p-5 text-center">
          <component :is="s.icon" class="mx-auto mb-3 size-7" :class="s.tone" aria-hidden="true" />
          <p class="hud-label">{{ s.title }}</p>
          <p class="mt-2 font-mono text-sm font-semibold text-white">{{ s.line }}</p>
        </div>
      </li>
      <li
        v-if="i < steps.length - 1"
        class="grid place-items-center text-primary"
        aria-hidden="true"
      >
        <ArrowDown class="size-5 lg:hidden" /><ArrowRight class="hidden size-5 lg:block" />
      </li>
    </template>
  </ol>
</template>
