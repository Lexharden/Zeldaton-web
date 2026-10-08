<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { Settings2 } from 'lucide-vue-next'
import { tx } from '@/i18n'
import AgeBadge from '@/components/common/AgeBadge.vue'
import LiveBadge from '@/components/common/LiveBadge.vue'
import ProgressBar from '@/components/common/ProgressBar.vue'
import { ago, hms, localDateTime } from '../format'
import EventControls from '../components/EventControls.vue'
import InfoTip from '../components/InfoTip.vue'
import PageHeader from '../components/PageHeader.vue'
import RacerControlDialog from '../components/RacerControlDialog.vue'
import StatCard from '../components/StatCard.vue'
import { useAdminStore } from '../stores/admin'
import { useOverviewStore } from '../stores/overview'
import type { OverviewRacer } from '../types'

const admin = useAdminStore()
const overview = useOverviewStore()
const data = computed(() => overview.data)

// A 1 s tick keeps live clocks moving between the 3 s polls.
const now = ref(Date.now())
let tick: ReturnType<typeof setInterval> | null = null
onMounted(() => {
  overview.start()
  tick = setInterval(() => (now.value = Date.now()), 1000)
})
onBeforeUnmount(() => {
  overview.stop()
  if (tick) clearInterval(tick)
})

const controlled = ref<OverviewRacer | null>(null)

/** Why this racer is where they are: the values the ranking compares, in order. */
function rankWhy(row: OverviewRacer): string {
  const k = row.rankKey
  if (!k) return ''
  if (row.racer.status === 'finished')
    return `Terminó el juego${row.racer.finishedAtUtc ? ` · ${localDateTime(row.racer.finishedAtUtc)}` : ''}. Ordena quien cruzó la meta primero.`
  return [
    `Objetivos requeridos: ${k.requiredDone}`,
    `Progreso: ${k.progressPercent}%`,
    `Ítems: ${k.items}`,
    `Llegó a ese punto: ${k.milestoneAtUtc ? localDateTime(k.milestoneAtUtc) : 'aún no'}`,
    `Tiempo jugado: ${hms(k.playedSeconds)}`,
    'Las donaciones no cuentan.',
  ].join('\n')
}
const summary = computed(() => data.value?.summary)
const winnerName = computed(() => {
  const w = data.value?.winner
  if (!w) return ''
  return data.value?.racers.find((r) => r.racer.id === w.racerId)?.racer.displayName ?? w.racerId
})
const levelTone = {
  error: 'border-danger/60 text-[#ff8aa0]',
  warn: 'border-warning/50 text-warning',
  info: 'border-secondary/40 text-secondary',
}
</script>

