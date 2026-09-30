import type { Catalog, CatalogAge, CatalogItem, CatalogObjective } from '@/types/catalog'

/** Sections in the order they are shown: each Link first, then what both share. */
export const AGE_ORDER: CatalogAge[] = ['child', 'adult', 'both']

/** Sub-sections inside an age, in reading order. Unknown groups go last, alphabetically. */
export const GROUP_ORDER = [
  'weapon',
  'shield',
  'tunic',
  'boots',
  'tool',
  'arrow',
  'spell',
  'upgrade',
  'key',
  'stone',
  'medallion',
  'song',
]

export interface ItemGroup {
  group: string
  items: CatalogItem[]
}

export interface ItemSection {
  age: CatalogAge
  groups: ItemGroup[]
  items: CatalogItem[]
}

const bySort = <T extends { sortOrder: number; id: string }>(a: T, b: T) =>
  a.sortOrder - b.sortOrder || a.id.localeCompare(b.id)

function groupRank(group: string): number {
  const i = GROUP_ORDER.indexOf(group)
  return i === -1 ? GROUP_ORDER.length : i
}

/** Enabled items grouped by Link (child / adult / both) and then by category. Empty sections are dropped. */
export function groupItems(items: CatalogItem[]): ItemSection[] {
  return AGE_ORDER.flatMap((age) => {
    const inAge = items.filter((i) => i.enabled && i.age === age).sort(bySort)
    if (!inAge.length) return []
    const groups = new Map<string, CatalogItem[]>()
    for (const item of inAge) groups.set(item.group, [...(groups.get(item.group) ?? []), item])
    return [
      {
        age,
        items: inAge,
        groups: [...groups.entries()]
          .map(([group, list]) => ({ group, items: list }))
          .sort(
            (a, b) => groupRank(a.group) - groupRank(b.group) || a.group.localeCompare(b.group),
          ),
      },
    ]
  })
}

export interface ObjectiveSection {
  age: CatalogAge
  objectives: CatalogObjective[]
}

/** Enabled objectives split by Link, keeping the race order inside each. */
export function groupObjectives(objectives: CatalogObjective[]): ObjectiveSection[] {
  return AGE_ORDER.flatMap((age) => {
    const list = objectives.filter((o) => o.enabled && o.age === age).sort(bySort)
    return list.length ? [{ age, objectives: list }] : []
  })
}

/** How many of a section's items the racer owns. */
export function countOwned(
  items: CatalogItem[],
  owned: Record<string, boolean> | undefined,
): { owned: number; total: number } {
  return { owned: items.filter((i) => owned?.[i.id]).length, total: items.length }
}

/** Enabled entries only, in display order (what the site shows). */
export function publicView(catalog: Catalog): Catalog {
  return {
    ...catalog,
    items: catalog.items.filter((i) => i.enabled).sort(bySort),
    objectives: catalog.objectives.filter((o) => o.enabled).sort(bySort),
  }
}
