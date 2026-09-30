import { computed, ref } from 'vue'
import { en } from './en'
import { es, type Messages } from './es'

export type Locale = 'es' | 'en'
const STORAGE_KEY = 'zeldathon-locale'
const catalogs: Record<Locale, Messages> = { es, en }

function initialLocale(): Locale {
  try {
    const saved = localStorage.getItem(STORAGE_KEY)
    if (saved === 'es' || saved === 'en') return saved
  } catch {
    /* storage unavailable */
  }
  return 'es' // Spanish (Latin America) is the default audience.
}

/** Shared, reactive locale. Spanish by default, English optional and remembered. */
export const locale = ref<Locale>(initialLocale())

export function setLocale(next: Locale) {
  locale.value = next
  try {
    localStorage.setItem(STORAGE_KEY, next)
  } catch {
    /* ignore */
  }
  if (typeof document !== 'undefined')
    document.documentElement.lang = next === 'es' ? 'es-MX' : 'en'
}

function lookup(catalog: Messages, key: string): unknown {
  return key.split('.').reduce<unknown>((node, part) => {
    if (node && typeof node === 'object') return (node as Record<string, unknown>)[part]
    return undefined
  }, catalog)
}

function interpolate(text: string, params?: Record<string, string | number>): string {
  if (!params) return text
  return text.replace(/\{(\w+)\}/g, (m, k: string) => (k in params ? String(params[k]) : m))
}

/** Translate a dotted key. Falls back to Spanish, then to the key itself. */
export function t(key: string, params?: Record<string, string | number>): string {
  const value = lookup(catalogs[locale.value], key) ?? lookup(catalogs.es, key)
  return typeof value === 'string' ? interpolate(value, params) : key
}

/** Translate a catalog value (area, boss, objective id...); unknown values pass through as-is. */
export function tx(namespace: string, value: string | undefined): string {
  if (!value) return ''
  const key = `${namespace}.${value}`
  const out = t(key)
  return out === key ? value : out
}

/** Raw (non-string) message such as an array of FAQ entries. */
export function tm<T>(key: string): T {
  return (lookup(catalogs[locale.value], key) ?? lookup(catalogs.es, key)) as T
}

/** Intl locale tag for date/number formatting. */
export const intlLocale = computed(() => (locale.value === 'es' ? 'es-MX' : 'en-GB'))

export function useI18n() {
  return { t, tm, tx, locale, setLocale, intlLocale }
}

if (typeof document !== 'undefined')
  document.documentElement.lang = locale.value === 'es' ? 'es-MX' : 'en'
