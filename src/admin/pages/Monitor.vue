<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { CheckCheck, EyeOff, RotateCcw, Send, Settings2, Trash2 } from 'lucide-vue-next'
import { tx } from '@/i18n'
import LiveBadge from '@/components/common/LiveBadge.vue'
import ProgressBar from '@/components/common/ProgressBar.vue'
import { actionLabel, ago, hms, localDateTime } from '../format'
import PageHeader from '../components/PageHeader.vue'
import RacerControlDialog from '../components/RacerControlDialog.vue'
import StatCard from '../components/StatCard.vue'
import { messageOf, useToasts } from '../composables/useToasts'
import { useAdminStore } from '../stores/admin'
import { useMonitorStore } from '../stores/monitor'
import { useOverviewStore } from '../stores/overview'
import type { Incident, IncidentKind, IncidentStatus, OverviewRacer, RefereeNote } from '../types'

/** Referee monitor: who is playing now, what needs a look, and what happened while nobody was here. */
const overview = useOverviewStore()
const monitor = useMonitorStore()
const admin = useAdminStore()
const toasts = useToasts()

const now = ref(Date.now())
let tick: ReturnType<typeof setInterval> | null = null
onMounted(() => {
  overview.start()
  monitor.start()
  tick = setInterval(() => (now.value = Date.now()), 1000)
})
onBeforeUnmount(() => {
  overview.stop()
  monitor.stop()
  if (tick) clearInterval(tick)
})

const KIND_LABEL: Record<IncidentKind, string> = {
  disconnected: 'Desconexión',
  low_time: 'Poco tiempo',
  suspicious: 'Progreso sospechoso',
  donation_cap: 'Tope de donaciones',
  exhausted: 'Sin tiempo',
}
const levelTone = {
  error: 'border-danger/60 text-[#ff8aa0]',
  warn: 'border-warning/50 text-warning',
  info: 'border-secondary/40 text-secondary',
}

// ---- who is playing now
const racers = computed(() => overview.data?.racers ?? [])
const eventLive = computed(() => overview.data?.event.status === 'live')
const ORDER: Record<string, number> = {
  live: 0,
  paused: 1,
  online: 2,
  offline: 3,
  exhausted: 4,
  finished: 5,
}
const sortedRacers = computed(() =>
  [...racers.value].sort(
    (a, b) =>
      (ORDER[a.racer.status] ?? 9) - (ORDER[b.racer.status] ?? 9) ||
      a.racer.displayName.localeCompare(b.racer.displayName),
  ),
)
const openByRacer = computed(() => {
  const out = new Map<string, number>()
  for (const i of monitor.data?.open ?? []) {
    if (i.racerId) out.set(i.racerId, (out.get(i.racerId) ?? 0) + 1)
  }
  return out
})
/** The game should be sending data but HiveShock is not connected. */
const noSignal = (r: OverviewRacer) =>
  eventLive.value && !r.connected && ['live', 'paused', 'online'].includes(r.racer.status)
const controlled = ref<OverviewRacer | null>(null)

// ---- incidents
const filterRacer = ref('')
const tab = ref<'open' | 'closed'>('open')
const shown = computed<Incident[]>(() => {
  const list = tab.value === 'open' ? (monitor.data?.open ?? []) : (monitor.data?.recent ?? [])
  return filterRacer.value ? list.filter((i) => i.racerId === filterRacer.value) : list
})
const notes = ref<Record<number, string>>({})
const busy = ref<number | null>(null)

async function review(i: Incident, status: IncidentStatus) {
  busy.value = i.id
  try {
    await monitor.review(i, status, notes.value[i.id]?.trim() || undefined)
    delete notes.value[i.id]
  } catch (e) {
    toasts.error(messageOf(e))
  } finally {
    busy.value = null
  }
}

const ageOf = (iso: string) => Math.max(0, Math.round((now.value - new Date(iso).getTime()) / 1000))

// ---- log book
const noteText = ref('')
const noteRacer = ref('')
const noteBusy = ref(false)
async function addNote() {
  if (!noteText.value.trim()) return
  noteBusy.value = true
  try {
    await monitor.addNote(noteText.value, noteRacer.value || undefined)
    noteText.value = ''
  } catch (e) {
    toasts.error(messageOf(e))
  } finally {
    noteBusy.value = false
  }
}
async function removeNote(n: RefereeNote) {
  try {
    await monitor.deleteNote(n.id)
  } catch (e) {
    toasts.error(messageOf(e))
  }
}
/** Admins delete any note; everyone else only their own. */
const canDeleteNote = (n: RefereeNote) => admin.isAdmin || n.author === admin.user?.username

