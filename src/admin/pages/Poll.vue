<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { RefreshCw, Trash2 } from 'lucide-vue-next'
import type { AdminPoll, PollMode } from '@/types/poll'
import PollResults from '@/components/poll/PollResults.vue'
import { adminApi } from '../api/AdminApi'
import { useAdminStore } from '../stores/admin'
import { confirm } from '../composables/useConfirm'
import { localDateTime } from '../format'
import { messageOf, useToasts } from '../composables/useToasts'
import PageHeader from '../components/PageHeader.vue'
import StatCard from '../components/StatCard.vue'

/** The public "who will win?" poll: state, live results and the reset. Only admins change anything. */
const toasts = useToasts()
const admin = useAdminStore()
const poll = ref<AdminPoll | null>(null)
const loading = ref(false)

async function load() {
  loading.value = true
  try {
    poll.value = await adminApi.poll()
  } catch (e) {
    toasts.error(messageOf(e))
  } finally {
    loading.value = false
  }
}

let timer: ReturnType<typeof setInterval> | null = null
onMounted(() => {
  void load()
  timer = setInterval(() => !document.hidden && void load(), 15000)
})
onBeforeUnmount(() => timer && clearInterval(timer))

const MODES: { value: PollMode; label: string; hint: string }[] = [
  {
    value: 'auto',
    label: 'Automática',
    hint: 'Abierta durante la carrera; se cierra sola cuando alguien gana o el evento termina.',
  },
  { value: 'open', label: 'Abierta', hint: 'Se puede votar siempre, incluso si ya hay ganador.' },
  { value: 'closed', label: 'Cerrada', hint: 'Nadie puede votar; la web muestra los resultados.' },
]

async function setMode(mode: PollMode) {
  try {
    poll.value = await adminApi.setPollMode(mode)
    toasts.success(`Encuesta: ${MODES.find((m) => m.value === mode)?.label.toLowerCase()}.`)
  } catch (e) {
    toasts.error(messageOf(e))
  }
}

async function clearVotes() {
  const ok = await confirm({
    title: 'Reiniciar votos',
    message:
      'Se borran todos los votos y cualquiera podrá volver a votar. Úsalo para quitar las pruebas antes de abrir la encuesta de verdad.',
    confirmLabel: 'Borrar votos',
    danger: true,
    requireText: 'BORRAR',
  })
  if (!ok) return
  try {
    poll.value = await adminApi.clearPollVotes()
    toasts.success('Votos borrados.')
  } catch (e) {
    toasts.error(messageOf(e))
  }
}

async function removeVote(id: number, name: string | null) {
  const ok = await confirm({
    title: 'Quitar voto',
    message: `Se borra el voto de «${name ?? 'sin nombre'}». Esa conexión podrá votar de nuevo.`,
    confirmLabel: 'Quitar',
    danger: true,
  })
  if (!ok) return
  try {
    poll.value = await adminApi.deletePollVote(id)
    toasts.success('Voto quitado.')
  } catch (e) {
    toasts.error(messageOf(e))
  }
}

const nameOf = (id: string) => poll.value?.options.find((o) => o.racerId === id)?.name ?? id

const leader = computed(() => {
  const p = poll.value
  const top = p?.results?.[0]
  if (!p || !top || top.votes === 0) return '—'
  return p.options.find((o) => o.racerId === top.racerId)?.name ?? top.racerId
})
</script>

<template>
  <div>
    <PageHeader
      title="Encuesta"
      subtitle="¿Quién ganará? Un voto por conexión (se guarda una huella cifrada de la IP, nunca la IP)."
    >
      <button type="button" class="a-btn" :disabled="loading" @click="load">
        <RefreshCw class="size-4" />Actualizar
      </button>
    </PageHeader>

    <template v-if="poll">
      <div class="mb-6 grid gap-4 sm:grid-cols-3">
        <StatCard
          label="Estado"
          :value="poll.open ? 'Abierta' : 'Cerrada'"
          :tone="poll.open ? 'good' : 'default'"
        />
        <StatCard label="Votos" :value="String(poll.totalVotes ?? 0)" />
        <StatCard label="Va ganando" :value="leader" />
      </div>

      <section class="panel mb-6 p-4">
        <h2 class="display mb-3 text-2xl text-white">Cuándo se puede votar</h2>
        <div class="grid gap-3 sm:grid-cols-3" role="radiogroup" aria-label="Modo de la encuesta">
          <label
            v-for="m in MODES"
            :key="m.value"
            class="flex cursor-pointer flex-col gap-1 border p-3"
            :class="poll.mode === m.value ? 'border-accent bg-accent/10' : 'border-line'"
          >
            <span class="flex items-center gap-2 font-semibold text-white">
              <input
                type="radio"
                name="poll-mode"
                :value="m.value"
                :checked="poll.mode === m.value"
                :disabled="!admin.isAdmin"
                @change="setMode(m.value)"
              />{{ m.label }}
            </span>
            <span class="text-xs text-muted">{{ m.hint }}</span>
          </label>
        </div>
        <p v-if="!admin.isAdmin" class="mt-2 text-xs text-muted">
          Solo un administrador puede cambiarlo.
        </p>
      </section>

      <section class="panel p-4">
        <div class="mb-4 flex flex-wrap items-center justify-between gap-3">
          <h2 class="display text-2xl text-white">Resultados</h2>
          <button
            v-if="admin.isAdmin"
            type="button"
            class="a-btn a-btn-danger"
            :disabled="!(poll.totalVotes ?? 0)"
            @click="clearVotes"
          >
            <Trash2 class="size-4" />Reiniciar votos
          </button>
        </div>
        <PollResults v-if="poll.results?.length" :poll="poll" />
        <p v-else class="text-sm text-muted">Todavía no hay corredores para votar.</p>
      </section>

      <section v-if="poll.voters.length" class="panel mt-6 p-4">
        <h2 class="display mb-1 text-2xl text-white">Quién votó</h2>
        <p class="mb-3 text-xs text-muted">
          Los nombres solo los ves tú; la web pública nunca los muestra. Últimos
          {{ poll.voters.length }} votos.
        </p>
        <div class="overflow-x-auto">
          <table class="a-table">
            <thead>
              <tr>
                <th>Nombre</th>
                <th>Votó por</th>
                <th>Cuándo</th>
                <th v-if="admin.isAdmin"></th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="v in poll.voters" :key="v.id">
                <td class="font-semibold text-white">{{ v.name ?? '—' }}</td>
                <td>{{ nameOf(v.racerId) }}</td>
                <td class="num">{{ localDateTime(v.at) }}</td>
                <td v-if="admin.isAdmin" class="text-right">
                  <button
                    type="button"
                    class="a-btn a-btn-sm a-btn-danger"
                    :aria-label="`Quitar el voto de ${v.name ?? 'sin nombre'}`"
                    @click="removeVote(v.id, v.name)"
                  >
                    <Trash2 class="size-4" />
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>
    </template>
    <p v-else class="text-sm text-muted">Cargando…</p>
  </div>
</template>
