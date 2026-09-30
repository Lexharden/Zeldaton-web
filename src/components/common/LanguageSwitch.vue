<script setup lang="ts">
import { locale, setLocale, t, type Locale } from '@/i18n'

/** ES / EN toggle. Spanish is the default; the choice is remembered. */
const options: { id: Locale; label: string }[] = [
  { id: 'es', label: 'ES' },
  { id: 'en', label: 'EN' },
]
</script>

<template>
  <div
    class="flex items-center gap-0.5 font-mono text-[11px] font-semibold tracking-widest"
    role="group"
    :aria-label="t('nav.language')"
  >
    <button
      v-for="o in options"
      :key="o.id"
      type="button"
      class="px-2 py-1.5 transition-colors"
      :class="
        locale === o.id
          ? 'bg-accent/15 text-accent ring-1 ring-accent/40'
          : 'text-muted hover:text-white'
      "
      :aria-pressed="locale === o.id"
      :lang="o.id"
      @click="setLocale(o.id)"
    >
      {{ o.label }}
    </button>
  </div>
</template>
