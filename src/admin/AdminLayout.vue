<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { ART } from '@/config/artwork'
import { DEVELOPER } from '@/config/event'
import { RouterLink, RouterView, useRoute, useRouter } from 'vue-router'
import {
  Activity,
  BookOpen,
  CalendarClock,
  CalendarDays,
  ExternalLink,
  Gift,
  KeyRound,
  LayoutDashboard,
  Radar,
  LogOut,
  ScrollText,
  Users,
} from 'lucide-vue-next'
import { adminApi } from './api/AdminApi'
import ConfirmHost from './components/ConfirmHost.vue'
import Modal from './components/Modal.vue'
import ToastHost from './components/ToastHost.vue'
import { messageOf, useToasts } from './composables/useToasts'
import { useAdminStore } from './stores/admin'
import { useMonitorStore } from './stores/monitor'
import './admin.css'

/** Chrome of the organizer panel: sidebar (what the role may see), user menu and global hosts. */
const admin = useAdminStore()
const route = useRoute()
const router = useRouter()
const toasts = useToasts()
const monitor = useMonitorStore()

// Open incidents are counted in the menu from any page of the panel.
const baseTitle = ref('')
onMounted(() => {
  baseTitle.value = document.title.replace(/^\(\d+\) /, '')
  monitor.start()
})
onBeforeUnmount(() => {
  monitor.stop()
  if (baseTitle.value) document.title = baseTitle.value
})
// ...and in the tab title, so a referee with the tab in the background sees it.
watch(
  () => monitor.openCount,
  (n) => {
    if (!baseTitle.value) return
    document.title = n ? `(${n}) ${baseTitle.value}` : baseTitle.value
  },
)

const nav = computed(() =>
  [
    { name: 'admin-dashboard', label: 'Panel', icon: LayoutDashboard, role: 'moderator' },
    { name: 'admin-monitor', label: 'Monitor', icon: Radar, role: 'moderator' },
    { name: 'admin-schedule', label: 'Agenda', icon: CalendarDays, role: 'moderator' },
    { name: 'admin-event', label: 'Evento', icon: CalendarClock, role: 'admin' },
    { name: 'admin-racers', label: 'Corredores', icon: Activity, role: 'moderator' },
    { name: 'admin-catalog', label: 'Catálogo', icon: BookOpen, role: 'admin' },
    { name: 'admin-donations', label: 'Donaciones', icon: Gift, role: 'moderator' },
    { name: 'admin-audit', label: 'Auditoría', icon: ScrollText, role: 'moderator' },
    { name: 'admin-accounts', label: 'Cuentas', icon: Users, role: 'admin' },
  ].filter((n) => n.role === 'moderator' || admin.isAdmin),
)

async function signOut() {
  await admin.logout()
  await router.replace({ name: 'admin-login' })
}

// Own password.
const pwOpen = ref(false)
const pw = ref({ current: '', next: '', again: '' })
const pwBusy = ref(false)
const pwError = computed(() =>
  pw.value.next && pw.value.next.length < 12
    ? 'Mínimo 12 caracteres.'
    : pw.value.again && pw.value.next !== pw.value.again
      ? 'Las contraseñas no coinciden.'
      : '',
)
async function changePassword() {
  pwBusy.value = true
  try {
    await adminApi.changeOwnPassword(pw.value.current, pw.value.next)
    toasts.success('Contraseña cambiada. Se cerraron tus otras sesiones.')
    pwOpen.value = false
    pw.value = { current: '', next: '', again: '' }
  } catch (e) {
    toasts.error(messageOf(e))
  } finally {
    pwBusy.value = false
  }
}
</script>

