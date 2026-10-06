import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { raceApi } from '@/services/api'
import type { DonorsBoard } from '@/types/donors'
import { formatSpan } from '@/utils/format'
import { useDonorsStore } from './donors'

const board = (enabled: boolean): DonorsBoard => ({
  enabled,
  totals: { donations: 1, addedSeconds: 60, removedSeconds: 0 },
  donors: enabled
    ? [
        {
          rank: 1,
          viewer: 'Fan',
          platform: 'twitch',
          currency: 'bits',
          donations: 1,
          amount: 50,
          addedSeconds: 60,
          removedSeconds: 0,
        },
      ]
    : [],
})

describe('donors store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.restoreAllMocks()
    vi.useRealTimers()
  })

  it('shows the section while loading, follows the organizer switch and hides on failure', async () => {
    const store = useDonorsStore()
    let resolve!: (b: DonorsBoard) => void
    vi.spyOn(raceApi, 'getDonors').mockReturnValueOnce(new Promise((r) => (resolve = r)))
    const pending = store.load()
    expect(store.visible).toBe(true) // first answer pending: skeleton, no flash of nothing
    resolve(board(true))
    await pending
    expect(store.visible).toBe(true)
    expect(store.donors).toHaveLength(1)

    vi.spyOn(raceApi, 'getDonors').mockResolvedValueOnce(board(false))
    await store.load()
    expect(store.visible).toBe(false)
  })

  it('stays hidden when the backend cannot be reached', async () => {
    const store = useDonorsStore()
    vi.spyOn(raceApi, 'getDonors').mockRejectedValueOnce(new Error('down'))
    await store.load()
    expect(store.visible).toBe(false)
    expect(store.error).toBeTruthy()
  })

  it('turns a burst of donations into one refresh', () => {
    vi.useFakeTimers()
    const store = useDonorsStore()
    const spy = vi.spyOn(raceApi, 'getDonors').mockResolvedValue(board(true))
    store.refreshSoon()
    store.refreshSoon()
    store.refreshSoon()
    expect(spy).not.toHaveBeenCalled()
    vi.advanceTimersByTime(3100)
    expect(spy).toHaveBeenCalledTimes(1)
  })
})

describe('formatSpan', () => {
  it('reads as a short span', () => {
    expect(formatSpan(0)).toBe('0s')
    expect(formatSpan(45)).toBe('45s')
    expect(formatSpan(720)).toBe('12m')
    expect(formatSpan(7800)).toBe('2h 10m')
    expect(formatSpan(7200)).toBe('2h')
  })
})
