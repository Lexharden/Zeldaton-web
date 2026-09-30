<script setup lang="ts">
import ArtImage from './ArtImage.vue'
/**
 * Layered decorative background: gradients, glow, grid, stars, noise and geometric shapes.
 * Pure CSS/SVG (no canvas). Toggle layers via props; place inside a `relative` section.
 */
withDefaults(
  defineProps<{
    glow?: boolean
    grid?: boolean
    particles?: boolean
    shapes?: boolean
    noise?: boolean
    variant?: 'hero' | 'section'
    art?: string
  }>(),
  { glow: true, grid: true, particles: false, shapes: false, noise: true, variant: 'section' },
)
const stars = Array.from({ length: 34 }, (_, i) => ({
  left: `${(i * 37 + (i % 5) * 11) % 100}%`,
  top: `${(i * 53 + (i % 7) * 9) % 100}%`,
  size: 1 + (i % 3),
  delay: `${(i % 9) * 0.6}s`,
  dur: `${3 + (i % 5)}s`,
}))
</script>

<template>
  <div
    class="pointer-events-none absolute inset-0 -z-10 overflow-hidden"
    :class="noise && 'grain'"
    aria-hidden="true"
  >
    <ArtImage
      v-if="art"
      :src="art"
      class="absolute inset-0 size-full object-cover opacity-30"
      :style="{
        maskImage: 'linear-gradient(to bottom, transparent, #000 25%, #000 60%, transparent)',
        WebkitMaskImage: 'linear-gradient(to bottom, transparent, #000 25%, #000 60%, transparent)',
      }"
    />
    <div
      v-if="glow"
      class="absolute inset-0"
      :style="{
        background:
          variant === 'hero'
            ? 'radial-gradient(60% 55% at 78% 18%, rgb(47 125 58 / 0.5), transparent 70%), radial-gradient(45% 40% at 8% 85%, rgb(74 144 217 / 0.16), transparent 70%), radial-gradient(40% 30% at 50% 0%, rgb(214 74 58 / 0.14), transparent 70%)'
            : 'radial-gradient(50% 50% at 85% 10%, rgb(47 125 58 / 0.22), transparent 70%), radial-gradient(40% 40% at 5% 90%, rgb(74 144 217 / 0.08), transparent 70%)',
      }"
    />
    <div
      v-if="grid"
      class="absolute inset-0 opacity-[0.16]"
      :style="{
        backgroundImage:
          'linear-gradient(rgb(76 154 82 / 0.5) 1px, transparent 1px), linear-gradient(90deg, rgb(76 154 82 / 0.5) 1px, transparent 1px)',
        backgroundSize: '56px 56px',
        maskImage: 'radial-gradient(70% 70% at 60% 30%, #000, transparent)',
        WebkitMaskImage: 'radial-gradient(70% 70% at 60% 30%, #000, transparent)',
      }"
    />
    <template v-if="shapes">
      <div
        class="absolute -right-24 top-0 h-full w-[46%] skew-x-[-14deg] bg-gradient-to-b from-primary/16 via-primary/5 to-transparent"
      />
      <div
        class="absolute right-[22%] top-0 h-full w-px skew-x-[-14deg] bg-gradient-to-b from-secondary/50 to-transparent"
      />
      <div
        class="absolute bottom-0 left-[-6%] h-40 w-[60%] skew-x-[-14deg] bg-gradient-to-r from-accent/8 to-transparent"
      />
    </template>
    <template v-if="particles">
      <span
        v-for="(s, i) in stars"
        :key="i"
        class="absolute rounded-full bg-white motion-safe:animate-[twinkle_var(--dur)_ease-in-out_infinite]"
        :style="{
          left: s.left,
          top: s.top,
          width: `${s.size}px`,
          height: `${s.size}px`,
          animationDelay: s.delay,
          '--dur': s.dur,
        }"
      />
    </template>
    <div class="absolute inset-x-0 bottom-0 h-40 bg-gradient-to-t from-background to-transparent" />
  </div>
</template>
