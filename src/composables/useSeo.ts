import { watchEffect } from 'vue'
import { t } from '@/i18n'
import { useEventStore } from '@/stores/event'

function ensureCanonical(): HTMLLinkElement {
  let link = document.head.querySelector<HTMLLinkElement>('link[rel="canonical"]')
  if (!link) {
    link = document.createElement('link')
    link.rel = 'canonical'
    document.head.appendChild(link)
  }
  return link
}

function setMeta(selector: string, attr: string, value: string) {
  document.head.querySelector(selector)?.setAttribute(attr, value)
}

/** Per-route SEO. The OG image can change when the event is live. */
export function useSeo(title?: () => string | undefined, description?: () => string | undefined) {
  const event = useEventStore()
  watchEffect(() => {
    const pageTitle = title?.()
    const fullTitle = pageTitle ? `${pageTitle} — Zeldathon` : t('meta.title')
    const desc = description?.() ?? t('meta.description')
    document.title = fullTitle
    setMeta('meta[name="description"]', 'content', desc)
    setMeta('meta[property="og:title"]', 'content', fullTitle)
    setMeta('meta[property="og:description"]', 'content', desc)
    setMeta('meta[name="twitter:title"]', 'content', fullTitle)
    const image = event.status === 'live' ? '/og-image-live.svg' : '/og-image.svg'
    setMeta('meta[property="og:image"]', 'content', image)
    setMeta('meta[name="twitter:image"]', 'content', image)
    ensureCanonical().href = window.location.origin + window.location.pathname
  })
}
