import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'
import type { Racer } from '@/types/racer'
import { compareRacers, computeStandings, progressTenths } from './standings'

const racer = (id: string, over: Partial<Racer>): Racer => ({
  id,
  slug: id,
  displayName: id,
  timezone: 'UTC',
  channels: [],
  status: 'live',
  elapsedSeconds: 100,
  remainingSeconds: 100,
  progressPercentage: 0,
  completedObjectives: [],
  ...over,
})

const ids = (list: Racer[], required?: string[]) =>
  computeStandings(list, required).map((s) => s.racerId)

describe('standings', () => {
  it('ranks by objectives, then progress', () => {
    const list = [
      racer('a', { completedObjectives: ['1'], progressPercentage: 15 }),
      racer('b', { completedObjectives: ['1', '2'], progressPercentage: 25 }),
      racer('c', { completedObjectives: ['1'], progressPercentage: 19 }),
    ]
    expect(ids(list)).toEqual(['b', 'c', 'a'])
  })

  it('puts finished racers first, by final time when they crossed together', () => {
    const list = [
      racer('a', { progressPercentage: 90, completedObjectives: ['1', '2', '3'] }),
      racer('b', { status: 'finished', finalTimeSeconds: 200 }),
      racer('c', { status: 'finished', finalTimeSeconds: 100 }),
    ]
    expect(ids(list)).toEqual(['c', 'b', 'a'])
    expect(computeStandings(list)[0].rank).toBe(1)
  })

  it('only the required objectives count once the event says which they are', () => {
    const list = [
      racer('extras', { completedObjectives: ['x', 'y', 'z'] }),
      racer('required', { completedObjectives: ['r1'] }),
    ]
    expect(ids(list, ['r1'])).toEqual(['required', 'extras'])
    // Before the event loads, every objective counts.
    expect(ids(list)).toEqual(['extras', 'required'])
  })

  it('progress is compared in tenths, so noise below that never reorders', () => {
    expect(progressTenths(41.64)).toBe(416)
    expect(progressTenths(41.65)).toBe(417)
    expect(progressTenths(Number.NaN)).toBe(0)
    expect(progressTenths(250)).toBe(1000)
  })

  it('does not depend on the order the racers arrive in', () => {
    const list = ['d', 'a', 'c', 'b', 'e'].map((id, i) =>
      racer(id, { progressPercentage: (i % 3) * 10 }),
    )
    const expected = ids(list)
    for (let shift = 0; shift < list.length; shift++) {
      const rotated = [...list.slice(shift), ...list.slice(0, shift)]
      expect(ids(rotated)).toEqual(expected)
      expect(ids([...rotated].reverse())).toEqual(expected)
    }
  })

  it('is a total order: antisymmetric and transitive over a grid of values', () => {
    const pool: Racer[] = [
      racer('r0', {}),
      racer('r1', {
        completedObjectives: ['o0'],
        progressPercentage: 10,
        items: { a: true },
        milestoneAtUtc: '2026-10-07T13:00:00Z',
        playedSeconds: 100,
      }),
      racer('r2', {
        completedObjectives: ['o0'],
        progressPercentage: 10.04,
        items: { a: true },
        milestoneAtUtc: '2026-10-07T12:00:00Z',
        playedSeconds: 100,
      }),
      racer('r3', {
        completedObjectives: ['o0'],
        progressPercentage: 10,
        items: { a: true, b: true },
        playedSeconds: 50,
      }),
      racer('r4', { completedObjectives: ['o0', 'o1'], progressPercentage: 5, playedSeconds: 10 }),
      racer('r5', {
        status: 'finished',
        finishedAtUtc: '2026-10-08T10:00:00Z',
        finalTimeSeconds: 9,
      }),
    ]
    const req = ['o0', 'o1', 'o2']
    for (const a of pool)
      for (const b of pool) {
        expect(Math.sign(compareRacers(a, b, req))).toBe(-Math.sign(compareRacers(b, a, req)) || 0)
        for (const c of pool)
          if (compareRacers(a, b, req) <= 0 && compareRacers(b, c, req) <= 0)
            expect(compareRacers(a, c, req)).toBeLessThanOrEqual(0)
      }
  })
})

/** The cases the backend's test reads too: both implementations must agree on every one. */
describe('shared golden cases (contract/standings-cases.json)', () => {
  interface Fixture {
    id: string
    status?: Racer['status']
    objectives?: string[]
    progress?: number
    items?: number
    milestoneAt?: string
    played?: number
    finishedAt?: string
    finalTime?: number
    elapsed?: number
  }
  const doc = JSON.parse(
    readFileSync(
      fileURLToPath(new URL('../../contract/standings-cases.json', import.meta.url)),
      'utf8',
    ),
  ) as { cases: { name: string; required: string[]; racers: Fixture[]; order: string[] }[] }

  it.each(doc.cases)('$name', (c) => {
    const racers = c.racers.map((f) =>
      racer(f.id, {
        status: f.status ?? 'live',
        completedObjectives: f.objectives ?? [],
        progressPercentage: f.progress ?? 0,
        items: Object.fromEntries([...Array(f.items ?? 0).keys()].map((i) => [`item${i}`, true])),
        milestoneAtUtc: f.milestoneAt,
        playedSeconds: f.played ?? 0,
        finishedAtUtc: f.finishedAt,
        finalTimeSeconds: f.finalTime,
        elapsedSeconds: f.elapsed ?? 100,
      }),
    )
    expect(ids(racers, c.required)).toEqual(c.order)
    expect(ids([...racers].reverse(), c.required)).toEqual(c.order)
  })
})
