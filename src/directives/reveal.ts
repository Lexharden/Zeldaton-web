import type { Directive } from 'vue'

/**
 * v-reveal: fades an element in once when it enters the viewport.
 * Optional stagger index: v-reveal="2" sets --i for cascaded delays.
 */
let observer: IntersectionObserver | null = null

function getObserver(): IntersectionObserver | null {
  if (typeof IntersectionObserver === 'undefined') return null
  observer ??= new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (entry.isIntersecting) {
          entry.target.classList.add('is-visible')
          observer?.unobserve(entry.target)
        }
      }
    },
    { threshold: 0.12, rootMargin: '0px 0px -6% 0px' },
  )
  return observer
}

export const vReveal: Directive<HTMLElement, number | undefined> = {
  mounted(el, binding) {
    el.classList.add('reveal')
    if (typeof binding.value === 'number') el.style.setProperty('--i', String(binding.value))
    const obs = getObserver()
    if (!obs || window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      el.classList.add('is-visible')
      return
    }
    obs.observe(el)
  },
  unmounted(el) {
    observer?.unobserve(el)
  },
}
