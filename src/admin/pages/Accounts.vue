<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { KeyRound, Plus, Trash2, Wand2 } from 'lucide-vue-next'
import { adminApi } from '../api/AdminApi'
import { localDateTime } from '../format'
import { confirm } from '../composables/useConfirm'
import { messageOf, useToasts } from '../composables/useToasts'
import { useAdminStore } from '../stores/admin'
import type { UserRow } from '../types'
import Modal from '../components/Modal.vue'
import PageHeader from '../components/PageHeader.vue'
import SecretDialog from '../components/SecretDialog.vue'

const admin = useAdminStore()
const toasts = useToasts()
const users = ref<UserRow[]>([])

async function load() {
  try {
    users.value = await adminApi.listUsers()
  } catch (e) {
    toasts.error(messageOf(e))
  }
}
onMounted(load)

/** A random passphrase the organizer can hand over (the owner changes it on first login). */
function randomPassword(): string {
  const chars = 'abcdefghijkmnpqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789'
  const bytes = crypto.getRandomValues(new Uint32Array(18))
  return Array.from(bytes, (b) => chars[b % chars.length]).join('')
}

// ---- create
const create = ref({
  open: false,
  username: '',
  password: '',
  role: 'moderator' as 'admin' | 'moderator',
})
const createOk = computed(
  () => /^[a-z0-9._-]{3,32}$/i.test(create.value.username) && create.value.password.length >= 12,
)
const secret = ref({ open: false, title: '', value: '' })

async function submitCreate() {
  try {
    const { username, password, role } = create.value
    await adminApi.createUser({ username, password, role })
    toasts.success('Cuenta creada.')
    secret.value = { open: true, title: `Contraseña de ${username.toLowerCase()}`, value: password }
    create.value.open = false
    void load()
  } catch (e) {
    toasts.error(messageOf(e))
  }
}

// ---- reset password
const reset = ref<{ user: UserRow | null; password: string }>({ user: null, password: '' })
async function submitReset() {
  if (!reset.value.user) return
  try {
    await adminApi.resetPassword(reset.value.user.id, reset.value.password)
    toasts.success('Contraseña restablecida. Se cerraron sus sesiones.')
    secret.value = {
      open: true,
      title: `Contraseña de ${reset.value.user.username}`,
      value: reset.value.password,
    }
    reset.value = { user: null, password: '' }
  } catch (e) {
    toasts.error(messageOf(e))
  }
}

async function patch(u: UserRow, change: { role?: 'admin' | 'moderator'; disabled?: boolean }) {
  try {
    await adminApi.updateUser(u.id, change)
    toasts.success('Cuenta actualizada.')
  } catch (e) {
    toasts.error(messageOf(e))
  }
  void load()
}

async function remove(u: UserRow) {
  const ok = await confirm({
    title: 'Eliminar cuenta',
    message: `Se elimina la cuenta de ${u.username} y sus sesiones.`,
    confirmLabel: 'Eliminar',
    danger: true,
    requireText: u.username,
  })
  if (!ok) return
  try {
    await adminApi.deleteUser(u.id)
    toasts.success('Cuenta eliminada.')
  } catch (e) {
    toasts.error(messageOf(e))
  }
  void load()
}

const mine = (u: UserRow) => admin.user?.id === u.id
</script>

