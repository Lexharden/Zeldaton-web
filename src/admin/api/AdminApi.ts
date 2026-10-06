import { runtime } from '@/config/runtime'
import type { EventInfo } from '@/types/event'
import type { CatalogItem, CatalogObjective } from '@/types/catalog'
import type { Racer } from '@/types/racer'
import type {
  AdminCatalog,
  AuditRow,
  DiscordStatus,
  DonationsResponse,
  EventAction,
  EventPatchInput,
  LoginResponse,
  MeResponse,
  NewRacerInput,
  Overview,
  OverviewRacer,
  RacerAction,
  RacerActionBody,
  RacerPatchInput,
  UserRow,
} from '../types'

export class AdminApiError extends Error {
  constructor(
    public status: number,
    public code: string,
    message: string,
    /** Seconds to wait when the server answered 429. */
    public retryAfter?: number,
  ) {
    super(message)
    this.name = 'AdminApiError'
  }
}

type Fetch = typeof fetch

/**
 * REST client for `/api/admin`. Authentication is the session cookie the server sets at login; every
 * write also carries the CSRF token that only this page received. Same-origin in production (the
 * reverse proxy serves site and API) and through the Vite proxy in development.
 */
export class AdminApi {
  /** Set after login/restore; sent as `X-CSRF-Token` on every write. */
  csrf: string | null = null
  /** Called on any 401 (session expired or revoked) so the app can go back to the login page. */
  onUnauthorized: (() => void) | null = null

  constructor(
    private base = `${runtime.apiUrl || '/api'}/admin`,
    private fetcher: Fetch = (...args) => fetch(...args),
  ) {}

  private async request<T>(method: string, path: string, body?: unknown): Promise<T> {
    const headers: Record<string, string> = { Accept: 'application/json' }
    if (body !== undefined) headers['Content-Type'] = 'application/json'
    if (method !== 'GET' && this.csrf) headers['X-CSRF-Token'] = this.csrf
    let res: Response
    try {
      res = await this.fetcher(`${this.base}${path}`, {
        method,
        credentials: 'include',
        headers,
        body: body === undefined ? undefined : JSON.stringify(body),
      })
    } catch {
      throw new AdminApiError(0, 'network', 'No se pudo conectar con el servidor.')
    }
    if (res.status === 204) return undefined as T
    let data: unknown = null
    try {
      data = await res.json()
    } catch {
      /* empty or non-JSON body */
    }
    if (!res.ok) {
      const err = (data ?? {}) as { error?: string; message?: string }
      if (res.status === 401 && !path.startsWith('/auth/login')) this.onUnauthorized?.()
      const retry = Number(res.headers.get('retry-after')) || undefined
      throw new AdminApiError(
        res.status,
        err.error ?? 'http',
        err.message ?? `Error ${res.status}`,
        retry,
      )
    }
    return data as T
  }

  // ---- session
  async login(username: string, password: string): Promise<LoginResponse> {
    const out = await this.request<LoginResponse>('POST', '/auth/login', { username, password })
    this.csrf = out.csrfToken
    return out
  }
  async logout(): Promise<void> {
    try {
      await this.request('POST', '/auth/logout', {})
    } finally {
      this.csrf = null
    }
  }
  async me(): Promise<MeResponse> {
    const out = await this.request<MeResponse>('GET', '/me')
    this.csrf = out.csrfToken
    return out
  }
  changeOwnPassword(currentPassword: string, password: string) {
    return this.request<{ ok: true }>('POST', '/me/password', { currentPassword, password })
  }

  // ---- dashboard / event
  overview = () => this.request<Overview>('GET', '/overview')
  updateEvent = (patch: EventPatchInput) => this.request<EventInfo>('PUT', '/event', patch)
  eventAction = (action: EventAction, reason?: string) =>
    this.request<EventInfo>('POST', `/event/${action}`, { reason })

  // ---- racers
  listRacers = () => this.request<OverviewRacer[]>('GET', '/racers')
  createRacer = (input: NewRacerInput) =>
    this.request<{ racer: Racer; token: string }>('POST', '/racers', input)
  updateRacer = (id: string, patch: RacerPatchInput) =>
    this.request<Racer>('PATCH', `/racers/${encodeURIComponent(id)}`, patch)
  deleteRacer = (id: string) => this.request<void>('DELETE', `/racers/${encodeURIComponent(id)}`)
  rotateToken = (id: string) =>
    this.request<{ token: string }>('POST', `/racers/${encodeURIComponent(id)}/token`, {})
  racerAction = (id: string, action: RacerAction, body: RacerActionBody = {}) =>
    this.request<Racer>('POST', `/racers/${encodeURIComponent(id)}/actions/${action}`, body)

