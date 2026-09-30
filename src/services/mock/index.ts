import { MockRaceService } from './MockRaceService'

let instance: MockRaceService | null = null

/** One shared in-memory "backend" so REST and the mock socket stay consistent. */
export function getMockServer(): MockRaceService {
  instance ??= new MockRaceService()
  return instance
}
