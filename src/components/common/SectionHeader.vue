<script setup lang="ts">
/** Big uppercase section title. `title` accepts several lines. */
withDefaults(
  defineProps<{
    eyebrow?: string
    title: string | string[]
    subtitle?: string
    align?: 'left' | 'center'
    index?: string
  }>(),
  { align: 'left' },
)
</script>

<template>
  <header v-reveal class="mb-10 md:mb-14" :class="align === 'center' ? 'text-center' : ''">
    <p
      v-if="eyebrow || index"
      class="hud-label mb-4 flex items-center gap-3"
      :class="align === 'center' && 'justify-center'"
    >
      <span v-if="index" class="text-accent">{{ index }}</span>
      <span class="h-px w-8 bg-primary/60" aria-hidden="true" />
      <span>{{ eyebrow }}</span>
    </p>
    <h2 class="display text-[clamp(2.6rem,7vw,5.2rem)] text-white">
      <template v-if="Array.isArray(title)">
        <span v-for="(line, i) in title" :key="i" class="block" :class="i > 0 && 'text-gradient'">{{
          line
        }}</span>
      </template>
      <template v-else>{{ title }}</template>
    </h2>
    <p
      v-if="subtitle"
      class="mt-4 max-w-xl text-base text-muted md:text-lg"
      :class="align === 'center' && 'mx-auto'"
    >
      {{ subtitle }}
    </p>
    <slot />
  </header>
</template>