<template>
  <div class="min-h-screen bg-background text-text lg:grid lg:grid-cols-[15rem_1fr]">
    <aside
      class="border-b border-line bg-surface lg:sticky lg:top-0 lg:h-screen lg:border-b-0 lg:border-r"
    >
      <div class="flex items-center justify-between gap-3 p-4 lg:block">
        <RouterLink :to="{ name: 'admin-dashboard' }" class="flex items-center gap-2">
          <img
            :src="ART.zeldatonLogoSmall"
            alt="Zeldatón"
            width="480"
            height="149"
            class="h-10 w-auto"
            draggable="false"
          />
        </RouterLink>
        <p class="hud-label hidden text-[10px] lg:mt-1 lg:block">PANEL DE ORGANIZACIÓN</p>
      </div>
      <nav
        class="flex gap-1 overflow-x-auto px-2 pb-2 lg:block lg:space-y-1 lg:px-3"
        aria-label="Panel"
      >
        <RouterLink
          v-for="n in nav"
          :key="n.name"
          :to="{ name: n.name }"
          class="flex items-center gap-3 whitespace-nowrap rounded px-3 py-2 text-sm text-muted hover:bg-white/5 hover:text-white"
          active-class="!bg-primary/15 !text-white"
          :exact-active-class="n.name === 'admin-dashboard' ? '!bg-primary/15 !text-white' : ''"
        >
          <component :is="n.icon" class="size-4" aria-hidden="true" />{{ n.label }}
          <span
            v-if="n.name === 'admin-monitor' && monitor.openCount"
            class="num ml-auto rounded-full bg-danger/80 px-1.5 text-[11px] font-bold text-white"
            :aria-label="`${monitor.openCount} incidentes abiertos`"
            >{{ monitor.openCount }}</span
          >
        </RouterLink>
      </nav>
      <div
        class="hidden border-t border-line p-4 lg:absolute lg:bottom-0 lg:left-0 lg:right-0 lg:block"
      >
        <p class="truncate text-sm text-white">{{ admin.user?.username }}</p>
        <p class="hud-label mb-3 text-[10px]">
          {{ admin.user?.role === 'admin' ? 'ADMINISTRADOR' : 'MODERADOR' }}
          <span v-if="admin.via === 'token'"> · TOKEN</span>
        </p>
        <div class="flex flex-wrap gap-2">
          <button
            v-if="admin.via === 'session'"
            type="button"
            class="a-btn a-btn-sm"
            @click="pwOpen = true"
          >
            <KeyRound class="size-3.5" />Contraseña
          </button>
          <button type="button" class="a-btn a-btn-sm" @click="signOut">
            <LogOut class="size-3.5" />Salir
          </button>
        </div>
        <RouterLink
          to="/"
          class="mt-3 inline-flex items-center gap-1.5 text-xs text-muted hover:text-white"
          ><ExternalLink class="size-3" />Ver el sitio público</RouterLink
        >
        <p class="mt-3 text-[11px] text-muted">
          Desarrollado por {{ DEVELOPER.name }} · Powered by HiveShock
        </p>
      </div>
    </aside>

    <main class="min-w-0 p-4 sm:p-6 lg:p-8">
      <RouterView :key="route.name?.toString()" />
      <!-- small screens: the user box lives at the bottom -->
      <div
        class="mt-10 flex flex-wrap items-center gap-3 border-t border-line pt-4 text-sm lg:hidden"
      >
        <span>{{ admin.user?.username }}</span>
        <button type="button" class="a-btn a-btn-sm" @click="signOut">Salir</button>
      </div>
    </main>

    <ConfirmHost />
    <ToastHost />

    <Modal :open="pwOpen" title="Cambiar mi contraseña" @close="pwOpen = false">
      <form class="space-y-4" @submit.prevent="changePassword">
        <div>
          <label class="a-label" for="pw-current">Contraseña actual</label>
          <input
            id="pw-current"
            v-model="pw.current"
            type="password"
            class="a-input"
            autocomplete="current-password"
            required
          />
        </div>
        <div>
          <label class="a-label" for="pw-new">Nueva contraseña</label>
          <input
            id="pw-new"
            v-model="pw.next"
            type="password"
            class="a-input"
            autocomplete="new-password"
            required
            minlength="12"
          />
          <p class="a-hint">Al menos 12 caracteres. Se cerrarán tus otras sesiones.</p>
        </div>
        <div>
          <label class="a-label" for="pw-again">Repite la nueva</label>
          <input
            id="pw-again"
            v-model="pw.again"
            type="password"
            class="a-input"
            autocomplete="new-password"
            required
          />
          <p v-if="pwError" class="a-error">{{ pwError }}</p>
        </div>
        <div class="flex justify-end gap-3">
          <button type="button" class="a-btn" @click="pwOpen = false">Cancelar</button>
          <button
            type="submit"
            class="a-btn a-btn-primary"
            :disabled="pwBusy || !!pwError || !pw.next || !pw.current"
          >
            Guardar
          </button>
        </div>
      </form>
    </Modal>
  </div>
</template>
