<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { Plus } from 'lucide-vue-next'
import { adminApi } from '../api/AdminApi'
import PageHeader from '../components/PageHeader.vue'
import SlotRow from '../components/SlotRow.vue'
import { confirm } from '../composables/useConfirm'
import { messageOf, useToasts } from '../composables/useToasts'
import { dayLabel, isoToLocalInput, localInputToIso, slotState } from '../schedule'
import { useAdminStore } from '../stores/admin'
import { useOverviewStore } from '../stores/overview'
import type { ScheduleSlot } from '../types'

/** The streams' schedule for the next days: everyone can see it and take a slot; admins keep it. */
const admin = useAdminStore()
const overview = useOverviewStore()
const toasts = useToasts()

const slots = ref<ScheduleSlot[]>([])
const loaded = ref(false)
const error = ref<string | null>(null)
const busy = ref<number | null>(null)
const now = ref(Date.now())
let tick: ReturnType<typeof setInterval> | null = null
let poll: ReturnType<typeof setInterval> | null = null

async function load() {
  try {
    const from = new Date(Date.now() - 12 * 3600_000).toISOString()
    const to = new Date(Date.now() + 14 * 24 * 3600_000).toISOString()
    slots.value = (await adminApi.schedule(from, to)).slots
    error.value = null
  } catch (e) {
    error.value = messageOf(e)
  } finally {
    loaded.value = true
  }
}

onMounted(() => {
  overview.start()
  void load()
  tick = setInterval(() => (now.value = Date.now()), 1000)
  poll = setInterval(() => !document.hidden && void load(), 15_000)
})
onBeforeUnmount(() => {
  overview.stop()
  if (tick) clearInterval(tick)
  if (poll) clearInterval(poll)
})

const online = (racerId: string) => {
  const r = overview.data?.racers.find((x) => x.racer.id === racerId)
  return !!r && (r.connected || !!r.racer.stream?.isLive)
}

/** Slots grouped by the viewer's day, in order. */
const days = computed(() => {
  const out: { label: string; slots: ScheduleSlot[] }[] = []
  for (const s of slots.value) {
    const label = dayLabel(s.startUtc)
    const last = out[out.length - 1]
    if (last?.label === label) last.slots.push(s)
    else out.push({ label, slots: [s] })
  }
  return out
})

async function run(slot: ScheduleSlot, action: () => Promise<unknown>, ok?: string) {
  busy.value = slot.id
  try {
    await action()
    if (ok) toasts.success(ok)
    await load()
  } catch (e) {
    toasts.error(messageOf(e))
  } finally {
    busy.value = null
  }
}

async function remove(slot: ScheduleSlot) {
  const yes = await confirm({
    title: 'Quitar el horario',
    message: `Se quita el live de ${slot.racerName} y las franjas tomadas por los árbitros.`,
    confirmLabel: 'Quitar',
    danger: true,
  })
  if (yes) await run(slot, () => adminApi.deleteSlot(slot.id), 'Horario quitado.')
}

// ---- new slot (admins)
const form = ref({ racerId: '', start: '', end: '', note: '' })
const saving = ref(false)
const racers = computed(() => overview.data?.racers ?? [])
const valid = computed(() => {
  const { racerId, start, end } = form.value
  return !!racerId && !!start && !!end && new Date(end) > new Date(start)
})
function useHours(hours: number) {
  if (!form.value.start) return
  const end = new Date(new Date(form.value.start).getTime() + hours * 3600_000)
  form.value.end = isoToLocalInput(end.toISOString())
}
async function create() {
  saving.value = true
  try {
    await adminApi.createSlot({
      racerId: form.value.racerId,
      startUtc: localInputToIso(form.value.start),
      endUtc: localInputToIso(form.value.end),
      note: form.value.note.trim() || undefined,
    })
    toasts.success('Live programado.')
    form.value = { ...form.value, start: '', end: '', note: '' }
    await load()
  } catch (e) {
    toasts.error(messageOf(e))
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div>
    <PageHeader
      title="Agenda"
      subtitle="Lives programados y quién los arbitra. Horas en tu zona."
    />

    <p
      v-if="error"
      class="mb-4 rounded border border-danger/50 bg-danger/10 p-3 text-sm text-[#ff8aa0]"
      role="alert"
    >
      No se pudo cargar: {{ error }}
    </p>

    <section v-if="admin.isAdmin" class="panel mb-6" aria-label="Programar un live">
      <form
        class="grid gap-3 p-4 sm:grid-cols-2 lg:grid-cols-[1fr_1fr_1fr_1fr_auto]"
        @submit.prevent="create"
      >
        <label class="a-label">
          Corredor
          <select v-model="form.racerId" class="a-select" required>
            <option value="" disabled>Elige…</option>
            <option v-for="r in racers" :key="r.racer.id" :value="r.racer.id">
              {{ r.racer.displayName }} ({{ r.racer.timezone }})
            </option>
          </select>
        </label>
        <label class="a-label">
          Empieza
          <input v-model="form.start" type="datetime-local" class="a-input" required />
        </label>
        <label class="a-label">
          Termina
          <input v-model="form.end" type="datetime-local" class="a-input" required />
          <span class="mt-1 flex gap-1">
            <button
              v-for="h in [2, 3, 4]"
              :key="h"
              type="button"
              class="a-btn a-btn-sm"
              @click="useHours(h)"
            >
              +{{ h }} h
            </button>
          </span>
        </label>
        <label class="a-label">
          Nota
          <input
            v-model="form.note"
            class="a-input"
            maxlength="200"
            placeholder="Parte 1, boss rush…"
          />
        </label>
        <button
          type="submit"
          class="a-btn a-btn-primary self-start sm:mt-5"
          :disabled="!valid || saving"
        >
          <Plus class="size-4" />Programar
        </button>
      </form>
    </section>

    <p v-if="!loaded" class="text-sm text-muted">Cargando…</p>
    <p v-else-if="!days.length" class="text-sm text-muted">
      No hay lives programados en los próximos días.
    </p>
    <section v-for="d in days" :key="d.label" class="mb-6" :aria-label="d.label">
      <h2 class="hud-label mb-2 capitalize">{{ d.label }}</h2>
      <ul class="space-y-2">
        <SlotRow
          v-for="s in d.slots"
          :key="s.id"
          :entry="s"
          :state="slotState(s, online(s.racerId), now)"
          :user-id="admin.user?.id ?? null"
          :can-remove="admin.isAdmin"
          :busy="busy === s.id"
          @take="run(s, () => adminApi.takeSlot(s.id))"
          @leave="run(s, () => adminApi.leaveSlot(s.id))"
          @remove="remove(s)"
        />
      </ul>
    </section>
  </div>
</template>
