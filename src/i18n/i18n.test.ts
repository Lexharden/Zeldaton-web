import { describe, expect, it } from 'vitest'
import { en } from './en'
import { es } from './es'
import { setLocale, t, tx } from './index'

/** Collects every dotted key so we can assert both catalogs have the same shape. */
function keys(node: unknown, prefix = ''): string[] {
  if (Array.isArray(node)) return [prefix]
  if (node && typeof node === 'object') {
    return Object.entries(node).flatMap(([k, v]) => keys(v, prefix ? `${prefix}.${k}` : k))
  }
  return [prefix]
}

describe('i18n', () => {
  it('has identical keys in Spanish and English', () => {
    expect(keys(en).sort()).toEqual(keys(es).sort())
  })

  it('keeps FAQ lengths aligned', () => {
    expect(en.rulesPage.faq.length).toBe(es.rulesPage.faq.length)
  })

  it('defaults to Spanish and interpolates', () => {
    setLocale('es')
    expect(t('hero.hoursDay', { n: 4 })).toBe('4 HORAS AL DÍA.')
    setLocale('en')
    expect(t('hero.hoursDay', { n: 4 })).toBe('4 HOURS A DAY.')
    setLocale('es')
  })

  it('translates catalog values and passes unknown ones through', () => {
    setLocale('es')
    expect(tx('objectives', 'water-temple')).toBe('Templo del Agua')
    expect(tx('areas', 'Some New Area')).toBe('Some New Area')
    expect(t('does.not.exist')).toBe('does.not.exist')
  })

  it('never uses the forbidden phrases for HiveShock', () => {
    const all = JSON.stringify([en, es]).toLowerCase()
    expect(all).not.toContain('open platform')
    expect(all).not.toContain('open-source software')
  })
})