// ---- since the last visit
const catchUp = computed(() => monitor.data?.catchUp)
const hasNews = computed(() => {
  const c = catchUp.value
  return !!c && (c.newIncidents > 0 || c.actions.length > 0 || c.donationsLimited > 0)
})
const racerName = (id: string | null) =>
  (id && racers.value.find((r) => r.racer.id === id)?.racer.displayName) || id || ''
async function markSeen() {
  try {
    await monitor.markSeen()
  } catch (e) {
    toasts.error(messageOf(e))
  }
}
</script>

<template>
  <div>
    <PageHeader
      title="Monitor"
      :subtitle="overview.data ? `${overview.data.event.name} · vista de árbitros` : 'Cargando…'"
    >
      <LiveBadge v-if="overview.data" :status="overview.data.event.status" />
    </PageHeader>

    <p
      v-if="monitor.error || overview.error"
      class="mb-4 rounded border border-danger/50 bg-danger/10 p-3 text-sm text-[#ff8aa0]"
      role="alert"
    >
      No se pudo actualizar: {{ monitor.error || overview.error }}
    </p>

    <section
      v-if="catchUp && hasNews"
      class="panel mb-6"
      aria-label="Desde tu última visita"
      data-test="catch-up"
    >
      <div class="p-4">
        <div class="flex flex-wrap items-start justify-between gap-3">
          <div>
            <h2 class="hud-label">DESDE TU ÚLTIMA VISITA</h2>
            <p class="mt-1 text-xs text-muted">
              {{
                monitor.data?.lastSeenUtc
                  ? `Desde ${localDateTime(catchUp.since)}`
                  : 'Es tu primera vez aquí: últimas 12 horas'
              }}
            </p>
          </div>
          <button type="button" class="a-btn a-btn-sm" @click="markSeen">
            <CheckCheck class="size-3.5" />Marcar como leído
          </button>
        </div>
        <ul class="mt-3 space-y-1 text-sm">
          <li v-if="catchUp.newIncidents" class="text-white">
            <strong>{{ catchUp.newIncidents }}</strong>
            {{ catchUp.newIncidents === 1 ? 'incidente nuevo' : 'incidentes nuevos' }}:
            <span class="text-muted">{{
              Object.entries(catchUp.byKind)
                .map(([k, n]) => `${KIND_LABEL[k as IncidentKind] ?? k} ×${n}`)
                .join(' · ')
            }}</span>
          </li>
          <li v-if="catchUp.donationsLimited" class="text-white">
            <strong>{{ catchUp.donationsLimited }}</strong> de {{ catchUp.donations }} donaciones de
            tiempo se recortaron por un límite.
          </li>
          <li v-for="(a, idx) in catchUp.actions" :key="idx" class="flex gap-3">
            <span class="num shrink-0 text-xs text-muted">{{ localDateTime(a.ts) }}</span>
            <span class="text-white"
              >{{ a.actor }}: {{ actionLabel(a.action)
              }}<template v-if="a.racerId"> · {{ racerName(a.racerId) }}</template
              ><template v-if="a.payload?.reason">
                — <em class="text-muted">{{ a.payload.reason }}</em></template
              ></span
            >
          </li>
        </ul>
      </div>
    </section>

    <section class="mb-6 grid grid-cols-2 gap-3 sm:grid-cols-4" aria-label="Resumen">
      <StatCard
        label="Incidentes abiertos"
        :value="monitor.openCount"
        :tone="monitor.openCount ? 'bad' : 'good'"
      />
      <StatCard
        label="En vivo"
        :value="overview.data?.summary.live ?? '—'"
        :tone="overview.data?.summary.live ? 'good' : 'default'"
      />
      <StatCard
        label="Conectados"
        :value="
          overview.data
            ? `${overview.data.summary.connected} / ${overview.data.summary.racers}`
            : '—'
        "
      />
      <StatCard label="Sin tiempo" :value="overview.data?.summary.exhausted ?? '—'" />
    </section>

    <section aria-label="Ahora" class="mb-6">
      <h2 class="hud-label mb-3">AHORA</h2>
      <div class="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
        <article
          v-for="row in sortedRacers"
          :key="row.racer.id"
          class="panel panel-sm"
          :data-racer="row.racer.id"
        >
          <div class="p-4">
            <div class="flex items-start justify-between gap-2">
              <div class="min-w-0">
                <RouterLink
                  :to="`/racer/${row.racer.id}`"
                  class="block truncate font-semibold text-white hover:underline"
                  >{{ row.racer.displayName }}</RouterLink
                >
                <p class="truncate text-xs text-muted">
                  {{ row.racer.currentArea ? tx('areas', row.racer.currentArea) : '—' }}
                </p>
              </div>
              <LiveBadge :status="row.racer.status" size="sm" />
            </div>
            <div class="mt-3">
              <ProgressBar
                :value="row.racer.progressPercentage"
                tone="gold"
                :segments="10"
                :label="`Progreso de ${row.racer.displayName}`"
              />
              <div class="num mt-0.5 flex justify-between text-xs text-muted">
                <span>{{ Math.round(row.racer.progressPercentage) }}%</span>
                <span class="font-semibold text-white">{{
                  hms(overview.remainingNow(row.racer.status, row.racer.remainingSeconds, now))
                }}</span>
              </div>
            </div>
            <div class="mt-3 flex flex-wrap items-center gap-2 text-xs">
              <span
                class="inline-flex items-center gap-1.5"
                :class="noSignal(row) ? 'text-[#ff8aa0]' : 'text-muted'"
              >
                <span class="a-dot" :class="row.connected && 'a-dot-on'" />
                {{
                  row.connected
                    ? ago(row.heartbeatAgeSeconds)
                    : noSignal(row)
                      ? 'sin señal de HiveShock'
                      : 'sin conexión'
                }}
              </span>
              <span v-if="row.racer.stream?.isLive" class="text-success"
                >● en vivo{{
                  row.racer.stream.viewers != null ? ` · ${row.racer.stream.viewers}` : ''
                }}</span
              >
              <span
                v-if="openByRacer.get(row.racer.id)"
                class="num rounded-full bg-danger/80 px-1.5 font-bold text-white"
                :title="`${openByRacer.get(row.racer.id)} incidentes abiertos`"
                >{{ openByRacer.get(row.racer.id) }} ⚠</span
              >
              <button type="button" class="a-btn a-btn-sm ml-auto" @click="controlled = row">
                <Settings2 class="size-3.5" />Controlar
              </button>
            </div>
          </div>
        </article>
      </div>
      <p v-if="!sortedRacers.length && overview.data" class="text-sm text-muted">
        Todavía no hay corredores.
      </p>
    </section>

    <section class="panel mb-6" aria-label="Incidentes">
      <div class="p-4">
        <div class="mb-3 flex flex-wrap items-center justify-between gap-3">
          <div class="flex gap-1" role="tablist">
            <button
              type="button"
              role="tab"
              class="a-btn a-btn-sm"
              :class="tab === 'open' && 'a-btn-primary'"
              :aria-selected="tab === 'open'"
              @click="tab = 'open'"
            >
              Abiertos ({{ monitor.openCount }})
            </button>
            <button
              type="button"
              role="tab"
              class="a-btn a-btn-sm"
              :class="tab === 'closed' && 'a-btn-primary'"
              :aria-selected="tab === 'closed'"
              @click="tab = 'closed'"
            >
              Revisados
            </button>
          </div>
          <select v-model="filterRacer" class="a-select !w-auto" aria-label="Filtrar por corredor">
            <option value="">Todos los corredores</option>
            <option v-for="r in racers" :key="r.racer.id" :value="r.racer.id">
              {{ r.racer.displayName }}
            </option>
          </select>
        </div>

        <ul v-if="shown.length" class="space-y-2" data-test="incidents">
          <li
            v-for="i in shown"
            :key="i.id"
            class="rounded border bg-white/[0.03] px-3 py-2 text-sm"
            :class="levelTone[i.severity]"
          >
            <div class="flex flex-wrap items-baseline justify-between gap-2">
              <span class="hud-label !text-inherit">{{ KIND_LABEL[i.kind] ?? i.kind }}</span>
              <span class="num text-xs text-muted"
                >{{ localDateTime(i.ts) }} · {{ ago(ageOf(i.ts)) }}</span
              >
            </div>
            <p class="mt-1 text-white">{{ i.message }}</p>
            <p v-if="i.endedAt" class="mt-0.5 text-xs text-success">
              La condición terminó ({{ localDateTime(i.endedAt) }}).
            </p>
            <div v-if="i.status === 'open'" class="mt-2 flex flex-wrap items-center gap-2">
              <input
                v-model="notes[i.id]"
                class="a-input min-w-40 flex-1"
                maxlength="500"
                placeholder="Nota (opcional)"
                :aria-label="`Nota del incidente ${i.id}`"
              />
              <button
                type="button"
                class="a-btn a-btn-sm a-btn-primary"
                :disabled="busy === i.id"
                @click="review(i, 'reviewed')"
              >
                <CheckCheck class="size-3.5" />Revisado
              </button>
              <button
                type="button"
                class="a-btn a-btn-sm"
                :disabled="busy === i.id"
                @click="review(i, 'dismissed')"
              >
                <EyeOff class="size-3.5" />Descartar
              </button>
            </div>
            <div v-else class="mt-2 flex flex-wrap items-center justify-between gap-2 text-xs">
              <span class="text-muted">
                {{ i.status === 'reviewed' ? 'Revisado' : 'Descartado' }} por {{ i.reviewedBy }} ·
                {{ localDateTime(i.reviewedAt) }}<template v-if="i.note"> — {{ i.note }}</template>
              </span>
              <button
                type="button"
                class="a-btn a-btn-sm"
                :disabled="busy === i.id"
                @click="review(i, 'open')"
              >
                <RotateCcw class="size-3.5" />Reabrir
              </button>
            </div>
          </li>
        </ul>
        <p v-else class="text-sm text-muted">
          {{
            tab === 'open' ? 'Sin incidentes abiertos. Todo en orden.' : 'Nada revisado todavía.'
          }}
        </p>
      </div>
    </section>

    <section class="panel" aria-label="Bitácora">
      <div class="p-4">
        <h2 class="hud-label mb-3">BITÁCORA DE ÁRBITROS</h2>
        <form class="mb-4 flex flex-wrap gap-2" @submit.prevent="addNote">
          <select v-model="noteRacer" class="a-select !w-auto" aria-label="Corredor de la nota">
            <option value="">General</option>
            <option v-for="r in racers" :key="r.racer.id" :value="r.racer.id">
              {{ r.racer.displayName }}
            </option>
          </select>
          <input
            v-model="noteText"
            class="a-input min-w-48 flex-1"
            maxlength="500"
            placeholder="Qué viste, qué se acordó, qué sigue pendiente…"
            aria-label="Nueva nota"
          />
          <button
            type="submit"
            class="a-btn a-btn-primary"
            :disabled="noteBusy || !noteText.trim()"
          >
            <Send class="size-4" />Anotar
          </button>
        </form>
        <ul v-if="monitor.data?.notes.length" class="space-y-2 text-sm" data-test="notes">
          <li v-for="n in monitor.data.notes" :key="n.id" class="flex items-start gap-3">
            <span class="num shrink-0 text-xs text-muted">{{ localDateTime(n.ts) }}</span>
            <span class="min-w-0 flex-1 text-white">
              <strong>{{ n.author }}</strong
              ><template v-if="n.racerId"> · {{ racerName(n.racerId) }}</template
              >: {{ n.text }}
            </span>
            <button
              v-if="canDeleteNote(n)"
              type="button"
              class="text-muted hover:text-white"
              :aria-label="`Borrar la nota de ${n.author}`"
              @click="removeNote(n)"
            >
              <Trash2 class="size-3.5" />
            </button>
          </li>
        </ul>
        <p v-else class="text-sm text-muted">Todavía no hay notas. Úsala para pasar turno.</p>
      </div>
    </section>

    <RacerControlDialog
      v-if="controlled"
      :open="!!controlled"
      :racer-id="controlled.racer.id"
      :name="controlled.racer.displayName"
      :status="controlled.racer.status"
      @close="controlled = null"
      @done="overview.refresh()"
    />
  </div>
</template>
