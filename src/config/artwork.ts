/**
 * Artwork slots. Only /tloz.png ships with the project; everything under /art/ is optional
 * and skipped when the file does not exist (see ArtImage.vue). Swap paths here freely.
 */
export const ART = {
  logo: '/tloz.png',
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
