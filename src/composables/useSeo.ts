import { watchEffect } from 'vue'
import { t } from '@/i18n'

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

/** Per-route SEO: title, description and the social preview image. */
export function useSeo(title?: () => string | undefined, description?: () => string | undefined) {
  watchEffect(() => {
    const pageTitle = title?.()
    const fullTitle = pageTitle ? `${pageTitle} — Zeldatón` : t('meta.title')
    const desc = description?.() ?? t('meta.description')
    document.title = fullTitle
    setMeta('meta[name="description"]', 'content', desc)
    setMeta('meta[property="og:title"]', 'content', fullTitle)
    setMeta('meta[property="og:description"]', 'content', desc)
    setMeta('meta[name="twitter:title"]', 'content', fullTitle)
    // PNG: social networks do not render SVG previews.
    const image = '/og-zeldaton.png'
    setMeta('meta[property="og:image"]', 'content', image)
    setMeta('meta[name="twitter:image"]', 'content', image)
    ensureCanonical().href = window.location.origin + window.location.pathname
  })
}
