<script setup lang="ts">
import LanguageSwitch from '@/components/common/LanguageSwitch.vue'
import { t } from '@/i18n'
import { Menu, X } from 'lucide-vue-next'
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { RouterLink, useRoute } from 'vue-router'
import { ART } from '@/config/artwork'
import LiveBadge from '@/components/common/LiveBadge.vue'
import { useEventStore } from '@/stores/event'

/** Sticky nav: compacts on scroll, fullscreen menu on mobile that closes on navigation. */
const event = useEventStore()
const route = useRoute()
const open = ref(false)
const scrolled = ref(false)

const links = [
  { label: 'nav.home', mobile: 'nav.home', to: '/' },
  { label: 'nav.race', mobile: 'nav.liveRace', to: '/race' },
  { label: 'nav.racers', mobile: 'nav.racers', to: { path: '/', hash: '#racers' } },
  { label: 'nav.streams', mobile: 'nav.streams', to: '/streams' },
  { label: 'nav.rules', mobile: 'nav.rules', to: '/rules' },
]

/** Hash links (/#racers) share a path with Home, so compare the hash explicitly. */
function isActive(to: string | { path: string; hash: string }): boolean {
  if (typeof to === 'string') return route.path === to && !(to === '/' && route.hash)
  return route.path === to.path && route.hash === to.hash
}

const onScroll = () => (scrolled.value = window.scrollY > 24)
const onKey = (e: KeyboardEvent) => e.key === 'Escape' && (open.value = false)

onMounted(() => {
  onScroll()
  window.addEventListener('scroll', onScroll, { passive: true })
  window.addEventListener('keydown', onKey)
})
onBeforeUnmount(() => {
  window.removeEventListener('scroll', onScroll)
  window.removeEventListener('keydown', onKey)
  document.body.style.overflow = ''
})
watch(
  () => route.fullPath,
  () => (open.value = false),
)
watch(open, (v) => (document.body.style.overflow = v ? 'hidden' : ''))
</script>

<template>
  <header
    class="fixed inset-x-0 top-0 z-50 border-b transition-all duration-300"
    :class="
      scrolled || open
        ? 'border-line bg-background/85 backdrop-blur-md'
        : 'border-transparent bg-transparent'
    "
  >
    <nav class="container-x flex h-16 items-center gap-6" aria-label="Main">
      <RouterLink to="/" class="flex items-center gap-2.5" :aria-label="t('nav.home')">
        <img
          :src="ART.zeldatonLogoSmall"
          alt="Zeldatón"
          width="480"
          height="149"
          class="h-9 w-auto drop-shadow-[0_0_10px_rgb(245_196_81/0.25)] sm:h-10"
          decoding="async"
          draggable="false"
        />
      </RouterLink>

      <ul class="ml-6 hidden items-center gap-1 lg:flex">
        <li v-for="l in links" :key="l.label">
          <RouterLink
            :to="l.to"
            class="relative px-3.5 py-2 font-display text-[1.05rem] font-semibold tracking-[0.14em] transition-colors hover:text-white"
            :class="
              isActive(l.to)
                ? 'text-white after:absolute after:inset-x-3.5 after:-bottom-0.5 after:h-0.5 after:bg-accent'
                : 'text-muted'
            "
          >
            {{ t(l.label) }}
          </RouterLink>
        </li>
      </ul>

      <div class="ml-auto flex items-center gap-3">
        <RouterLink
          v-if="event.isLive"
          to="/race"
          class="hidden sm:block"
          :aria-label="t('nav.liveNow')"
        >
          <LiveBadge status="live" />
        </RouterLink>
        <LanguageSwitch class="hidden sm:flex" />
        <RouterLink to="/hiveshock" class="btn btn-ghost btn-sm hidden sm:inline-flex">{{
          t('nav.hiveshock')
        }}</RouterLink>
        <button
          type="button"
          class="grid size-10 place-items-center text-white lg:hidden"
          :aria-expanded="open"
          aria-controls="mobile-menu"
          :aria-label="open ? t('nav.closeMenu') : t('nav.openMenu')"
          @click="open = !open"
        >
          <X v-if="open" class="size-6" /><Menu v-else class="size-6" />
        </button>
      </div>
    </nav>

    <!-- In body: the header's backdrop-filter would otherwise become the containing block of this
         fixed panel and collapse it to the header's height. -->
    <Teleport to="body">
      <Transition name="page">
        <div
          v-if="open"
          id="mobile-menu"
          class="fixed inset-x-0 bottom-0 top-16 z-40 overflow-y-auto bg-background/98 lg:hidden"
        >
          <div class="container-x flex min-h-full flex-col py-8">
            <ul class="space-y-1">
              <li
                v-for="(l, i) in links.concat([
                  { label: 'nav.hiveshock', mobile: 'nav.hiveshock', to: '/hiveshock' },
                ])"
                :key="l.label"
                class="enter"
                :style="{ '--d': i }"
              >
                <RouterLink
                  :to="l.to"
                  class="display block border-b border-line py-4 text-5xl text-white active:text-accent"
                  @click="open = false"
                >
                  {{ t(l.mobile) }}
                </RouterLink>
              </li>
            </ul>
            <LanguageSwitch class="mt-8" />
            <div v-if="event.isLive" class="mt-6">
              <LiveBadge status="live" :label="t('nav.liveNow')" />
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>
  </header>
</template>
