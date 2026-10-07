import { createRouter, createWebHistory } from 'vue-router'
import { adminApi } from '@/admin/api/AdminApi'
import { adminGuard } from '@/admin/guard'
import { useAdminStore } from '@/admin/stores/admin'

/** Route-level code splitting: each page is its own chunk. */
export const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      name: 'home',
      component: () => import('@/pages/Home.vue'),
      meta: { eventHeader: false },
    },
    { path: '/race', name: 'race', component: () => import('@/pages/Race.vue') },
    { path: '/racer/:id', name: 'racer', component: () => import('@/pages/Racer.vue') },
    { path: '/streams', name: 'streams', component: () => import('@/pages/Streams.vue') },
    { path: '/rules', name: 'rules', component: () => import('@/pages/Rules.vue') },
    {
      path: '/privacy',
      name: 'privacy',
      component: () => import('@/pages/Legal.vue'),
      meta: { eventHeader: false, doc: 'privacy' },
    },
    {
      path: '/terms',
      name: 'terms',
      component: () => import('@/pages/Legal.vue'),
      meta: { eventHeader: false, doc: 'terms' },
    },
    {
      path: '/hiveshock',
      name: 'hiveshock',
      component: () => import('@/pages/AboutHiveShock.vue'),
      meta: { eventHeader: false },
    },
    // ---- organizer panel: its own chrome (`bare`), behind a login ------------------------------
    {
      path: '/admin/login',
      name: 'admin-login',
      component: () => import('@/admin/pages/Login.vue'),
      meta: { bare: true },
    },
    {
      path: '/admin',
      component: () => import('@/admin/AdminLayout.vue'),
      meta: { bare: true },
      children: [
        {
          path: '',
          name: 'admin-dashboard',
          component: () => import('@/admin/pages/Dashboard.vue'),
        },
        {
          path: 'monitor',
          name: 'admin-monitor',
          component: () => import('@/admin/pages/Monitor.vue'),
        },
        {
          path: 'schedule',
          name: 'admin-schedule',
          component: () => import('@/admin/pages/Schedule.vue'),
        },
        {
          path: 'event',
          name: 'admin-event',
          component: () => import('@/admin/pages/EventPage.vue'),
          meta: { role: 'admin' },
        },
        {
          path: 'racers',
          name: 'admin-racers',
          component: () => import('@/admin/pages/Racers.vue'),
        },
        {
          path: 'catalog',
          name: 'admin-catalog',
          component: () => import('@/admin/pages/Catalog.vue'),
          meta: { role: 'admin' },
        },
        { path: 'audit', name: 'admin-audit', component: () => import('@/admin/pages/Audit.vue') },
        {
          path: 'donations',
          name: 'admin-donations',
          component: () => import('@/admin/pages/Donations.vue'),
        },
        {
          path: 'accounts',
          name: 'admin-accounts',
          component: () => import('@/admin/pages/Accounts.vue'),
          meta: { role: 'admin' },
        },
      ],
    },
    {
      path: '/:pathMatch(.*)*',
      name: 'not-found',
      component: () => import('@/pages/NotFound.vue'),
      meta: { eventHeader: false },
    },
  ],
  scrollBehavior(to, from, saved) {
    if (saved) return saved
    if (to.hash) return { el: to.hash, top: 72, behavior: 'smooth' }
    if (to.path === from.path) return false
    return { top: 0 }
  },
})

// Panel access: ask the server who we are once, then apply the pure guard.
router.beforeEach(async (to) => {
  if (!to.path.startsWith('/admin')) return true
  const admin = useAdminStore()
  if (!admin.ready) await admin.restore()
  return adminGuard(to, admin.user)
})

// A 401 anywhere in the panel (session expired or revoked) sends the user back to the login page.
adminApi.onUnauthorized = () => {
  useAdminStore().clear()
  const current = router.currentRoute.value
  if (current.path.startsWith('/admin') && current.name !== 'admin-login') {
    void router.push({ name: 'admin-login', query: { next: current.fullPath, expired: '1' } })
  }
}
