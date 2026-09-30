// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createMemoryHistory, createRouter } from 'vue-router'
import type { Component } from 'vue'
import { DEFAULT_CATALOG } from '@/config/catalog'
import type { Racer } from '@/types/racer'
import { adminApi, AdminApiError } from '../api/AdminApi'
import { answer, confirm, pending } from '../composables/useConfirm'
import { useToasts } from '../composables/useToasts'
import { useAdminStore } from '../stores/admin'
import type { Overview, UserRow } from '../types'
import Accounts from './Accounts.vue'
import Audit from './Audit.vue'
import Catalog from './Catalog.vue'
import Dashboard from './Dashboard.vue'
import EventPage from './EventPage.vue'
import Login from './Login.vue'
import Racers from './Racers.vue'
import ConfirmHost from '../components/ConfirmHost.vue'

const racer = (id: string, over: Partial<Racer> = {}): Racer => ({
  id,
  displayName: id.toUpperCase(),
  slug: id,
  timezone: 'America/Mexico_City',
  status: 'live',
  elapsedSeconds: 600,
  remainingSeconds: 13800,
  progressPercentage: 40,
  completedObjectives: ['kokiri-forest'],
  channels: [{ platform: 'twitch', handle: id, url: `https://twitch.tv/${id}` }],
  currentArea: 'water-temple',
  stats: { age: 'adult' },
  ...over,
})

const overview: Overview = {
  serverTimeUtc: '2026-10-07T13:00:00.000Z',
  event: {
    id: 'z',
    name: 'Zeldathon',
    game: 'OoT',
    edition: '2026',
    status: 'live',
    startAtUtc: '2026-10-07T12:00:00.000Z',
    timezone: 'America/Mexico_City',
    dailyBudgetSeconds: 14400,
    dailyResetLocalTime: '06:00',
    rules: {
      winCondition: 'first to finish',
      requiredObjectiveIds: ['kokiri-forest', 'deku-tree'],
    },
  },
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
      racer: racer('cuaco', { status: 'offline', stats: {} }),
      connected: false,
      lastHeartbeatUtc: null,
      heartbeatAgeSeconds: null,
    },
  ],
  alerts: [
    {
      level: 'info',
      code: 'offline',
      racerId: 'cuaco',
      racerName: 'CUACO',
      message: 'CUACO is offline.',
    },
  ],
  hiveshock: {
    connectedRacers: 1,
    gameEvents: 10,
    itemEvents: 2,
    progressEvents: 3,
    chatEvents: 4,
  },
  activity: [
    {
      id: 'a1',
      timestampUtc: '2026-10-07T13:00:00.000Z',
      kind: 'item',
      message: 'Ralbat acquired hookshot',
      code: 'ITEM_ACQUIRED',
    },
  ],
  winner: null,
  catalogVersion: 'abc',
}

let mounted: VueWrapper[] = []

async function mountPage(component: Component, path = '/admin') {
  const pinia = createPinia()
  setActivePinia(pinia)
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [{ path: '/:p(.*)*', name: 'any', component: { template: '<div />' } }],
  })
  await router.push(path)
  await router.isReady()
  const wrapper = mount(component, {
    global: { plugins: [pinia, router] },
    attachTo: document.body,
  })
  mounted.push(wrapper)
  await flushPromises()
  return { wrapper, router }
}

beforeEach(() => {
  vi.restoreAllMocks()
  useToasts().toasts.value = []
})
afterEach(() => {
  mounted.forEach((w) => w.unmount())
  mounted = []
  document.body.innerHTML = ''
})

