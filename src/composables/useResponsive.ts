import { onBeforeUnmount, onMounted, ref } from 'vue'

const QUERIES = {
  md: '(min-width: 768px)',
  lg: '(min-width: 1024px)',
  reduceMotion: '(prefers-reduced-motion: reduce)',
}

/** Reactive media queries. Breakpoints mirror Tailwind's md / lg. */
export function useResponsive() {
  const isMd = ref(false)
  const isLg = ref(false)
  const reducedMotion = ref(false)
  const lists: [MediaQueryList, () => void][] = []

  onMounted(() => {
    const bind = (query: string, target: typeof isMd) => {
      const mql = window.matchMedia(query)
      const update = () => (target.value = mql.matches)
      update()
      mql.addEventListener('change', update)
      lists.push([mql, update])
    }
    bind(QUERIES.md, isMd)
    bind(QUERIES.lg, isLg)
    bind(QUERIES.reduceMotion, reducedMotion)
  })
  onBeforeUnmount(() => lists.forEach(([mql, fn]) => mql.removeEventListener('change', fn)))

  return { isMd, isLg, reducedMotion }
}
