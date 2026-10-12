import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { raceApi } from '@/services/api'
import { ApiError } from '@/services/api/RaceApi'
import type { Poll } from '@/types/poll'
import { usePollStore } from './poll'

const options = [
  { racerId: 'a', name: 'A' },
  { racerId: 'b', name: 'B' },
]
const open = (voted: string | null = null): Poll => ({
  open: true,
  mode: 'auto',
  options,
  voted,
  ...(voted
    ? {
        totalVotes: 1,
        results: [
          { racerId: voted, votes: 1, percent: 100 },
          { racerId: 'b', votes: 0, percent: 0 },
        ],
      }
    : {}),
})

function fakeStorage() {
  const data = new Map<string, string>()
  return {
    getItem: (k: string) => data.get(k) ?? null,
    setItem: (k: string, v: string) => void data.set(k, v),
  }
}

describe('poll store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.restoreAllMocks()
    vi.unstubAllGlobals()
    vi.stubGlobal('localStorage', fakeStorage())
  })

  it('invites only an open poll the visitor has not voted in', async () => {
    const store = usePollStore()
    expect(store.shouldPrompt).toBe(false) // nothing loaded yet
    vi.spyOn(raceApi, 'getPoll').mockResolvedValueOnce(open())
    await store.load()
    expect(store.canVote).toBe(true)
    expect(store.shouldPrompt).toBe(true)

    vi.spyOn(raceApi, 'getPoll').mockResolvedValueOnce(open('a'))
    await store.load()
    expect(store.shouldPrompt).toBe(false) // the server says this connection voted

    vi.spyOn(raceApi, 'getPoll').mockResolvedValueOnce({ ...open(), open: false })
    await store.load()
    expect(store.shouldPrompt).toBe(false)
  })

  it('"not now" silences the invitation for a day, and survives a new page load', async () => {
    const store = usePollStore()
    vi.spyOn(raceApi, 'getPoll').mockResolvedValue(open())
    await store.load()
    store.snooze()
    expect(store.shouldPrompt).toBe(false)
    expect(store.canVote).toBe(true) // the poll page still lets them vote

    setActivePinia(createPinia())
    const again = usePollStore()
    await again.load()
    expect(again.shouldPrompt).toBe(false)
  })

  it('stores the results after a vote and reports a rejected one', async () => {
    const store = usePollStore()
    vi.spyOn(raceApi, 'vote').mockResolvedValueOnce(open('a'))
    expect(await store.vote('a', 'Ana')).toBe(true)
    expect(store.poll?.voted).toBe('a')
    expect(store.poll?.results).toHaveLength(2)
  })

  it('sends the voter name trimmed', async () => {
    const store = usePollStore()
    const spy = vi.spyOn(raceApi, 'vote').mockResolvedValueOnce(open('a'))
    await store.vote('a', '  Ana  ')
    expect(spy).toHaveBeenCalledWith('a', 'Ana')
  })

  it('treats "already voted" as done and re-reads the poll', async () => {
    const store = usePollStore()
    vi.spyOn(raceApi, 'vote').mockRejectedValueOnce(new ApiError('http', 'conflict', 409))
    vi.spyOn(raceApi, 'getPoll').mockResolvedValueOnce(open('b'))
    expect(await store.vote('a', 'Ana')).toBe(true)
    expect(store.poll?.voted).toBe('b')
    expect(store.error).toBeNull()
  })

  it('keeps the visitor on the ballot with a message when the vote fails', async () => {
    const store = usePollStore()
    vi.spyOn(raceApi, 'vote').mockRejectedValueOnce(new ApiError('network', 'down'))
    expect(await store.vote('a', 'Ana')).toBe(false)
    expect(store.error).toBeTruthy()
    expect(store.busy).toBe(false)
  })
})