  // ---- accounts
  listUsers = () => this.request<UserRow[]>('GET', '/users')
  createUser = (input: { username: string; password: string; role: 'admin' | 'moderator' }) =>
    this.request<UserRow>('POST', '/users', input)
  updateUser = (id: number, patch: { role?: 'admin' | 'moderator'; disabled?: boolean }) =>
    this.request<UserRow>('PATCH', `/users/${id}`, patch)
  deleteUser = (id: number) => this.request<void>('DELETE', `/users/${id}`)
  resetPassword = (id: number, password: string) =>
    this.request<{ ok: true }>('POST', `/users/${id}/password`, { password })

  // ---- catalog
  catalog = () => this.request<AdminCatalog>('GET', '/catalog')
  saveItem = (item: CatalogItem) =>
    this.request<CatalogItem>('PUT', `/catalog/items/${encodeURIComponent(item.id)}`, item)
  deleteItem = (id: string) =>
    this.request<void>('DELETE', `/catalog/items/${encodeURIComponent(id)}`)
  saveObjective = (objective: CatalogObjective) =>
    this.request<CatalogObjective>(
      'PUT',
      `/catalog/objectives/${encodeURIComponent(objective.id)}`,
      objective,
    )
  deleteObjective = (id: string) =>
    this.request<void>('DELETE', `/catalog/objectives/${encodeURIComponent(id)}`)

  /** Wipes the test run and puts the event back to "upcoming". `confirm` must be REINICIAR. */
  resetEvent = (body: { confirm: string; startAtUtc?: string; leaveRehearsal: boolean }) =>
    this.request<EventInfo>('POST', '/event/reset', body)

  // ---- Discord announcements
  discord = () => this.request<DiscordStatus>('GET', '/discord')
  setDiscord = (enabled: boolean) =>
    this.request<{ enabled: boolean }>('PUT', '/discord', { enabled })
  testDiscord = () => this.request<{ ok: true }>('POST', '/discord/test', {})

  // ---- public donors board
  setDonorsVisible = (enabled: boolean) =>
    this.request<{ donorsPublic: boolean }>('PUT', '/donors/visibility', { enabled })
  setDonorHidden = (platform: string, viewer: string, hidden: boolean) =>
    this.request<{ ok: true }>('PUT', '/donors/hidden', { platform, viewer, hidden })

  // ---- racer photos
  /** Uploads the photo itself (not JSON); the server stores it and points the racer's avatar at it. */
  async uploadRacerPhoto(id: string, file: Blob): Promise<Racer> {
    const headers: Record<string, string> = { Accept: 'application/json' }
    if (this.csrf) headers['X-CSRF-Token'] = this.csrf
    headers['Content-Type'] = file.type || 'application/octet-stream'
    let res: Response
    try {
      res = await this.fetcher(`${this.base}/racers/${encodeURIComponent(id)}/photo`, {
        method: 'POST',
        credentials: 'include',
        headers,
        body: file,
      })
    } catch {
      throw new AdminApiError(0, 'network', 'No se pudo conectar con el servidor.')
    }
    let data: unknown = null
    try {
      data = await res.json()
    } catch {
      /* empty or non-JSON body */
    }
    if (!res.ok) {
      const err = (data ?? {}) as { error?: string; message?: string }
      if (res.status === 401) this.onUnauthorized?.()
      throw new AdminApiError(
        res.status,
        err.error ?? 'http',
        res.status === 413
          ? 'La foto pesa demasiado (máximo 2 MB).'
          : (err.message ?? `Error ${res.status}`),
      )
    }
    return data as Racer
  }
  deleteRacerPhoto = (id: string) =>
    this.request<Racer>('DELETE', `/racers/${encodeURIComponent(id)}/photo`)

  // ---- audit
  audit = (limit = 100) => this.request<AuditRow[]>('GET', `/audit?limit=${limit}`)

  // ---- time donations
  donations = (limit = 200, racer?: string) =>
    this.request<DonationsResponse>(
      'GET',
      `/donations?limit=${limit}${racer ? `&racer=${encodeURIComponent(racer)}` : ''}`,
    )
}

/** One client for the whole panel. */
export const adminApi = new AdminApi()