describe('Login', () => {
  it('shows a clear message for a wrong password and clears the field', async () => {
    vi.spyOn(adminApi, 'login').mockRejectedValue(
      new AdminApiError(401, 'unauthorized', 'unauthorized'),
    )
    const { wrapper } = await mountPage(Login, '/admin/login')
    await wrapper.find('#u').setValue('ana')
    await wrapper.find('#p').setValue('wrong')
    await wrapper.find('form').trigger('submit')
    await flushPromises()
    expect(wrapper.text()).toContain('Usuario o contraseña incorrectos')
    expect((wrapper.find('#p').element as HTMLInputElement).value).toBe('')
  })

  it('tells the user how long to wait when locked out', async () => {
    vi.spyOn(adminApi, 'login').mockRejectedValue(
      new AdminApiError(429, 'too_many_requests', 'x', 840),
    )
    const { wrapper } = await mountPage(Login, '/admin/login')
    await wrapper.find('#u').setValue('ana')
    await wrapper.find('#p').setValue('whatever-password')
    await wrapper.find('form').trigger('submit')
    await flushPromises()
    expect(wrapper.text()).toContain('14 min')
  })

  it('signs in and goes where the guard wanted to send the user (only inside the panel)', async () => {
    vi.spyOn(adminApi, 'login').mockResolvedValue({
      user: { id: 1, username: 'ana', role: 'admin' },
      csrfToken: 'c',
      expiresAtUtc: 'x',
    })
    const { wrapper, router } = await mountPage(Login, '/admin/login?next=/admin/racers')
    await wrapper.find('#u').setValue('ana')
    await wrapper.find('#p').setValue('a-strong-passphrase')
    await wrapper.find('form').trigger('submit')
    await flushPromises()
    expect(router.currentRoute.value.fullPath).toBe('/admin/racers')
    expect(useAdminStore().user?.username).toBe('ana')

    const other = await mountPage(Login, '/admin/login?next=https://evil.example')
    await other.wrapper.find('#u').setValue('ana')
    await other.wrapper.find('#p').setValue('a-strong-passphrase')
    await other.wrapper.find('form').trigger('submit')
    await flushPromises()
    expect(other.router.currentRoute.value.fullPath).toBe('/admin')
  })

  it('explains an expired session', async () => {
    const { wrapper } = await mountPage(Login, '/admin/login?expired=1')
    expect(wrapper.text()).toContain('Tu sesión terminó')
  })
})

describe('Dashboard', () => {
  it('renders the event, the racers, alerts and activity', async () => {
    vi.spyOn(adminApi, 'overview').mockResolvedValue(overview)
    const { wrapper } = await mountPage(Dashboard)
    const text = wrapper.text()
    expect(text).toContain('Zeldathon')
    expect(text).toContain('RALBAT')
    expect(text).toContain('CUACO')
    expect(text).toContain('1 / 2') // connected
    expect(text).toContain('CUACO is offline.')
    expect(text).toContain('Ralbat acquired hookshot')
    expect(text).toContain('sin señal')
    expect(wrapper.findAll('tbody tr')).toHaveLength(2)
  })

  it('offers the event controls only to admins', async () => {
    vi.spyOn(adminApi, 'overview').mockResolvedValue(overview)
    const { wrapper } = await mountPage(Dashboard)
    expect(wrapper.text()).not.toContain('Pausar evento')

    useAdminStore().user = { id: 1, username: 'ana', role: 'admin' }
    await flushPromises()
    expect(wrapper.text()).toContain('Pausar evento')
    expect(wrapper.text()).toContain('Terminar evento')
  })

  it('shows the error when the server cannot be reached', async () => {
    vi.spyOn(adminApi, 'overview').mockRejectedValue(
      new AdminApiError(0, 'network', 'No se pudo conectar con el servidor.'),
    )
    const { wrapper } = await mountPage(Dashboard)
    expect(wrapper.text()).toContain('No se pudo conectar')
  })
})

