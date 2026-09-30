import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { DEFAULT_CATALOG } from '@/config/catalog'
import { setLocale } from '@/i18n'
import { useCatalogStore } from './catalog'

describe('catalog store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    setLocale('es')
  })

  it('starts with the factory catalog so the site renders before the API answers', () => {
    const catalog = useCatalogStore()
    expect(catalog.items.length).toBe(DEFAULT_CATALOG.items.length)
    expect(catalog.itemSections.map((s) => s.age)).toEqual(['child', 'adult', 'both'])
    expect(catalog.loaded).toBe(false)
  })

  it('names follow the language and fall back for ids it does not know', () => {
    const catalog = useCatalogStore()
    expect(catalog.itemName('kokiri-sword')).toBe('Espada Kokiri')
    setLocale('en')
    expect(catalog.itemName('kokiri-sword')).toBe('Kokiri Sword')
    expect(catalog.objectiveName('deku-tree')).toBe('Deku Tree')
    expect(catalog.itemName('some-future-item')).toBe('some-future-item')
    setLocale('es')
  })

  it('an organizer edit replaces the catalog and hides disabled entries', () => {
    const catalog = useCatalogStore()
    catalog.set({
      version: 'new',
      items: [
        { ...DEFAULT_CATALOG.items[0], id: 'kokiri-sword', nameEs: 'Espada de Prueba' },
        { ...DEFAULT_CATALOG.items[1], enabled: false },
      ],
      objectives: DEFAULT_CATALOG.objectives,
    })
    expect(catalog.items).toHaveLength(1)
    expect(catalog.itemName('kokiri-sword')).toBe('Espada de Prueba')
    expect(catalog.version).toBe('new')
    expect(catalog.loaded).toBe(true)
  })

  it('CATALOG_UPDATED reloads only when the version differs', async () => {
    const catalog = useCatalogStore()
    catalog.set({ ...DEFAULT_CATALOG, version: 'same', items: [DEFAULT_CATALOG.items[0]] })
    await catalog.onUpdated('same')
    expect(catalog.items).toHaveLength(1) // nothing to do

    await catalog.onUpdated('different') // mock API answers with the factory catalog
    expect(catalog.items.length).toBe(DEFAULT_CATALOG.items.length)
    expect(catalog.version).toBe(DEFAULT_CATALOG.version)
  })
})
