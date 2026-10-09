import { describe, expect, it, vi } from 'vitest'
import { AdminApi, AdminApiError } from './AdminApi'

function fakeFetch(
  responses: Array<{ status?: number; body?: unknown; headers?: Record<string, string> } | Error>,
) {
  const calls: Array<{ url: string; init: RequestInit }> = []
  const fetcher = vi.fn(async (url: string | URL | Request, init?: RequestInit) => {
    calls.push({ url: String(url), init: init ?? {} })
    const next = responses.shift()
    if (!next) throw new Error('unexpected request')
    if (next instanceof Error) throw next
    const status = next.status ?? 200
    return new Response(status === 204 ? null : JSON.stringify(next.body ?? {}), {
      status,
      headers: { 'content-type': 'application/json', ...next.headers },
    })
  })
  return { fetcher: fetcher as unknown as typeof fetch, calls }
}

const headersOf = (init: RequestInit) => init.headers as Record<string, string>

describe('AdminApi', () => {
  it('logs in, remembers the CSRF token and sends it on writes but not on reads', async () => {
    const { fetcher, calls } = fakeFetch([
      {
        body: {
          user: { id: 1, username: 'ana', role: 'admin' },
          csrfToken: 'csrf-1',
          expiresAtUtc: 'x',
        },
      },
      { body: { ok: true } },
      { body: [] },
    ])
    const api = new AdminApi('/api/admin', fetcher)
    await api.login('ana', 'a-strong-passphrase')
    await api.eventAction('start')
    await api.listRacers()

    expect(calls[0].url).toBe('/api/admin/auth/login')
    expect(calls[0].init.credentials).toBe('include')
    expect(headersOf(calls[0].init)['X-CSRF-Token']).toBeUndefined() // nothing to send yet
    expect(headersOf(calls[1].init)['X-CSRF-Token']).toBe('csrf-1')
    expect(calls[1].init.method).toBe('POST')
    expect(JSON.parse(String(calls[1].init.body))).toEqual({})
    expect(headersOf(calls[2].init)['X-CSRF-Token']).toBeUndefined()
  })

  it('builds the right requests', async () => {
    const { fetcher, calls } = fakeFetch(Array.from({ length: 6 }, () => ({ body: {} })))
    const api = new AdminApi('/api/admin', fetcher)
    api.csrf = 'c'
    await api.racerAction('ralbat', 'adjust-time', { deltaSeconds: -60, reason: 'lag' })
    await api.updateRacer('a b', { displayName: 'X' })
    await api.saveItem({
      id: 'magic-beans',
      group: 'tool',
      age: 'child',
      nameEs: 'a',
      nameEn: 'b',
      short: 'FM',
      sortOrder: 1,
      enabled: true,
    })
    await api.audit(50)
    await api.updateUser(3, { disabled: true })
    await api.eventAction('finish', 'fin')

    expect(calls[0].url).toBe('/api/admin/racers/ralbat/actions/adjust-time')
    expect(JSON.parse(String(calls[0].init.body))).toEqual({ deltaSeconds: -60, reason: 'lag' })
    expect(calls[1].url).toBe('/api/admin/racers/a%20b') // ids are encoded
    expect(calls[1].init.method).toBe('PATCH')
    expect(calls[2].url).toBe('/api/admin/catalog/items/magic-beans')
    expect(calls[2].init.method).toBe('PUT')
    expect(calls[3].url).toBe('/api/admin/audit?limit=50')
    expect(calls[4].url).toBe('/api/admin/users/3')
    expect(calls[5].url).toBe('/api/admin/event/finish')
  })

  it('a 401 on any call (not the login) tells the app the session is over', async () => {
    const { fetcher } = fakeFetch([
      { status: 401, body: { error: 'unauthorized', message: 'unauthorized' } },
      { status: 401, body: { error: 'unauthorized', message: 'unauthorized' } },
    ])
    const api = new AdminApi('/api/admin', fetcher)
    const onUnauthorized = vi.fn()
    api.onUnauthorized = onUnauthorized

    await expect(api.overview()).rejects.toMatchObject({ status: 401 })
    expect(onUnauthorized).toHaveBeenCalledTimes(1)

    // A wrong password is a normal answer of the login form, not an expired session.
    await expect(api.login('ana', 'nope')).rejects.toBeInstanceOf(AdminApiError)
    expect(onUnauthorized).toHaveBeenCalledTimes(1)
  })

  it('maps server errors, rate limits, empty answers and network failures', async () => {
    const { fetcher } = fakeFetch([
      {
        status: 429,
        body: { error: 'too_many_requests', message: 'too many attempts' },
        headers: { 'retry-after': '840' },
      },
      { status: 409, body: { error: 'conflict', message: 'conflict: the event is already live' } },
      { status: 204 },
      new TypeError('Failed to fetch'),
    ])
    const api = new AdminApi('/api/admin', fetcher)

    const limited = await api.login('a', 'b').catch((e) => e as AdminApiError)
    expect(limited).toMatchObject({ status: 429, code: 'too_many_requests', retryAfter: 840 })
    await expect(api.eventAction('start')).rejects.toMatchObject({
      status: 409,
      message: expect.stringContaining('already live'),
    })
    await expect(api.deleteRacer('x')).resolves.toBeUndefined()
    await expect(api.overview()).rejects.toMatchObject({ status: 0, code: 'network' })
  })

  it('logout forgets the CSRF token even if the call fails', async () => {
    const { fetcher } = fakeFetch([{ status: 500, body: { error: 'internal' } }])
    const api = new AdminApi('/api/admin', fetcher)
    api.csrf = 'c'
    await expect(api.logout()).rejects.toBeInstanceOf(AdminApiError)
    expect(api.csrf).toBeNull()
  })

  it("uploads a racer's photo as the raw body, with the CSRF token, and can remove it", async () => {
    const { fetcher, calls } = fakeFetch([
      { body: { user: { id: 1, username: 'ana', role: 'admin' }, csrfToken: 'csrf-9' } },
      { status: 201, body: { id: 'ralbat', avatarUrl: '/api/media/racers/ralbat-1.png' } },
      { status: 413, body: {} },
      { body: { id: 'ralbat' } },
    ])
    const api = new AdminApi('/api/admin', fetcher)
    await api.login('ana', 'pw')
    const file = new Blob([new Uint8Array([1, 2, 3])], { type: 'image/png' })
    const out = await api.uploadRacerPhoto('ralbat', file)
    expect(out.avatarUrl).toBe('/api/media/racers/ralbat-1.png')
    expect(calls[1].url).toBe('/api/admin/racers/ralbat/photo')
    expect(calls[1].init.method).toBe('POST')
    expect(calls[1].init.body).toBe(file)
    expect(headersOf(calls[1].init)['X-CSRF-Token']).toBe('csrf-9')
    expect(headersOf(calls[1].init)['Content-Type']).toBe('image/png')
    await expect(api.uploadRacerPhoto('ralbat', file)).rejects.toMatchObject({ status: 413 })
    await api.deleteRacerPhoto('ralbat')
    expect(calls[3].url).toBe('/api/admin/racers/ralbat/photo')
    expect(calls[3].init.method).toBe('DELETE')
  })

  it('resets the event with the confirmation word, a new start and the rehearsal flag', async () => {
    const { fetcher, calls } = fakeFetch([{ body: { status: 'upcoming', rehearsal: false } }])
    const api = new AdminApi('/api/admin', fetcher)
    const out = await api.resetEvent({
      confirm: 'REINICIAR',
      startAtUtc: '2026-10-07T12:00:00.000Z',
      leaveRehearsal: true,
    })
    expect(out.status).toBe('upcoming')
    expect(calls[0].url).toBe('/api/admin/event/reset')
    expect(calls[0].init.method).toBe('POST')
    expect(JSON.parse(String(calls[0].init.body))).toEqual({
      confirm: 'REINICIAR',
      startAtUtc: '2026-10-07T12:00:00.000Z',
      leaveRehearsal: true,
    })
  })

  it('hides a donor and switches the public board with the CSRF token', async () => {
    const { fetcher, calls } = fakeFetch([
      { body: { user: { id: 1, username: 'ana', role: 'admin' }, csrfToken: 'csrf-3' } },
      { body: { ok: true } },
      { body: { donorsPublic: false } },
    ])
    const api = new AdminApi('/api/admin', fetcher)
    await api.login('ana', 'pw')
    await api.setDonorHidden('tiktok', 'Fan Uno', true)
    await api.setDonorsVisible(false)
    expect(calls[1].url).toBe('/api/admin/donors/hidden')
    expect(calls[1].init.method).toBe('PUT')
    expect(JSON.parse(String(calls[1].init.body))).toEqual({
      platform: 'tiktok',
      viewer: 'Fan Uno',
      hidden: true,
    })
    expect(headersOf(calls[1].init)['X-CSRF-Token']).toBe('csrf-3')
    expect(calls[2].url).toBe('/api/admin/donors/visibility')
    expect(JSON.parse(String(calls[2].init.body))).toEqual({ enabled: false })
  })

  it('reads the Discord state, patches it and tests each channel', async () => {
    const state = {
      rehearsal: false,
      channels: {
        public: {
          configured: true,
          enabled: false,
          lastSentAt: null,
          lastError: null,
          dropped: 0,
          queued: 0,
        },
        staff: {
          configured: false,
          enabled: false,
          lastSentAt: null,
          lastError: null,
          dropped: 0,
          queued: 0,
        },
      },
      kinds: [],
      thresholds: {
        disconnectMinutes: 3,
        lowTimeMinutes: 10,
        jumpPercent: 20,
        jumpWindowSeconds: 120,
      },
    }
    const { fetcher, calls } = fakeFetch([
      { body: { user: { id: 1, username: 'ana', role: 'admin' }, csrfToken: 'csrf-4' } },
      { body: state },
      { body: state },
      { body: { ok: true } },
    ])
    const api = new AdminApi('/api/admin', fetcher)
    await api.login('ana', 'pw')
    expect((await api.discord()).channels.public.configured).toBe(true)
    await api.setDiscord({
      staffEnabled: true,
      kinds: { boss: false },
      thresholds: { lowTimeMinutes: 15 },
    })
    await api.testDiscord('staff')
    expect(calls[1].url).toBe('/api/admin/discord')
    expect(calls[2].init.method).toBe('PUT')
    expect(JSON.parse(String(calls[2].init.body))).toEqual({
      staffEnabled: true,
      kinds: { boss: false },
      thresholds: { lowTimeMinutes: 15 },
    })
    expect(calls[3].url).toBe('/api/admin/discord/test')
    expect(JSON.parse(String(calls[3].init.body))).toEqual({ channel: 'staff' })
    expect(headersOf(calls[3].init)['X-CSRF-Token']).toBe('csrf-4')
  })

  it("sends the donation rate with the rest of the limits (null = use HiveShock's number)", async () => {
    const { fetcher, calls } = fakeFetch([
      { body: { user: { id: 1, username: 'ana', role: 'admin' }, csrfToken: 'csrf-7' } },
      { body: {} },
    ])
    const api = new AdminApi('/api/admin', fetcher)
    await api.login('ana', 'pw')
    await api.updateEvent({
      donationTime: {
        enabled: true,
        allowAdd: true,
        allowRemove: true,
        maxSecondsPerDonation: 300,
        maxAddedSecondsPerDay: 3600,
        maxRemovedSecondsPerDay: 3600,
        secondsPerDiamond: 3,
        secondsPerBit: null,
      },
    })
    const body = JSON.parse(String(calls[1].init.body))
    expect(body.donationTime.secondsPerDiamond).toBe(3)
    expect(body.donationTime.secondsPerBit).toBeNull()
  })

  it('reads the day by day statistics', async () => {
    const { fetcher, calls } = fakeFetch([{ body: { days: ['2026-10-07'], rows: [] } }])
    const api = new AdminApi('/api/admin', fetcher)
    const out = await api.statsDays()
    expect(out.days).toEqual(['2026-10-07'])
    expect(calls[0].url).toBe('/api/admin/stats/days')
  })
})
