<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { KeyRound, Pencil, Plus, Settings2, Trash2 } from 'lucide-vue-next'
import LiveBadge from '@/components/common/LiveBadge.vue'
import PlatformIcon from '@/components/racers/PlatformIcon.vue'
import { adminApi } from '../api/AdminApi'
import { confirm } from '../composables/useConfirm'
import { messageOf, useToasts } from '../composables/useToasts'
import { useAdminStore } from '../stores/admin'
import type { OverviewRacer } from '../types'
import PageHeader from '../components/PageHeader.vue'
import RacerControlDialog from '../components/RacerControlDialog.vue'
import RacerFormDialog from '../components/RacerFormDialog.vue'
import SecretDialog from '../components/SecretDialog.vue'

const admin = useAdminStore()
const toasts = useToasts()
const rows = ref<OverviewRacer[]>([])
const loading = ref(true)

async function load() {
  try {
    rows.value = await adminApi.listRacers()
  } catch (e) {
    toasts.error(messageOf(e))
  } finally {
    loading.value = false
  }
}
let timer: ReturnType<typeof setInterval> | null = null
onMounted(() => {
  void load()
  timer = setInterval(() => !document.hidden && void load(), 5000)
})
onBeforeUnmount(() => timer && clearInterval(timer))

const form = ref<{ open: boolean; racer: OverviewRacer['racer'] | null }>({
  open: false,
  racer: null,
})
const control = ref<OverviewRacer | null>(null)
const secret = ref<{ open: boolean; title: string; value: string }>({
  open: false,
  title: '',
  value: '',
})

function showToken(title: string, token: string) {
  secret.value = { open: true, title, value: token }
}

async function rotate(row: OverviewRacer) {
  const ok = await confirm({
    title: 'Rotar el token',
    message: `El token actual de ${row.racer.displayName} deja de funcionar y su HiveShock se desconecta. Tendrás que darle el nuevo.`,
    confirmLabel: 'Rotar token',
    danger: true,
  })
  if (!ok) return
  try {
    const out = await adminApi.rotateToken(row.racer.id)
    showToken(`Token nuevo de ${row.racer.displayName}`, out.token)
    void load()
  } catch (e) {
    toasts.error(messageOf(e))
  }
}

async function remove(row: OverviewRacer) {
  const ok = await confirm({
    title: 'Eliminar corredor',
    message: `Se borra a ${row.racer.displayName} con todo su progreso. No se puede deshacer.`,
    confirmLabel: 'Eliminar',
    danger: true,
    requireText: row.racer.id,
  })
  if (!ok) return
  try {
    await adminApi.deleteRacer(row.racer.id)
    toasts.success('Corredor eliminado.')
    void load()
  } catch (e) {
    toasts.error(messageOf(e))
  }
}

function saved(token?: string) {
  void load()
  if (token) showToken('Token del nuevo corredor', token)
}
</script>

<template>
  <div>
    <PageHeader
      title="Corredores"
      subtitle="Altas, tokens de HiveShock y control de cada corredor."
    >
      <button
        v-if="admin.isAdmin"
        type="button"
        class="a-btn a-btn-primary"
        @click="form = { open: true, racer: null }"
      >
        <Plus class="size-4" />Nuevo corredor
      </button>
    </PageHeader>

    <section class="panel">
      <div class="overflow-x-auto p-2">
        <table class="a-table">
          <thead>
            <tr>
              <th>Corredor</th>
              <th>Estado</th>
              <th>Señal</th>
              <th>Zona horaria</th>
              <th>Canales</th>
              <th />
            </tr>
          </thead>
          <tbody>
            <tr v-for="row in rows" :key="row.racer.id">
              <td>
                <div class="font-semibold text-white">{{ row.racer.displayName }}</div>
                <div class="text-xs text-muted">{{ row.racer.id }}</div>
              </td>
              <td><LiveBadge :status="row.racer.status" size="sm" /></td>
              <td>
                <span class="a-dot" :class="row.connected && 'a-dot-on'" />
                <span class="ml-2 text-xs text-muted">{{
                  row.connected
                    ? 'HiveShock conectado'
                    : row.lastHeartbeatUtc
                      ? 'desconectado'
                      : 'nunca conectó'
                }}</span>
              </td>
              <td class="text-xs">{{ row.racer.timezone }}</td>
              <td>
                <span
                  v-for="c in row.racer.channels"
                  :key="c.platform"
                  class="mr-3 inline-flex items-center gap-1 text-xs text-muted"
                >
                  <PlatformIcon :platform="c.platform" />@{{ c.handle }}
                </span>
                <span v-if="!row.racer.channels.length" class="text-xs text-muted">—</span>
              </td>
              <td>
                <div class="flex flex-wrap justify-end gap-2">
                  <button type="button" class="a-btn a-btn-sm" @click="control = row">
                    <Settings2 class="size-3.5" />Controlar
                  </button>
                  <template v-if="admin.isAdmin">
                    <button
                      type="button"
                      class="a-btn a-btn-sm"
                      @click="form = { open: true, racer: row.racer }"
                    >
                      <Pencil class="size-3.5" />Editar
                    </button>
                    <button type="button" class="a-btn a-btn-sm" @click="rotate(row)">
                      <KeyRound class="size-3.5" />Token
                    </button>
                    <button
                      type="button"
                      class="a-btn a-btn-sm a-btn-danger"
                      aria-label="Eliminar"
                      @click="remove(row)"
                    >
                      <Trash2 class="size-3.5" />
                    </button>
                  </template>
                </div>
              </td>
            </tr>
            <tr v-if="!rows.length && !loading">
              <td colspan="6" class="py-6 text-center text-muted">No hay corredores todavía.</td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>

    <RacerFormDialog
      :open="form.open"
      :racer="form.racer"
      @close="form.open = false"
      @saved="saved"
    />
    <RacerControlDialog
      v-if="control"
      :open="!!control"
      :racer-id="control.racer.id"
      :name="control.racer.displayName"
      :status="control.racer.status"
      @close="control = null"
      @done="load()"
    />
    <SecretDialog
      :open="secret.open"
      :title="secret.title"
      :secret="secret.value"
      note="En HiveShock: menú Zeldatón → pega este token junto con la dirección del servidor."
      @close="secret.open = false"
    />
  </div>
</template>
