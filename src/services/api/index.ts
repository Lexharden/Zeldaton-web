import { runtime } from '@/config/runtime'
import { getMockServer } from '../mock'
import { HttpRaceApi } from './HttpRaceApi'
import type { RaceApi } from './RaceApi'

/** Swap point: mock in VITE_MOCK_MODE, real REST otherwise. */
export const raceApi: RaceApi = runtime.mockMode ? getMockServer() : new HttpRaceApi()
export * from './RaceApi'
