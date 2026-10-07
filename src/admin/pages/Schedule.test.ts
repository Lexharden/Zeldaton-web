// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createMemoryHistory, createRouter } from 'vue-router'
import { adminApi } from '../api/AdminApi'
import { useToasts } from '../composables/useToasts'
import { useAdminStore } from '../stores/admin'
import type { Overview, ScheduleSlot } from '../types'
import Schedule from './Schedule.vue'

const hours = (h: number) => new Date(Date.now() + h * 3600_000).toISOString()
const slot = (id: number, racerId: string, from: number, to: number): ScheduleSlot => ({
  id,
  racerId,
  racerName: racerId.toUpperCase(),
  racerTimezone: 'America/Mexico_City',
  startUtc: hours(from),
  endUtc: hours(to),
  note: null,
  assignees: [],
})

const overview = {
  serverTimeUtc: new Date().toISOString(),
  event: { id: 'z', name: 'Zeldatón', edition: '2026', status: 'live' },
  summary: {},
  racers: [
    {
      racer: {
        id: 'ralbat',
        displayName: 'RALBAT',
        timezone: 'America/Mexico_City',
        status: 'live',
      },
      connected: true,
      lastHeartbeatUtc: 'x',
      heartbeatAgeSeconds: 1,
    },
  ],
  alerts: [],
  hiveshock: {},
  activity: [],
  winner: null,
  catalogVersion: 'v',
} as unknown as Overview

let mounted: VueWrapper[] = []

async function mountPage() {
  const pinia = createPinia()
  setActivePinia(pinia)
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [{ path: '/:p(.*)*', name: 'any', component: { template: '<div />' } }],
  })
  await router.push('/admin/schedule')
  await router.isReady()
  const wrapper = mount(Schedule, { global: { plugins: [pinia, router] }, attachTo: document.body })
  mounted.push(wrapper)
  await flushPromises()
  return wrapper
}

beforeEach(() => {
  vi.restoreAllMocks()
  useToasts().toasts.value = []
  vi.spyOn(adminApi, 'overview').mockResolvedValue(overview)
})
afterEach(() => {
  mounted.forEach((w) => w.unmount())
  mounted = []
  document.body.innerHTML = ''
})

describe('Schedule', () => {
  it('lists the slots with their state and lets a referee take one', async () => {
    const take = vi.spyOn(adminApi, 'takeSlot').mockResolvedValue({ ok: true })
    vi.spyOn(adminApi, 'schedule').mockResolvedValue({
      serverTimeUtc: new Date().toISOString(),
      slots: [slot(1, 'ralbat', -1, 2), slot(2, 'cuaco', 3, 6)],
    })
    const wrapper = await mountPage()
    const live = wrapper.find('[data-slot="1"]')
    expect(live.text()).toContain('RALBAT')
    expect(live.text()).toContain('En vivo') // connected inside its hours
    expect(wrapper.find('[data-slot="2"]').text()).toContain('Próximo')
    expect(wrapper.find('[data-slot="2"]').text()).toContain('Sin árbitro')
    await wrapper
      .find('[data-slot="2"]')
      .findAll('button')
      .find((b) => b.text() === 'Tomar')!
      .trigger('click')
    expect(take).toHaveBeenCalledWith(2)
  })

  it('offers the form and the remove button only to admins', async () => {
    vi.spyOn(adminApi, 'schedule').mockResolvedValue({
      serverTimeUtc: new Date().toISOString(),
      slots: [slot(2, 'cuaco', 3, 6)],
    })
    const wrapper = await mountPage()
    expect(wrapper.text()).not.toContain('Programar')
    expect(wrapper.find('button[aria-label^="Quitar"]').exists()).toBe(false)

    useAdminStore().user = { id: 1, username: 'ana', role: 'admin' }
    await flushPromises()
    expect(wrapper.text()).toContain('Programar')
    expect(wrapper.find('button[aria-label^="Quitar"]').exists()).toBe(true)
  })

  it('creates a slot from the form', async () => {
    vi.spyOn(adminApi, 'schedule').mockResolvedValue({
      serverTimeUtc: new Date().toISOString(),
      slots: [],
    })
    const create = vi.spyOn(adminApi, 'createSlot').mockResolvedValue({ id: 9 })
    const wrapper = await mountPage()
    useAdminStore().user = { id: 1, username: 'ana', role: 'admin' }
    await flushPromises()
    await wrapper.find('select').setValue('ralbat')
    const inputs = wrapper.findAll('input[type="datetime-local"]')
    await inputs[0]!.setValue('2026-10-07T18:00')
    await inputs[1]!.setValue('2026-10-07T21:00')
    await wrapper.find('form').trigger('submit')
    await flushPromises()
    expect(create).toHaveBeenCalledOnce()
    const body = create.mock.calls[0]![0]
    expect(body.racerId).toBe('ralbat')
    expect(new Date(body.endUtc).getTime() - new Date(body.startUtc).getTime()).toBe(3 * 3600_000)
  })
})
