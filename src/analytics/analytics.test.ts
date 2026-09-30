// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Router } from 'vue-router'

type Analytics = typeof import('./analytics')

/** Minimal router: records the afterEach hook so tests can "navigate". */
function fakeRouter(path = '/') {
  let hook: ((to: { path: string }) => void) | null = null
  const router = {
    currentRoute: { value: { path } },
    afterEach: (fn: (to: { path: string }) => void) => {
      hook = fn
    },
  }
  return { router: router as unknown as Router, go: (p: string) => hook?.({ path: p }) }
}

async function load(id: string): Promise<Analytics> {
  vi.resetModules()
  vi.stubEnv('VITE_GA_MEASUREMENT_ID', id)
  return import('./analytics')
}

const gaScripts = () =>
  [...document.querySelectorAll('script')].filter((s) => s.src.includes('googletagmanager.com'))

/** Everything pushed to the dataLayer, as plain arrays. */
const layer = () => (window.dataLayer ?? []).map((args) => Array.from(args as ArrayLike<unknown>))

describe('Google Analytics with consent', () => {
  beforeEach(() => {
    localStorage.clear()
    document.head.innerHTML = ''
    window.dataLayer = []
    delete window.gtag
    vi.useFakeTimers()
  })
  afterEach(() => {
    vi.useRealTimers()
    vi.unstubAllEnvs()
  })

  it('does nothing at all without a measurement id', async () => {
    const a = await load('')
    a.initAnalytics(fakeRouter().router)
    expect(a.analyticsEnabled).toBe(false)
    expect(a.bannerOpen.value).toBe(false)
    expect(layer()).toHaveLength(0)
  })

  it('ignores a malformed id', async () => {
    const a = await load('UA-12345-1')
    expect(a.analyticsEnabled).toBe(false)
  })

  it('asks first and loads nothing from Google until the visitor accepts', async () => {
    const a = await load('g-test123')
    const { router, go } = fakeRouter('/race')
    a.initAnalytics(router)

    expect(a.measurementId).toBe('G-TEST123')
    expect(a.bannerOpen.value).toBe(true)
    expect(gaScripts()).toHaveLength(0)
    expect(layer()[0]).toEqual([
      'consent',
      'default',
      expect.objectContaining({ analytics_storage: 'denied', ad_storage: 'denied' }),
    ])

    a.acceptAnalytics()
    expect(a.bannerOpen.value).toBe(false)
    expect(gaScripts()[0]?.src).toContain('id=G-TEST123')
    expect(JSON.parse(localStorage.getItem('zeldaton-cookie-consent')!).analytics).toBe('granted')

    // Page views on navigation, never for the organizer panel.
    go('/streams')
    go('/admin/racers')
    vi.advanceTimersByTime(100)
    const views = layer()
      .filter((e) => e[0] === 'event' && e[1] === 'page_view')
      .map((e) => (e[2] as { page_path: string }).page_path)
    expect(views).toEqual(['/race', '/streams'])
  })

  it('remembers the choice: granted loads at once, denied never asks again', async () => {
    let a = await load('G-TEST123')
    a.initAnalytics(fakeRouter().router)
    a.acceptAnalytics()

    document.head.innerHTML = ''
    a = await load('G-TEST123')
    a.initAnalytics(fakeRouter().router)
    expect(a.bannerOpen.value).toBe(false)
    expect(gaScripts()).toHaveLength(1)

    a.rejectAnalytics()
    document.head.innerHTML = ''
    a = await load('G-TEST123')
    a.initAnalytics(fakeRouter().router)
    expect(a.bannerOpen.value).toBe(false)
    expect(a.consent.value).toBe('denied')
    expect(gaScripts()).toHaveLength(0)
  })

  it('withdrawing consent removes the Analytics cookies', async () => {
    const a = await load('G-TEST123')
    a.initAnalytics(fakeRouter().router)
    a.acceptAnalytics()
    document.cookie = '_ga=GA1.1.123; path=/'
    document.cookie = '_ga_TEST123=GS1.1.1; path=/'
    document.cookie = 'other=keep; path=/'

    a.rejectAnalytics()

    expect(document.cookie).not.toContain('_ga')
    expect(document.cookie).toContain('other=keep')
    expect(
      layer().some(
        (e) =>
          e[0] === 'consent' &&
          e[1] === 'update' &&
          (e[2] as { analytics_storage: string }).analytics_storage === 'denied',
      ),
    ).toBe(true)
  })

  it('treats Global Privacy Control as a rejection without asking', async () => {
    Object.defineProperty(navigator, 'globalPrivacyControl', { value: true, configurable: true })
    const a = await load('G-TEST123')
    a.initAnalytics(fakeRouter().router)
    expect(a.bannerOpen.value).toBe(false)
    expect(a.consent.value).toBe('denied')
    expect(gaScripts()).toHaveLength(0)
    Object.defineProperty(navigator, 'globalPrivacyControl', {
      value: undefined,
      configurable: true,
    })
  })
})
