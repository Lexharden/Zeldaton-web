// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest'
import { mount, type VueWrapper } from '@vue/test-utils'
import { nextTick } from 'vue'
import InfoTip from './InfoTip.vue'

let wrapper: VueWrapper | null = null
afterEach(() => {
  wrapper?.unmount()
  wrapper = null
  vi.restoreAllMocks()
})

const mountTip = () => {
  wrapper = mount(InfoTip, { props: { text: 'Explicación útil' }, attachTo: document.body })
  return wrapper
}
/** The popup lives in <body>, outside the component, so no parent can clip or cover it. */
const popup = () => document.body.querySelector<HTMLElement>('[role="tooltip"]')
const open = async (w: VueWrapper) => {
  await w.find('button').trigger('mouseenter')
  await nextTick()
}

/** jsdom has no layout: give the button and the popup a size and place the button. */
function layout(button: Partial<DOMRect>, popupSize = { w: 240, h: 60 }) {
  vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(
    () => ({ width: 16, height: 16, ...button }) as DOMRect,
  )
  vi.spyOn(HTMLElement.prototype, 'offsetWidth', 'get').mockReturnValue(popupSize.w)
  vi.spyOn(HTMLElement.prototype, 'offsetHeight', 'get').mockReturnValue(popupSize.h)
  Object.defineProperty(window, 'innerWidth', { value: 1000, configurable: true })
  Object.defineProperty(window, 'innerHeight', { value: 700, configurable: true })
}

describe('InfoTip', () => {
  it('is closed until the user asks, and is a labelled button', () => {
    const w = mountTip()
    expect(popup()).toBeNull()
    expect(w.find('button').attributes('aria-label')).toBe('Más información')
    expect(w.find('button').attributes('aria-expanded')).toBe('false')
  })

  it('opens on hover, outside the component, and links the button to the text', async () => {
    const w = mountTip()
    await open(w)
    expect(popup()?.textContent).toBe('Explicación útil')
    expect(w.element.contains(popup())).toBe(false)
    expect(popup()?.parentElement).toBe(document.body)
    expect(w.find('button').attributes('aria-describedby')).toBe(popup()?.id)
    await w.find('button').trigger('mouseleave')
    expect(popup()).toBeNull()
  })

  it('sits above everything else', async () => {
    const w = mountTip()
    await open(w)
    expect(popup()?.className).toContain('fixed')
    expect(popup()?.className).toContain('z-[1000]')
  })

  it('opens with the keyboard focus or a tap, and closes with Escape', async () => {
    const w = mountTip()
    await w.find('button').trigger('focus')
    await nextTick()
    expect(popup()).not.toBeNull()
    await w.find('button').trigger('keydown', { key: 'Escape' })
    expect(popup()).toBeNull()
    await w.find('button').trigger('click')
    await nextTick()
    expect(popup()).not.toBeNull()
    await w.find('button').trigger('click')
    expect(popup()).toBeNull()
  })

  it('closes when the user taps somewhere else', async () => {
    const w = mountTip()
    await open(w)
    document.body.dispatchEvent(new Event('pointerdown', { bubbles: true }))
    await nextTick()
    expect(popup()).toBeNull()
  })

  it('opens below the button, centred on it', async () => {
    layout({ left: 400, right: 416, top: 100, bottom: 116 })
    const w = mountTip()
    await open(w)
    expect(popup()?.style.top).toBe('122px') // bottom + 6
    expect(popup()?.style.left).toBe('288px') // centre 408 - 120
  })

  it('stays inside the screen near the edges', async () => {
    layout({ left: 2, right: 18, top: 100, bottom: 116 })
    let w = mountTip()
    await open(w)
    expect(popup()?.style.left).toBe('8px')
    w.unmount()

    layout({ left: 990, right: 1006, top: 100, bottom: 116 })
    w = mountTip()
    await open(w)
    expect(popup()?.style.left).toBe('752px') // 1000 - 240 - 8
  })

  it('goes above the button when there is no room below', async () => {
    layout({ left: 400, right: 416, top: 660, bottom: 676 })
    const w = mountTip()
    await open(w)
    expect(popup()?.style.top).toBe('594px') // top - 6 - height
  })
})
