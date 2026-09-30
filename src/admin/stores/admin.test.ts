import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { adminApi, AdminApiError } from '../api/AdminApi'
import { useAdminStore } from './admin'
import { useOverviewStore } from './overview'

const ana = { id: 1, username: 'ana', role: 'admin' as const }

describe('admin store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.restoreAllMocks()
  })

  it('restore() adopts an existing session and marks the store ready', async () => {
    vi.spyOn(adminApi, 'me').mockResolvedValue({ user: ana, via: 'session', csrfToken: 'c' })
    const admin = useAdminStore()
    expect(admin.ready).toBe(false)
    await admin.restore()
    expect(admin.ready).toBe(true)
    expect(admin.user?.username).toBe('ana')
    expect(admin.isAdmin).toBe(true)
  })

  it('restore() without a session leaves nobody signed in (and still ready)', async () => {
    vi.spyOn(adminApi, 'me').mockRejectedValue(
      new AdminApiError(401, 'unauthorized', 'unauthorized'),
    )
    const admin = useAdminStore()
    await admin.restore()
    expect(admin.user).toBeNull()
    expect(admin.ready).toBe(true)
  })

  it('login and logout', async () => {
    vi.spyOn(adminApi, 'login').mockResolvedValue({
      user: { ...ana, role: 'moderator' },
      csrfToken: 'c',
      expiresAtUtc: 'x',
    })
    const logout = vi.spyOn(adminApi, 'logout').mockResolvedValue()
    const admin = useAdminStore()
    await admin.login('ana', 'pw')
    expect(admin.user?.role).toBe('moderator')
    expect(admin.isAdmin).toBe(false)
    await admin.logout()
    expect(logout).toHaveBeenCalled()
    expect(admin.user).toBeNull()
  })

  it('a failed login does not sign anybody in', async () => {
    vi.spyOn(adminApi, 'login').mockRejectedValue(
      new AdminApiError(401, 'unauthorized', 'unauthorized'),
    )
    const admin = useAdminStore()
    await expect(admin.login('ana', 'bad')).rejects.toBeInstanceOf(AdminApiError)
    expect(admin.user).toBeNull()
  })

  it('logout clears the user even when the server call fails', async () => {
    vi.spyOn(adminApi, 'me').mockResolvedValue({ user: ana, via: 'session', csrfToken: 'c' })
    vi.spyOn(adminApi, 'logout').mockRejectedValue(new AdminApiError(0, 'network', 'down'))
    const admin = useAdminStore()
    await admin.restore()
    await admin.logout()
    expect(admin.user).toBeNull()
  })
})

describe('overview store', () => {
  beforeEach(() => setActivePinia(createPinia()))

  it('counts a live clock down between polls and freezes the others', () => {
    const overview = useOverviewStore()
    overview.receivedAt = 1_000_000
    expect(overview.remainingNow('live', 100, 1_003_000)).toBe(97)
    expect(overview.remainingNow('live', 2, 1_010_000)).toBe(0)
    expect(overview.remainingNow('paused', 100, 1_003_000)).toBe(100)
    expect(overview.remainingNow('offline', 100, 1_003_000)).toBe(100)
  })
})