describe('Catalog', () => {
  const catalog = {
    version: 'v',
    items: DEFAULT_CATALOG.items,
    objectives: DEFAULT_CATALOG.objectives,
  }

  it('lists the items and filters by Link', async () => {
    vi.spyOn(adminApi, 'catalog').mockResolvedValue(catalog)
    const { wrapper } = await mountPage(Catalog)
    const all = wrapper.findAll('tbody tr').length
    expect(all).toBe(DEFAULT_CATALOG.items.length)

    await wrapper.find('select[aria-label="Link"]').setValue('child')
    const child = wrapper.findAll('tbody tr').length
    expect(child).toBe(DEFAULT_CATALOG.items.filter((i) => i.age === 'child').length)
    expect(child).toBeLessThan(all)
    expect(wrapper.text()).toContain('Espada Kokiri')
  })

  it('creating an item validates the id and sends the right payload', async () => {
    vi.spyOn(adminApi, 'catalog').mockResolvedValue(catalog)
    const save = vi.spyOn(adminApi, 'saveItem').mockImplementation(async (i) => i)
    const { wrapper } = await mountPage(Catalog)
    await wrapper
      .findAll('button')
      .find((b) => b.text().includes('Nuevo ítem'))!
      .trigger('click')
    await flushPromises()

    const dialog = document.body
    const set = async (sel: string, value: string) => {
      const el = dialog.querySelector(sel) as HTMLInputElement
      el.value = value
      el.dispatchEvent(new Event('input'))
      await flushPromises()
    }
    await set('#it-id', 'Bad Id')
    expect(dialog.textContent).toContain('El identificador')
    expect((dialog.querySelector('button[type=submit]') as HTMLButtonElement).disabled).toBe(true)

    await set('#it-id', 'magic-beans')
    await set('#it-es', 'Frijoles Mágicos')
    await set('#it-en', 'Magic Beans')
    await set('#it-short', 'FM')
    expect((dialog.querySelector('button[type=submit]') as HTMLButtonElement).disabled).toBe(false)
    dialog.querySelector('form')!.dispatchEvent(new Event('submit'))
    await flushPromises()
    expect(save).toHaveBeenCalledTimes(1)
    const sent = save.mock.calls[0][0]
    expect(sent).toMatchObject({
      id: 'magic-beans',
      nameEs: 'Frijoles Mágicos',
      short: 'FM',
      enabled: true,
    })
    expect('isNew' in sent).toBe(false)
  })

  it('a duplicate id is rejected before it reaches the server', async () => {
    vi.spyOn(adminApi, 'catalog').mockResolvedValue(catalog)
    const { wrapper } = await mountPage(Catalog)
    await wrapper
      .findAll('button')
      .find((b) => b.text().includes('Nuevo ítem'))!
      .trigger('click')
    await flushPromises()
    const el = document.body.querySelector('#it-id') as HTMLInputElement
    el.value = 'hookshot'
    el.dispatchEvent(new Event('input'))
    await flushPromises()
    expect(document.body.textContent).toContain('Ya existe un ítem')
  })
})

describe('Racers', () => {
  it('lists racers and hides the admin-only actions from moderators', async () => {
    vi.spyOn(adminApi, 'listRacers').mockResolvedValue(overview.racers)
    const { wrapper } = await mountPage(Racers)
    expect(wrapper.text()).toContain('RALBAT')
    expect(wrapper.text()).toContain('HiveShock conectado')
    expect(wrapper.text()).toContain('Controlar')
    expect(wrapper.text()).not.toContain('Nuevo corredor')
    expect(wrapper.text()).not.toContain('Token')

    useAdminStore().user = { id: 1, username: 'ana', role: 'admin' }
    await flushPromises()
    expect(wrapper.text()).toContain('Nuevo corredor')
    expect(wrapper.text()).toContain('Token')
  })

  it('rotating a token asks first and shows the new token once', async () => {
    vi.spyOn(adminApi, 'listRacers').mockResolvedValue(overview.racers)
    const rotate = vi
      .spyOn(adminApi, 'rotateToken')
      .mockResolvedValue({ token: 'new-secret-token' })
    const { wrapper } = await mountPage(Racers)
    useAdminStore().user = { id: 1, username: 'ana', role: 'admin' }
    await flushPromises()
    const host = mount(ConfirmHost, { attachTo: document.body })
    mounted.push(host)

    await wrapper
      .findAll('button')
      .find((b) => b.text().includes('Token'))!
      .trigger('click')
    await flushPromises()
    expect(rotate).not.toHaveBeenCalled() // waiting for the confirmation
    expect(pending.value?.title).toBe('Rotar el token')
    answer(true)
    await flushPromises()
    expect(rotate).toHaveBeenCalledWith('ralbat')
    expect(document.body.textContent).toContain('new-secret-token')
    expect(document.body.textContent).toContain('no se vuelve a mostrar')
  })
})

