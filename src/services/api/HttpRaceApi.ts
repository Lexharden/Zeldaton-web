import { runtime } from '@/config/runtime'
import { ApiError, type RaceApi } from './RaceApi'

/** REST adapter for the real Zeldatón/HiveShock backend. */
export class HttpRaceApi implements RaceApi {
  constructor(
    private baseUrl = runtime.apiUrl,
    private timeoutMs = runtime.requestTimeoutMs,
  ) {}

  private async request<T>(path: string): Promise<T> {
    const controller = new AbortController()
    const timer = setTimeout(() => controller.abort(), this.timeoutMs)
    try {
      const res = await fetch(`${this.baseUrl}${path}`, {
        signal: controller.signal,
        headers: { Accept: 'application/json' },
      })
      if (res.status === 404) throw new ApiError('not-found', `Not found: ${path}`, 404)
      if (!res.ok) throw new ApiError('http', `HTTP ${res.status} for ${path}`, res.status)
      try {
        return (await res.json()) as T
      } catch {
        throw new ApiError('malformed', `Malformed JSON from ${path}`)
      }
    } catch (error) {
      if (error instanceof ApiError) throw error
      if (error instanceof DOMException && error.name === 'AbortError') {
        throw new ApiError('timeout', `Request timed out: ${path}`)
      }
      throw new ApiError('network', `Backend unavailable: ${path}`)
    } finally {
      clearTimeout(timer)
    }
  }

  getEvent = () => this.request<Awaited<ReturnType<RaceApi['getEvent']>>>('/event')
  getCatalog = () => this.request<Awaited<ReturnType<RaceApi['getCatalog']>>>('/catalog')
  getRacers = () => this.request<Awaited<ReturnType<RaceApi['getRacers']>>>('/racers')
  getStandings = () => this.request<Awaited<ReturnType<RaceApi['getStandings']>>>('/standings')
  getRacer = (id: string) =>
    this.request<Awaited<ReturnType<RaceApi['getRacer']>>>(`/racers/${encodeURIComponent(id)}`)
  getStreams = () => this.request<Awaited<ReturnType<RaceApi['getStreams']>>>('/streams')
  getActivity = () => this.request<Awaited<ReturnType<RaceApi['getActivity']>>>('/activity')
  getHiveShockStats = () =>
    this.request<Awaited<ReturnType<RaceApi['getHiveShockStats']>>>('/hiveshock/stats')
  getDonors = () => this.request<Awaited<ReturnType<RaceApi['getDonors']>>>('/donors?limit=10')
  getClocks = () => this.request<Awaited<ReturnType<RaceApi['getClocks']>>>('/clocks')
}
