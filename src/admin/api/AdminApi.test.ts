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
})
