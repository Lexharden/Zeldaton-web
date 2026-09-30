import { ref } from 'vue'
import type { Router } from 'vue-router'

/**
 * Google Analytics 4 with consent. Only the measurement id is configured (VITE_GA_MEASUREMENT_ID,
 * e.g. G-ABC123XYZ); without it nothing loads and no banner is shown.
 *
 * Privacy first: nothing is requested from Google until the visitor accepts. Consent Mode v2
 * starts with everything denied, the gtag script is injected only after "Accept", and rejecting
 * later removes the _ga cookies. The organizer panel (/admin) is never tracked. A browser that
 * sends Global Privacy Control is treated as a rejection without asking.
 */

const ID_PATTERN = /^G-[A-Z0-9]{4,}$/
const STORAGE_KEY = 'zeldaton-cookie-consent'
/** Bump when the policy changes materially so everyone is asked again. */
const CONSENT_VERSION = 1

export type ConsentChoice = 'granted' | 'denied'

interface StoredConsent {
  analytics: ConsentChoice
  version: number
  at: string
}

type Gtag = (...args: unknown[]) => void
declare global {
  interface Window {
    dataLayer?: unknown[]
    gtag?: Gtag
  }
}

const rawId = (import.meta.env.VITE_GA_MEASUREMENT_ID ?? '').trim().toUpperCase()
/** The configured GA4 id, or '' when analytics is off (missing or malformed id). */
export const measurementId = ID_PATTERN.test(rawId) ? rawId : ''
export const analyticsEnabled = measurementId !== ''

/** The visitor's decision (null = not asked yet). */
export const consent = ref<ConsentChoice | null>(null)
/** Whether the cookie banner is on screen. */
export const bannerOpen = ref(false)

let scriptLoaded = false
let routerRef: Router | null = null

function readStored(): StoredConsent | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (!raw) return null
    const parsed = JSON.parse(raw) as StoredConsent
    if (parsed.version !== CONSENT_VERSION) return null
    return parsed.analytics === 'granted' || parsed.analytics === 'denied' ? parsed : null
  } catch {
    return null
  }
}

function store(choice: ConsentChoice) {
  try {
    const value: StoredConsent = {
      analytics: choice,
      version: CONSENT_VERSION,
      at: new Date().toISOString(),
    }
    localStorage.setItem(STORAGE_KEY, JSON.stringify(value))
  } catch {
    // Private mode / blocked storage: the choice lasts for this visit only.
  }
}

/** The official gtag stub: gtag.js reads the `arguments` object, not an array. */
const gtag: Gtag = function () {
  window.dataLayer = window.dataLayer ?? []
  // eslint-disable-next-line prefer-rest-params
  window.dataLayer.push(arguments)
}

/** Consent Mode v2 defaults: everything denied until the visitor says otherwise. */
function setDefaults() {
  window.gtag = window.gtag ?? gtag
  window.gtag('consent', 'default', {
    ad_storage: 'denied',
    ad_user_data: 'denied',
    ad_personalization: 'denied',
    analytics_storage: 'denied',
    functionality_storage: 'granted',
    security_storage: 'granted',
  })
}

function isPrivatePath(path: string) {
  return path === '/admin' || path.startsWith('/admin/')
}

function pageView(path: string) {
  if (!scriptLoaded || consent.value !== 'granted' || isPrivatePath(path)) return
  // The page title is set by useSeo right after navigation: wait for it.
  setTimeout(() => {
    window.gtag?.('event', 'page_view', {
      page_title: document.title,
      page_location: window.location.origin + path,
      page_path: path,
    })
  }, 50)
}

function loadScript() {
  if (scriptLoaded || !analyticsEnabled) return
  scriptLoaded = true
  window.gtag?.('consent', 'update', { analytics_storage: 'granted' })
  window.gtag?.('js', new Date())
  // Page views are sent by hand on every SPA navigation (see pageView).
  window.gtag?.('config', measurementId, { send_page_view: false })
  const s = document.createElement('script')
  s.async = true
  s.src = `https://www.googletagmanager.com/gtag/js?id=${encodeURIComponent(measurementId)}`
  document.head.appendChild(s)
  const current = routerRef?.currentRoute.value.path
  if (current) pageView(current)
}

/** Removes Google Analytics cookies (_ga, _ga_<id>) on this host and its parent domains. */
function clearGaCookies() {
  const names = document.cookie
    .split(';')
    .map((c) => c.split('=')[0]?.trim() ?? '')
    .filter((n) => n === '_ga' || n.startsWith('_ga_') || n === '_gid')
  const parts = window.location.hostname.split('.')
  const domains = ['', ...parts.map((_, i) => '.' + parts.slice(i).join('.'))]
  for (const name of names) {
    for (const domain of domains) {
      document.cookie = `${name}=; Max-Age=0; path=/${domain ? `; domain=${domain}` : ''}`
    }
  }
}

export function acceptAnalytics() {
  consent.value = 'granted'
  store('granted')
  bannerOpen.value = false
  loadScript()
}

export function rejectAnalytics() {
  const wasGranted = consent.value === 'granted'
  consent.value = 'denied'
  store('denied')
  bannerOpen.value = false
  if (wasGranted) {
    window.gtag?.('consent', 'update', { analytics_storage: 'denied' })
    clearGaCookies()
  }
}

/** "Cookie settings" link in the footer: shows the banner again. */
export function openCookieSettings() {
  if (analyticsEnabled) bannerOpen.value = true
}

/** Call once at startup. Does nothing when no measurement id is configured. */
export function initAnalytics(router: Router) {
  if (!analyticsEnabled || typeof window === 'undefined') return
  routerRef = router
  setDefaults()
  router.afterEach((to) => pageView(to.path))

  const stored = readStored()
  const gpc = (navigator as Navigator & { globalPrivacyControl?: boolean }).globalPrivacyControl
  if (stored) {
    consent.value = stored.analytics
    if (stored.analytics === 'granted') loadScript()
  } else if (gpc) {
    // The browser already says "do not sell / share": respect it without asking.
    consent.value = 'denied'
  } else {
    bannerOpen.value = true
  }
}
