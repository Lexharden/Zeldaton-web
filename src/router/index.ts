import { createRouter, createWebHistory } from 'vue-router'

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
      path: '/hiveshock',
      name: 'hiveshock',
      component: () => import('@/pages/AboutHiveShock.vue'),
      meta: { eventHeader: false },
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