<template>
  <div>
    <PageHeader
      title="Cuentas"
      subtitle="Quién puede entrar al panel. Los moderadores llevan el día de carrera; los administradores lo configuran todo."
    >
      <button
        type="button"
        class="a-btn a-btn-primary"
        @click="
          create = { open: true, username: '', password: randomPassword(), role: 'moderator' }
        "
      >
        <Plus class="size-4" />Nueva cuenta
      </button>
    </PageHeader>

    <section class="panel">
      <div class="overflow-x-auto p-2">
        <table class="a-table">
          <thead>
            <tr>
              <th>Usuario</th>
              <th>Rol</th>
              <th>Estado</th>
              <th>Último acceso</th>
              <th />
            </tr>
          </thead>
          <tbody>
            <tr v-for="u in users" :key="u.id">
              <td class="font-semibold text-white">
                {{ u.username }}
                <span v-if="mine(u)" class="text-xs font-normal text-muted">(tú)</span>
              </td>
              <td>
                <select
                  class="a-select w-40"
                  :value="u.role"
                  :disabled="mine(u)"
                  :aria-label="`Rol de ${u.username}`"
                  @change="
                    patch(u, {
                      role: ($event.target as HTMLSelectElement).value as 'admin' | 'moderator',
                    })
                  "
                >
                  <option value="moderator">Moderador</option>
                  <option value="admin">Administrador</option>
                </select>
              </td>
              <td>
                <span class="a-dot" :class="!u.disabled && 'a-dot-on'" />
                <span class="ml-2 text-xs text-muted">{{
                  u.disabled ? 'desactivada' : 'activa'
                }}</span>
              </td>
              <td class="text-xs text-muted">{{ localDateTime(u.lastLoginAt) }}</td>
              <td>
                <div class="flex flex-wrap justify-end gap-2">
                  <button
                    type="button"
                    class="a-btn a-btn-sm"
                    @click="reset = { user: u, password: randomPassword() }"
                  >
                    <KeyRound class="size-3.5" />Contraseña
                  </button>
                  <button
                    type="button"
                    class="a-btn a-btn-sm"
                    :disabled="mine(u)"
                    @click="patch(u, { disabled: !u.disabled })"
                  >
                    {{ u.disabled ? 'Activar' : 'Desactivar' }}
                  </button>
                  <button
                    type="button"
                    class="a-btn a-btn-sm a-btn-danger"
                    :disabled="mine(u)"
                    aria-label="Eliminar"
                    @click="remove(u)"
                  >
                    <Trash2 class="size-3.5" />
                  </button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>

    <Modal :open="create.open" title="Nueva cuenta" @close="create.open = false">
      <form class="space-y-4" @submit.prevent="submitCreate">
        <div>
          <label class="a-label" for="ac-user">Usuario</label>
          <input id="ac-user" v-model="create.username" class="a-input" autocomplete="off" />
          <p class="a-hint">3-32 caracteres: letras, números, punto, guion o guion bajo.</p>
        </div>
        <div>
          <label class="a-label" for="ac-role">Rol</label>
          <select id="ac-role" v-model="create.role" class="a-select">
            <option value="moderator">Moderador (día de carrera)</option>
            <option value="admin">Administrador (todo)</option>
          </select>
        </div>
        <div>
          <label class="a-label" for="ac-pass">Contraseña inicial</label>
          <div class="flex gap-2">
            <input
              id="ac-pass"
              v-model="create.password"
              class="a-input font-mono"
              autocomplete="off"
            />
            <button
              type="button"
              class="a-btn"
              aria-label="Generar"
              @click="create.password = randomPassword()"
            >
              <Wand2 class="size-4" />
            </button>
          </div>
          <p class="a-hint">
            Mínimo 12 caracteres. Después de crearla se muestra una vez para que se la des.
          </p>
        </div>
        <div class="flex justify-end gap-3">
          <button type="button" class="a-btn" @click="create.open = false">Cancelar</button>
          <button type="submit" class="a-btn a-btn-primary" :disabled="!createOk">
            Crear cuenta
          </button>
        </div>
      </form>
    </Modal>

    <Modal
      :open="!!reset.user"
      :title="`Contraseña de ${reset.user?.username ?? ''}`"
      @close="reset = { user: null, password: '' }"
    >
      <form class="space-y-4" @submit.prevent="submitReset">
        <div>
          <label class="a-label" for="rp-pass">Nueva contraseña</label>
          <div class="flex gap-2">
            <input
              id="rp-pass"
              v-model="reset.password"
              class="a-input font-mono"
              autocomplete="off"
            />
            <button
              type="button"
              class="a-btn"
              aria-label="Generar"
              @click="reset.password = randomPassword()"
            >
              <Wand2 class="size-4" />
            </button>
          </div>
          <p class="a-hint">Se cierran todas sus sesiones abiertas.</p>
        </div>
        <div class="flex justify-end gap-3">
          <button type="button" class="a-btn" @click="reset = { user: null, password: '' }">
            Cancelar
          </button>
          <button type="submit" class="a-btn a-btn-primary" :disabled="reset.password.length < 12">
            Restablecer
          </button>
        </div>
      </form>
    </Modal>

    <SecretDialog
      :open="secret.open"
      :title="secret.title"
      :secret="secret.value"
      note="Pídele que la cambie en su primer acceso (Panel → Contraseña)."
      @close="secret.open = false"
    />
  </div>
</template>
