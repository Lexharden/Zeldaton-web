// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createMemoryHistory, createRouter } from 'vue-router'
import type { Racer } from '@/types/racer'
import { adminApi, AdminApiError } from '../api/AdminApi'
import { useToasts } from '../composables/useToasts'
import { useAdminStore } from '../stores/admin'
import type { Incident, MonitorData, Overview } from '../types'
import Monitor from './Monitor.vue'

const racer = (id: string, over: Partial<Racer> = {}): Racer => ({
  id,
  displayName: id.toUpperCase(),
  slug: id,
  timezone: 'America/Mexico_City',
  status: 'live',
  elapsedSeconds: 600,
  remainingSeconds: 13800,
  progressPercentage: 40,
  completedObjectives: [],
  channels: [],
  ...over,
})

const overview = {
  serverTimeUtc: '2026-10-07T13:00:00.000Z',
  event: { id: 'z', name: 'Zeldatón', edition: '2026', status: 'live' },
  summary: {
    racers: 2,
    connected: 1,
    live: 1,
    paused: 0,
    online: 0,
    offline: 1,
    exhausted: 0,
    finished: 0,
  },
  racers: [
    { racer: racer('ralbat'), connected: true, lastHeartbeatUtc: 'x', heartbeatAgeSeconds: 3 },
    {
      racer: racer('cuaco', { status: 'online' }),
      connected: false,
      lastHeartbeatUtc: null,
      heartbeatAgeSeconds: null,
    },
  ],
  alerts: [],
  hiveshock: {},
  activity: [],
  winner: null,
  catalogVersion: 'v',
} as unknown as Overview

const incident = (over: Partial<Incident> = {}): Incident => ({
  id: 7,
  ts: '2026-10-07T12:50:00.000Z',
  kind: 'suspicious',
  severity: 'error',
  racerId: 'cuaco',
  racerName: 'CUACO',
  message: 'CUACO pasó de 10% a 70% en 00:01:00 (salto brusco).',
  payload: {},
  status: 'open',
  reviewedBy: null,
  reviewedAt: null,
  note: null,
  endedAt: null,
  ...over,
})

const monitorData = (over: Partial<MonitorData> = {}): MonitorData => ({
  serverTimeUtc: '2026-10-07T13:00:00.000Z',
  openCount: 1,
  open: [incident()],
  recent: [],
  lastSeenUtc: '2026-10-07T09:00:00.000Z',
  catchUp: {
    since: '2026-10-07T09:00:00.000Z',
    newIncidents: 2,
    byKind: { suspicious: 1, disconnected: 1 },
    actions: [
      {
        ts: '2026-10-07T10:00:00.000Z',
        actor: 'ana',
        action: 'racer.adjust-time',
        racerId: 'ralbat',
        payload: { reason: 'lag del stream' },
      },
    ],
    donations: 5,
    donationsLimited: 1,
  },
  notes: [],
  ...over,
})

let mounted: VueWrapper[] = []

async function mountPage() {
  const pinia = createPinia()
  setActivePinia(pinia)
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [{ path: '/:p(.*)*', name: 'any', component: { template: '<div />' } }],
  })
  await router.push('/admin/monitor')
  await router.isReady()
  const wrapper = mount(Monitor, { global: { plugins: [pinia, router] }, attachTo: document.body })
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