<template>
  <div>
    <PageHeader
      title="Panel"
      :subtitle="data ? `${data.event.name} · ${data.event.edition}` : 'Cargando…'"
    >
      <LiveBadge v-if="data" :status="data.event.status" />
      <span
        v-if="data?.event.rehearsal"
        class="hud-label rounded border border-warning/60 px-2 py-1 text-warning"
        >ENSAYO</span
      >
      <EventControls
        v-if="data && admin.isAdmin"
        :status="data.event.status"
        @changed="overview.refresh()"
      />
    </PageHeader>

    <p
      v-if="overview.error"
      class="mb-4 rounded border border-danger/50 bg-danger/10 p-3 text-sm text-[#ff8aa0]"
      role="alert"
    >
      No se pudo actualizar: {{ overview.error }}
    </p>

    <template v-if="data && summary">
      <p
        v-if="data.winner"
        class="mb-4 rounded border border-accent/50 bg-accent/10 p-3 text-sm text-accent"
      >
        🏆 Ganador: <strong>{{ winnerName }}</strong>
        <span v-if="data.winner.finalTimeSeconds"> · {{ hms(data.winner.finalTimeSeconds) }}</span>
      </p>

      <section v-if="data.alerts.length" aria-label="Alertas" class="mb-6 space-y-2">
        <div
          v-for="(a, i) in data.alerts"
          :key="i"
          class="rounded border bg-white/[0.03] px-3 py-2 text-sm"
          :class="levelTone[a.level]"
        >
          {{ a.message }}
        </div>
      </section>

      <section
        class="mb-6 grid grid-cols-2 gap-3 sm:grid-cols-3 xl:grid-cols-6"
        aria-label="Resumen"
      >
        <StatCard
          label="Conectados"
          info="Corredores con HiveShock (el programa que reporta el juego) conectado, sobre el total."
          :value="`${summary.connected} / ${summary.racers}`"
          :tone="summary.connected ? 'good' : 'warn'"
        />
        <StatCard
          label="En vivo"
          info="Jugando ahora: su reloj del día está corriendo."
          :value="summary.live"
          :tone="summary.live ? 'good' : 'default'"
        />
        <StatCard
          label="En pausa"
          info="Partida pausada: su reloj del día está detenido."
          :value="summary.paused"
        />
        <StatCard
          label="En espera"
          info="Conectados pero sin partida activa todavía."
          :value="summary.online"
        />
        <StatCard
          label="Sin tiempo"
          info="Agotaron el tiempo de hoy. Vuelven a jugar tras su reinicio diario."
          :value="summary.exhausted"
          :tone="summary.exhausted ? 'warn' : 'default'"
        />
        <StatCard
          label="Terminaron"
          info="Completaron el juego (todos los objetivos requeridos)."
          :value="summary.finished"
        />
      </section>

      <section class="panel mb-6" aria-label="Corredores">
        <div class="overflow-x-auto p-2">
          <table class="a-table">
            <thead>
              <tr>
                <th>Corredor</th>
                <th>
                  Estado
                  <InfoTip
                    text="Situación del corredor: En vivo (jugando, el reloj corre), En pausa, En espera (conectado sin partida), Desconectado, Sin tiempo (agotó el tiempo de hoy) o Terminó."
                  />
                </th>
                <th>
                  Señal
                  <InfoTip
                    text="Conexión de HiveShock. El punto verde indica que está conectado y el texto, hace cuánto llegó su último latido. Sin señal, el servidor deja de recibir datos y su reloj se detiene."
                  />
                </th>
                <th>
                  Tiempo hoy
                  <InfoTip
                    text="Tiempo que le queda hoy de su presupuesto diario. Solo corre mientras juega y se reinicia a la hora de reinicio del evento, en su zona horaria."
                  />
                </th>
                <th>
                  Jugado hoy
                  <InfoTip
                    text="Tiempo que realmente ha jugado hoy (solo cuenta con el juego corriendo). A diferencia de «Tiempo hoy», las donaciones y los ajustes de tiempo no lo cambian: sirve para ver cuánto jugó antes de que se le cerrara el juego o se le acabara el tiempo."
                  />
                </th>
                <th class="min-w-40">
                  Progreso
                  <InfoTip text="Avance total del juego que reporta HiveShock, de 0 a 100 %." />
                </th>
                <th>
                  Ahora
                  <InfoTip
                    text="El Link que está jugando (niño o adulto) y la zona del juego donde se encuentra."
                  />
                </th>
                <th class="text-right">
                  Viewers
                  <InfoTip
                    text="Espectadores conectados a su transmisión en este momento (Twitch o TikTok). Aparece — si no está transmitiendo o HiveShock no lo reporta."
                  />
                </th>
                <th />
              </tr>
            </thead>
            <tbody>
              <tr v-for="row in data.racers" :key="row.racer.id">
                <td>
                  <RouterLink
                    :to="`/racer/${row.racer.id}`"
                    class="font-semibold text-white hover:underline"
                    >{{ row.racer.displayName }}</RouterLink
                  >
                  <div class="text-xs text-muted">{{ row.racer.id }}</div>
                  <div
                    v-if="row.rank"
                    class="num mt-0.5 inline-block cursor-help text-xs font-semibold text-gold"
                    :title="rankWhy(row)"
                  >
                    #{{ row.rank }}
                  </div>
                </td>
                <td><LiveBadge :status="row.racer.status" size="sm" /></td>
                <td>
                  <span
                    class="a-dot"
                    :class="row.connected && 'a-dot-on'"
                    :title="row.connected ? 'HiveShock conectado' : 'Sin conexión'"
                  />
                  <span class="ml-2 text-xs text-muted">{{
                    row.connected ? ago(row.heartbeatAgeSeconds) : 'sin señal'
                  }}</span>
                </td>
                <td class="num font-semibold text-white">
                  {{
                    hms(overview.remainingNow(row.racer.status, row.racer.remainingSeconds, now))
                  }}
                </td>
                <td class="num text-secondary">
                  {{
                    hms(
                      overview.playedNow(row.racer.status, row.racer.playedTodaySeconds ?? 0, now),
                    )
                  }}
                </td>
                <td>
                  <ProgressBar
                    :value="row.racer.progressPercentage"
                    tone="gold"
                    :segments="10"
                    :label="`Progreso de ${row.racer.displayName}`"
                  />
                  <div class="num mt-0.5 text-xs text-muted">
                    {{ Math.round(row.racer.progressPercentage) }}%
                  </div>
                </td>
                <td>
                  <div class="flex flex-wrap items-center gap-2">
                    <AgeBadge v-if="row.racer.stats?.age" :age="row.racer.stats.age" short />
                    <span class="text-xs text-secondary">{{
                      row.racer.currentArea ? tx('areas', row.racer.currentArea) : '—'
                    }}</span>
                  </div>
                </td>
                <td class="num text-right">{{ row.racer.stream?.viewers ?? '—' }}</td>
                <td class="text-right">
                  <button type="button" class="a-btn a-btn-sm" @click="controlled = row">
                    <Settings2 class="size-3.5" />Controlar
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>

      <div class="grid gap-6 lg:grid-cols-[1fr_20rem]">
        <section class="panel" aria-label="Actividad reciente">
          <div class="p-4">
            <h2 class="hud-label mb-3">
              ACTIVIDAD RECIENTE
              <InfoTip
                text="Los últimos eventos del juego: ítems, jefes, cambios de zona y de estado."
              />
            </h2>
            <ul v-if="data.activity.length" class="space-y-2 text-sm">
              <li v-for="a in data.activity" :key="a.id" class="flex gap-3">
                <span class="num shrink-0 text-xs text-muted">{{
                  localDateTime(a.timestampUtc)
                }}</span>
                <span class="text-white">{{ a.message }}</span>
              </li>
            </ul>
            <p v-else class="text-sm text-muted">Todavía no hay actividad.</p>
          </div>
        </section>
        <section class="grid grid-cols-2 gap-3 lg:grid-cols-1" aria-label="HiveShock">
          <StatCard label="Eventos de juego" :value="data.hiveshock.gameEvents" />
          <StatCard label="Ítems" :value="data.hiveshock.itemEvents" />
          <StatCard label="Progreso" :value="data.hiveshock.progressEvents" />
          <StatCard label="Mensajes de chat" :value="data.hiveshock.chatEvents" />
        </section>
      </div>
    </template>
    <p v-else-if="!overview.error" class="text-sm text-muted">Cargando…</p>

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
