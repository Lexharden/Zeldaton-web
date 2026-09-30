import { describe, expect, it } from 'vitest'
import { adminGuard, safeNext } from './guard'
import type { AdminUser } from './types'

const admin: AdminUser = { id: 1, username: 'ana', role: 'admin' }
const mod: AdminUser = { id: 2, username: 'bob', role: 'moderator' }
const to = (path: string, name?: string, meta: Record<string, unknown> = {}) => ({
  name,
  path,
  fullPath: path,
  meta,
})

describe('adminGuard', () => {
  it('ignores the public site', () => {
    expect(adminGuard(to('/race'), null)).toBe(true)
  })

  it('sends anonymous visitors to the login page remembering where they were going', () => {
    expect(adminGuard(to('/admin/racers', 'admin-racers'), null)).toEqual({
      name: 'admin-login',
      query: { next: '/admin/racers' },
    })
  })

  it('lets the login page through for anonymous and bounces signed-in users', () => {
    expect(adminGuard(to('/admin/login', 'admin-login'), null)).toBe(true)
    expect(adminGuard(to('/admin/login', 'admin-login'), admin)).toEqual({
      name: 'admin-dashboard',
    })
  })

  it('keeps moderators out of admin-only pages', () => {
    const page = to('/admin/accounts', 'admin-accounts', { role: 'admin' })
    expect(adminGuard(page, mod)).toEqual({ name: 'admin-dashboard' })
    expect(adminGuard(page, admin)).toBe(true)
    expect(adminGuard(to('/admin/racers', 'admin-racers'), mod)).toBe(true)
  })
})

describe('safeNext', () => {
  it('only follows destinations inside the panel', () => {
    expect(safeNext('/admin/racers?x=1')).toBe('/admin/racers?x=1')
    expect(safeNext('/admin')).toBe('/admin')
    for (const bad of [
      'https://evil.example',
      '//evil.example',
      '/race',
      '/administrator',
      undefined,
      42,
      null,
    ])
      expect(safeNext(bad)).toBe('/admin')
  })
})
