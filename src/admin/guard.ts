import type { RouteLocationRaw } from 'vue-router'
import type { AdminUser } from './types'

interface Target {
  name?: unknown
  path: string
  fullPath: string
  meta: Record<string, unknown>
}

/**
 * Route guard for `/admin/*`, kept pure so it is easy to test. Returns `true` to continue or a
 * redirect: anonymous visitors go to the login page (remembering where they wanted to go), the
 * login page bounces signed-in users to the dashboard, and admin-only pages bounce moderators.
 */
export function adminGuard(to: Target, user: AdminUser | null): true | RouteLocationRaw {
  if (!to.path.startsWith('/admin')) return true
  if (to.name === 'admin-login') return user ? { name: 'admin-dashboard' } : true
  if (!user) return { name: 'admin-login', query: { next: to.fullPath } }
  if (to.meta.role === 'admin' && user.role !== 'admin') return { name: 'admin-dashboard' }
  return true
}

/** Only follows `next` if it stays inside the panel (no open redirects). */
export function safeNext(next: unknown): string {
  return typeof next === 'string' && /^\/admin(\/|$|\?)/.test(next) ? next : '/admin'
}
