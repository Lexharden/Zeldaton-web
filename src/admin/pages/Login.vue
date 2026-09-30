<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { LogIn, ShieldCheck } from 'lucide-vue-next'
import { AdminApiError } from '../api/AdminApi'
import { safeNext } from '../guard'
import { useAdminStore } from '../stores/admin'
import '../admin.css'

const admin = useAdminStore()
const route = useRoute()
const router = useRouter()

const username = ref('')
const password = ref('')
const busy = ref(false)
const error = ref('')
const expired = computed(() => route.query.expired === '1')

async function submit() {
  busy.value = true
  error.value = ''
  try {
    await admin.login(username.value, password.value)
    await router.replace(safeNext(route.query.next))
  } catch (e) {
    if (e instanceof AdminApiError && e.status === 429) {
      const wait = e.retryAfter ? Math.ceil(e.retryAfter / 60) : 15
      error.value = `Demasiados intentos. Vuelve a intentarlo en unos ${wait} min.`
    } else if (e instanceof AdminApiError && e.status === 401) {
      error.value = 'Usuario o contraseña incorrectos.'
    } else {
      error.value = e instanceof Error ? e.message : 'No se pudo iniciar sesión.'
    }
    password.value = ''
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <div class="flex min-h-screen items-center justify-center bg-background p-4">
    <form class="panel panel-gold w-full max-w-sm" @submit.prevent="submit">
      <div class="space-y-5 p-6">
        <div class="text-center">
          <ShieldCheck class="mx-auto size-9 text-accent" aria-hidden="true" />
          <h1 class="display mt-2 text-4xl text-white">ZELDATHON</h1>
          <p class="hud-label">PANEL DE ORGANIZACIÓN</p>
        </div>
        <p
          v-if="expired"
          class="rounded border border-warning/40 bg-warning/10 p-3 text-sm text-warning"
        >
          Tu sesión terminó. Inicia sesión de nuevo.
        </p>
        <div>
          <label class="a-label" for="u">Usuario</label>
          <input
            id="u"
            v-model="username"
            class="a-input"
            autocomplete="username"
            autofocus
            required
          />
        </div>
        <div>
          <label class="a-label" for="p">Contraseña</label>
          <input
            id="p"
            v-model="password"
            type="password"
            class="a-input"
            autocomplete="current-password"
            required
          />
        </div>
        <p v-if="error" class="a-error" role="alert">{{ error }}</p>
        <button
          type="submit"
          class="a-btn a-btn-gold w-full"
          :disabled="busy || !username || !password"
        >
          <LogIn class="size-4" />{{ busy ? 'Entrando…' : 'Entrar' }}
        </button>
      </div>
    </form>
  </div>
</template>