describe('Monitor', () => {
  it('shows who is playing, flags a missing signal and lists the open incidents', async () => {
    vi.spyOn(adminApi, 'monitor').mockResolvedValue(monitorData())
    const wrapper = await mountPage()
    const text = wrapper.text()
    expect(text).toContain('RALBAT')
    expect(text).toContain('CUACO')
    // The event is live, cuaco is waiting but HiveShock is not connected.
    expect(text).toContain('sin señal de HiveShock')
    expect(wrapper.find('[data-racer="cuaco"]').text()).toContain('1 ⚠')
    expect(wrapper.find('[data-test="incidents"]').text()).toContain('salto brusco')
  })

  it('summarises what happened since the last visit and marks it as read', async () => {
    const seen = vi.spyOn(adminApi, 'monitorSeen').mockResolvedValue({ ok: true })
    vi.spyOn(adminApi, 'monitor').mockResolvedValue(monitorData())
    const wrapper = await mountPage()
    const banner = wrapper.find('[data-test="catch-up"]')
    expect(banner.text()).toContain('2 incidentes nuevos')
    expect(banner.text()).toContain('1 de 5 donaciones')
    expect(banner.text()).toContain('ana: Ajustó el tiempo de un corredor')
    expect(banner.text()).toContain('lag del stream')
    await banner
      .findAll('button')
      .find((b) => b.text() === 'Marcar como leído')!
      .trigger('click')
    expect(seen).toHaveBeenCalledOnce()
  })

  it('hides the summary when nothing happened', async () => {
    vi.spyOn(adminApi, 'monitor').mockResolvedValue(
      monitorData({
        openCount: 0,
        open: [],
        catchUp: {
          since: 'x',
          newIncidents: 0,
          byKind: {},
          actions: [],
          donations: 0,
          donationsLimited: 0,
        },
      }),
    )
    const wrapper = await mountPage()
    expect(wrapper.find('[data-test="catch-up"]').exists()).toBe(false)
    expect(wrapper.text()).toContain('Sin incidentes abiertos')
  })

  it('reviews an incident with the note typed by the referee', async () => {
    const review = vi
      .spyOn(adminApi, 'reviewIncident')
      .mockResolvedValue(incident({ status: 'reviewed' }))
    vi.spyOn(adminApi, 'monitor').mockResolvedValue(monitorData())
    const wrapper = await mountPage()
    await wrapper.find('input[aria-label="Nota del incidente 7"]').setValue(' lag del emulador ')
    const button = wrapper.findAll('button').find((b) => b.text() === 'Revisado')!
    await button.trigger('click')
    await flushPromises()
    expect(review).toHaveBeenCalledWith(7, 'reviewed', 'lag del emulador')
  })

  it('shows the error when the monitor cannot be loaded', async () => {
    vi.spyOn(adminApi, 'monitor').mockRejectedValue(
      new AdminApiError(0, 'network', 'No se pudo conectar con el servidor.'),
    )
    const wrapper = await mountPage()
    expect(wrapper.text()).toContain('No se pudo conectar')
  })

  it('keeps a log book: anyone writes, only the author or an admin deletes', async () => {
    const add = vi.spyOn(adminApi, 'addNote').mockResolvedValue({
      id: 1,
      ts: '2026-10-07T12:00:00.000Z',
      racerId: null,
      author: 'ana',
      text: 'x',
    })
    const del = vi.spyOn(adminApi, 'deleteNote').mockResolvedValue({ ok: true })
    vi.spyOn(adminApi, 'monitor').mockResolvedValue(
      monitorData({
        notes: [
          {
            id: 1,
            ts: '2026-10-07T12:00:00.000Z',
            racerId: 'cuaco',
            author: 'ana',
            text: 'Vuelve en 5',
          },
          {
            id: 2,
            ts: '2026-10-07T12:05:00.000Z',
            racerId: null,
            author: 'beto',
            text: 'Cambio de turno',
          },
        ],
      }),
    )
    const wrapper = await mountPage()
    useAdminStore().user = { id: 1, username: 'ana', role: 'moderator' }
    await flushPromises()
    const notes = wrapper.find('[data-test="notes"]')
    expect(notes.text()).toContain('Vuelve en 5')
    expect(notes.findAll('button')).toHaveLength(1) // only her own
    await notes.find('button').trigger('click')
    expect(del).toHaveBeenCalledWith(1)

    await wrapper.find('input[aria-label="Nueva nota"]').setValue('  Todo bien  ')
    await wrapper.find('form button[type="submit"]').trigger('submit')
    await wrapper.find('form').trigger('submit')
    await flushPromises()
    expect(add).toHaveBeenCalledWith('  Todo bien  ', undefined)
  })
})
