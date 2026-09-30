import { describe, expect, it } from 'vitest'
import type { Racer } from '@/types/racer'
import { computeStandings } from './standings'

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

describe('standings', () => {
  it('ranks by objectives, then progress', () => {
    const list = [
      racer('a', { completedObjectives: ['1'], progressPercentage: 15 }),
      racer('b', { completedObjectives: ['1', '2'], progressPercentage: 25 }),
      racer('c', { completedObjectives: ['1'], progressPercentage: 19 }),
    ]
    expect(computeStandings(list).map((s) => s.racerId)).toEqual(['b', 'c', 'a'])
  })

  it('puts finished racers first, by final time', () => {
    const list = [
      racer('a', { progressPercentage: 90, completedObjectives: ['1', '2', '3'] }),
      racer('b', { status: 'finished', finalTimeSeconds: 200 }),
      racer('c', { status: 'finished', finalTimeSeconds: 100 }),
    ]
    expect(computeStandings(list).map((s) => s.racerId)).toEqual(['c', 'b', 'a'])
    expect(computeStandings(list)[0].rank).toBe(1)
  })
})
