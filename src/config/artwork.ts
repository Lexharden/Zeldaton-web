/**
 * Artwork slots. Only /tloz.png ships with the project; everything under /art/ is optional
 * and skipped when the file does not exist (see ArtImage.vue). Swap paths here freely.
 */
export const ART = {
  logo: '/tloz.png',
  /** Event logo (optimized from /logo-zeldaton.png, which stays as the full-size original). */
  zeldatonLogo: '/logo-zeldaton.webp',
  /** Small version for bars and headers (480 px wide). */
  zeldatonLogoSmall: '/logo-zeldaton-sm.webp',
  hiveshockLogo: '/logo-HiveShock.png',
  heroBg: '/art/hero-bg.jpg',
  heroCharacter: '/art/hero-link.png',
  shield: '/art/hyrule-shield.png',
  masterSword: '/art/master-sword.png',
  bgForest: '/art/bg-forest.jpg',
  bgField: '/art/bg-field.jpg',
  bgTemple: '/art/bg-temple.jpg',
  bgCastle: '/art/bg-castle.jpg',
  item: (id: string) => `/art/items/${id}.png`,
} as const
