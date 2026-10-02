import { describe, expect, it } from 'vitest'
import { itemIconUrl, validIconName } from './media'

describe('item pictures', () => {
  it('accepts plain picture file names, including the ones already in the project', () => {
    for (const ok of [
      'Hookshot-Art.png',
      "Goron's_Ruby_-_NP_OOT_Player's_Guide.png",
      'Light_Arrow_-_OOT64_render.PNG',
      'a b (1).webp',
    ])
      expect(validIconName(ok), ok).toBe(true)
  })

  it('rejects folders, hidden files, scripts and other formats', () => {
    for (const bad of [
      '',
      '.x.png',
      '../x.png',
      'a/b.png',
      'x.svg',
      'x.png.exe',
      'noext',
      '<b>.png',
    ])
      expect(validIconName(bad), bad).toBe(false)
  })

  it('points a bare file name at the media route, encoded; older paths pass through', () => {
    // Tests run with VITE_MOCK_MODE unset => mock mode => the files shipped in public/art/items.
    expect(itemIconUrl("Goron's Ruby.png")).toBe("/art/items/Goron's%20Ruby.png")
    expect(itemIconUrl('/art/items/x.png')).toBe('/art/items/x.png')
    expect(itemIconUrl('https://example.com/x.png')).toBe('https://example.com/x.png')
  })
})
