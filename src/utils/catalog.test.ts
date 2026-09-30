import { describe, expect, it } from 'vitest'
import { DEFAULT_CATALOG } from '@/config/catalog'
import type { CatalogItem } from '@/types/catalog'
import { AGE_ORDER, countOwned, groupItems, groupObjectives, publicView } from './catalog'

const item = (over: Partial<CatalogItem> & { id: string }): CatalogItem => ({
  group: 'tool',
  age: 'both',
  nameEs: over.id,
  nameEn: over.id,
  short: 'X',
  sortOrder: 0,
  enabled: true,
  ...over,
})

describe('the factory catalog', () => {
  it('has every Link and keeps the original nine ids', () => {
    const ids = new Set(DEFAULT_CATALOG.items.map((i) => i.id))
    for (const original of [
      'master-sword',
      'hookshot',
      'longshot',
      'bow',
      'bombs',
      'boomerang',
      'megaton-hammer',
      'iron-boots',
      'mirror-shield',
    ])
      expect(ids.has(original), original).toBe(true)
    expect(DEFAULT_CATALOG.items.length).toBeGreaterThanOrEqual(60)
    for (const age of AGE_ORDER) expect(DEFAULT_CATALOG.items.some((i) => i.age === age)).toBe(true)
    expect(DEFAULT_CATALOG.objectives).toHaveLength(10)
  })
})

describe('groupItems', () => {
  it('splits by Link in the order child, adult, both and by category inside', () => {
    const sections = groupItems([
      item({ id: 'a-song', age: 'both', group: 'song', sortOrder: 5 }),
      item({ id: 'a-boots', age: 'adult', group: 'boots', sortOrder: 2 }),
      item({ id: 'a-sword', age: 'adult', group: 'weapon', sortOrder: 1 }),
      item({ id: 'k-sword', age: 'child', group: 'weapon', sortOrder: 1 }),
    ])
    expect(sections.map((s) => s.age)).toEqual(['child', 'adult', 'both'])
    expect(sections[1].groups.map((g) => g.group)).toEqual(['weapon', 'boots'])
  })

  it('drops disabled items and empty sections, and puts unknown categories last', () => {
    const sections = groupItems([
      item({ id: 'hidden', age: 'child', enabled: false }),
      item({ id: 'odd', age: 'adult', group: 'zzz-new' }),
      item({ id: 'sword', age: 'adult', group: 'weapon' }),
    ])
    expect(sections.map((s) => s.age)).toEqual(['adult'])
    expect(sections[0].groups.map((g) => g.group)).toEqual(['weapon', 'zzz-new'])
  })

  it('orders by sortOrder then id', () => {
    const [s] = groupItems([
      item({ id: 'b', sortOrder: 1 }),
      item({ id: 'a', sortOrder: 1 }),
      item({ id: 'c', sortOrder: 0 }),
    ])
    expect(s.items.map((i) => i.id)).toEqual(['c', 'a', 'b'])
  })

  it('groups the real catalog without losing an enabled item', () => {
    const sections = groupItems(DEFAULT_CATALOG.items)
    expect(sections.flatMap((s) => s.items)).toHaveLength(DEFAULT_CATALOG.items.length)
  })
})

describe('groupObjectives / countOwned / publicView', () => {
  it('splits objectives between child dungeons and adult temples', () => {
    const sections = groupObjectives(DEFAULT_CATALOG.objectives)
    expect(sections.map((s) => s.age)).toEqual(['child', 'adult'])
    expect(sections[0].objectives.map((o) => o.id)).toEqual([
      'kokiri-forest',
      'deku-tree',
      'dodongos-cavern',
      'jabu-jabu',
    ])
    expect(sections[1].objectives.at(-1)?.id).toBe('ganons-castle')
  })

  it('counts what a racer owns, ignoring missing telemetry', () => {
    const items = [item({ id: 'a' }), item({ id: 'b' }), item({ id: 'c' })]
    expect(countOwned(items, { a: true, b: false })).toEqual({ owned: 1, total: 3 })
    expect(countOwned(items, undefined)).toEqual({ owned: 0, total: 3 })
  })

  it('keeps only enabled entries, sorted', () => {
    const view = publicView({
      version: 'v',
      items: [
        item({ id: 'z', sortOrder: 2 }),
        item({ id: 'off', enabled: false }),
        item({ id: 'a', sortOrder: 1 }),
      ],
      objectives: [],
    })
    expect(view.items.map((i) => i.id)).toEqual(['a', 'z'])
  })
})