describe('Accounts', () => {
  const users: UserRow[] = [
    {
      id: 1,
      username: 'ana',
      role: 'admin',
      disabled: false,
      createdAt: '2026-01-01T00:00:00Z',
      lastLoginAt: null,
    },
    {
      id: 2,
      username: 'bob',
      role: 'moderator',
      disabled: false,
      createdAt: '2026-01-01T00:00:00Z',
      lastLoginAt: null,
    },
  ]

  it('lists accounts and protects your own row', async () => {
    vi.spyOn(adminApi, 'listUsers').mockResolvedValue(users)
    const { wrapper } = await mountPage(Accounts)
    useAdminStore().user = { id: 1, username: 'ana', role: 'admin' }
    await flushPromises()
    expect(wrapper.text()).toContain('(tú)')
    const rows = wrapper.findAll('tbody tr')
    expect(rows).toHaveLength(2)
    const own = rows[0].findAll('button').filter((b) => b.text().includes('Desactivar'))[0]
    expect((own.element as HTMLButtonElement).disabled).toBe(true)
    const other = rows[1].findAll('button').filter((b) => b.text().includes('Desactivar'))[0]
    expect((other.element as HTMLButtonElement).disabled).toBe(false)
  })

  it('generates a strong initial password when creating an account', async () => {
    vi.spyOn(adminApi, 'listUsers').mockResolvedValue(users)
    const { wrapper } = await mountPage(Accounts)
    await wrapper
      .findAll('button')
      .find((b) => b.text().includes('Nueva cuenta'))!
      .trigger('click')
    await flushPromises()
    const pass = (document.body.querySelector('#ac-pass') as HTMLInputElement).value
    expect(pass.length).toBeGreaterThanOrEqual(12)
  })
})

describe('Audit and Event pages', () => {
  it('audit shows who did what, with the reason, and can be searched', async () => {
    vi.spyOn(adminApi, 'audit').mockResolvedValue([
      {
        id: 2,
        ts: '2026-10-07T13:00:00.000Z',
        actor: 'ana',
        action: 'racer.adjust-time',
        racerId: 'xime',
        payload: { reason: 'lag de 10 min', deltaSeconds: 600 },
      },
      {
        id: 1,
        ts: '2026-10-07T12:00:00.000Z',
        actor: 'bob',
        action: 'event.start',
        racerId: null,
        payload: {},
      },
    ])
    const { wrapper } = await mountPage(Audit)
    expect(wrapper.text()).toContain('Ajustó el tiempo de un corredor')
    expect(wrapper.text()).toContain('Motivo: lag de 10 min')
    expect(wrapper.text()).toContain('+10 min')
    await wrapper.find('input[aria-label="Buscar"]').setValue('bob')
    expect(wrapper.findAll('tbody tr')).toHaveLength(1)
    expect(wrapper.text()).toContain('Inició el evento')
  })

  it('the event form loads the current values and validates the daily budget', async () => {
    vi.spyOn(adminApi, 'overview').mockResolvedValue(overview)
    vi.spyOn(adminApi, 'catalog').mockResolvedValue({
      version: 'v',
      items: [],
      objectives: DEFAULT_CATALOG.objectives,
    })
    const { wrapper } = await mountPage(EventPage)
    expect((wrapper.find('#ev-name').element as HTMLInputElement).value).toBe('Zeldathon')
    expect((wrapper.find('#ev-h').element as HTMLInputElement).value).toBe('4')
    expect(wrapper.text()).toContain('Cada corredor juega hasta 4 h al día')
    expect(wrapper.text()).toContain('Espada'.slice(0, 0) + 'Gran Árbol Deku') // objectives from the catalog

    await wrapper.find('#ev-h').setValue('0')
    await wrapper.find('#ev-m').setValue('0')
    expect(wrapper.text()).toContain('Entre 1 minuto y 24 horas')
    expect((wrapper.find('button[type=submit]').element as HTMLButtonElement).disabled).toBe(true)
  })
})

describe('confirm()', () => {
  it('resolves with the answer and needs the typed text for dangerous actions', async () => {
    const host = mount(ConfirmHost, { attachTo: document.body })
    mounted.push(host)
    const result = confirm({
      title: 'Eliminar',
      message: 'seguro',
      danger: true,
      requireText: 'ralbat',
    })
    await flushPromises()
    const button = () =>
      [...document.body.querySelectorAll('button')].find((b) =>
        b.textContent?.includes('Confirmar'),
      ) as HTMLButtonElement
    expect(button().disabled).toBe(true)
    const input = document.body.querySelector('#confirm-text') as HTMLInputElement
    input.value = 'ralbat'
    input.dispatchEvent(new Event('input'))
    await flushPromises()
    expect(button().disabled).toBe(false)
    button().click()
    await expect(result).resolves.toBe(true)
  })
})
