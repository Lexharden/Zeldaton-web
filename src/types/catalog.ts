/** Which Link an item or objective belongs to. */
export type CatalogAge = 'child' | 'adult' | 'both'

/** The Link a racer is playing right now. */
export type LinkAge = 'child' | 'adult'

export interface CatalogItem {
  id: string
  /** Category the item is filed under (weapon, shield, tunic, boots, arrow, spell, tool, upgrade, song, medallion, stone, key). */
  group: string
  age: CatalogAge
  nameEs: string
  nameEn: string
  /** Two to four characters shown when there is no icon. */
  short: string
  icon?: string
  sortOrder: number
  enabled: boolean
}

export interface CatalogObjective {
  id: string
  age: CatalogAge
  nameEs: string
  nameEn: string
  sortOrder: number
  required: boolean
  enabled: boolean
}

export interface Catalog {
  /** Content hash: clients compare it to know whether their copy is current. */
  version: string
  items: CatalogItem[]
  objectives: CatalogObjective[]
}
