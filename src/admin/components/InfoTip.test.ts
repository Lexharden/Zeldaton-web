// @vitest-environment jsdom
import { afterEach, describe, expect, it } from 'vitest'
import { mount, type VueWrapper } from '@vue/test-utils'
import InfoTip from './InfoTip.vue'

let wrapper: VueWrapper | null = null
afterEach(() => wrapper?.unmount())

const mountTip = () => {
  wrapper = mount(InfoTip, { props: { text: 'Explicación útil' }, attachTo: document.body })
  return wrapper
}

describe('InfoTip', () => {
  it('is closed until the user asks, and is a labelled button', () => {
    const w = mountTip()
    expect(w.find('[role="tooltip"]').exists()).toBe(false)
    expect(w.find('button').attributes('aria-label')).toBe('Más información')
    expect(w.find('button').attributes('aria-expanded')).toBe('false')
  })

  it('opens on hover and links the button to the text', async () => {
    const w = mountTip()
    await w.trigger('mouseenter')
    const tip = w.find('[role="tooltip"]')
    expect(tip.text()).toBe('Explicación útil')
    expect(w.find('button').attributes('aria-describedby')).toBe(tip.attributes('id'))
    await w.trigger('mouseleave')
    expect(w.find('[role="tooltip"]').exists()).toBe(false)
  })

  it('opens with the keyboard focus or a tap, and closes with Escape', async () => {
    const w = mountTip()
    await w.find('button').trigger('focus')
    expect(w.find('[role="tooltip"]').exists()).toBe(true)
    await w.find('button').trigger('keydown', { key: 'Escape' })
    expect(w.find('[role="tooltip"]').exists()).toBe(false)
    await w.find('button').trigger('click')
    expect(w.find('[role="tooltip"]').exists()).toBe(true)
    await w.find('button').trigger('click')
    expect(w.find('[role="tooltip"]').exists()).toBe(false)
  })
})
