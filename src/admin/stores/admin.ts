import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { adminApi } from '../api/AdminApi'
import type { AdminUser } from '../types'

/** Who is signed in to the panel. The session itself is an HttpOnly cookie the browser holds. */
export const useAdminStore = defineStore('admin', () => {
  const user = ref<AdminUser | null>(null)
  const via = ref<'session' | 'token' | null>(null)
  /** True once we asked the server who we are (so the router does not flash the login page). */
  const ready = ref(false)

  const isAdmin = computed(() => user.value?.role === 'admin')

  async function restore() {
    try {
      const me = await adminApi.me()
      user.value = me.user
      via.value = me.via
    } catch {
      user.value = null
      via.value = null
    } finally {
      ready.value = true
    }
  }

  async function login(username: string, password: string) {
    const out = await adminApi.login(username, password)
    user.value = out.user
    via.value = 'session'
    ready.value = true
  }

  async function logout() {
    try {
      await adminApi.logout()
    } catch {
      /* the cookie may already be gone */
    }
    clear()
  }

  function clear() {
    user.value = null
    via.value = null
  }

  return { user, via, ready, isAdmin, restore, login, logout, clear }
})
