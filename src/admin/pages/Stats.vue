<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { Download, RefreshCw } from 'lucide-vue-next'
import { adminApi } from '../api/AdminApi'
import { hms, secondsLabel } from '../format'
import { messageOf, useToasts } from '../composables/useToasts'
import InfoTip from '../components/InfoTip.vue'
import PageHeader from '../components/PageHeader.vue'
import StatCard from '../components/StatCard.vue'
import type { DayStat, DayStatsResponse } from '../types'
import { dayLabel, statsCsv, totalsOf } from '../stats'

/**
 * Day by day figures of every racer: time really played, sessions, progress, objectives, items,
 * bosses, donations, hand adjustments and times they ran out of time. A "day" is the stretch between
 * two of that racer's daily resets, named by the local date it began on.
 */
const toasts = useToasts()
const data = ref<DayStatsResponse | null>(null)
const view = ref<'day' | 'racer'>('day')
const day = ref('')
const racerId = ref('')
const loading = ref(false)

async function load() {
  loading.value = true
  try {
    const out = await adminApi.statsDays()
    data.value = out
    // Open on the latest day (and the first racer) the first time; keep the choice afterwards.
    if (!day.value || !out.days.includes(day.value)) day.value = out.days.at(-1) ?? ''
    if (!racerId.value || !out.rows.some((r) => r.racerId === racerId.value))
      racerId.value = out.rows[0]?.racerId ?? ''
  } catch (e) {
    toasts.error(messageOf(e))
  } finally {
    loading.value = false
  }
}
let timer: ReturnType<typeof setInterval> | null = null
onMounted(() => {
  void load()
  timer = setInterval(() => !document.hidden && void load(), 10000)
})
onBeforeUnmount(() => timer && clearInterval(timer))

const racers = computed(() => {
  const seen = new Map<string, string>()
  for (const r of data.value?.rows ?? []) seen.set(r.racerId, r.racerName || r.racerId)
  return [...seen].map(([id, name]) => ({ id, name })).sort((a, b) => a.name.localeCompare(b.name))
})

/** The rows on screen: every racer for one day, or one racer's every day. */
const shown = computed<DayStat[]>(() => {
  const rows = data.value?.rows ?? []
  return view.value === 'day'
    ? rows.filter((r) => r.day === day.value).sort((a, b) => b.playedSeconds - a.playedSeconds)
    : rows.filter((r) => r.racerId === racerId.value).sort((a, b) => a.day.localeCompare(b.day))
})
const totals = computed(() => totalsOf(shown.value))
const anyPartial = computed(() => shown.value.some((r) => r.partial))
const longest = computed(() => Math.max(1, ...shown.value.map((r) => r.playedSeconds)))

const signed = (s: number) => (s === 0 ? '0 s' : `${s > 0 ? '+' : '−'}${secondsLabel(s)}`)
const progress = (r: DayStat) =>
  r.progressStart === null || r.progressEnd === null
    ? '—'
    : `${Math.round(r.progressStart)}% → ${Math.round(r.progressEnd)}%`
const gained = (r: DayStat) =>
  r.progressStart === null || r.progressEnd === null
    ? null
    : Math.round(r.progressEnd - r.progressStart)

function download() {
  const blob = new Blob([statsCsv(data.value?.rows ?? [])], { type: 'text/csv;charset=utf-8' })
  const a = document.createElement('a')
  a.href = URL.createObjectURL(blob)
  a.download = `zeldaton-estadisticas-${new Date().toISOString().slice(0, 10)}.csv`
  a.click()
  URL.revokeObjectURL(a.href)
}
</script>

<template>
  <div>
    <PageHeader
      title="Estadísticas"
      subtitle="Lo que hizo cada corredor, día por día: tiempo jugado, avance, objetos, jefes y donaciones."
    >
      <div class="inline-flex overflow-hidden rounded border border-line" role="tablist">
        <button
          type="button"
          role="tab"
          class="px-3 py-1.5 text-sm"
          :class="view === 'day' ? 'bg-primary/20 text-white' : 'text-muted hover:text-white'"
          :aria-selected="view === 'day'"
          @click="view = 'day'"
        >
          Por día
        </button>
        <button
          type="button"
          role="tab"
          class="px-3 py-1.5 text-sm"
          :class="view === 'racer' ? 'bg-primary/20 text-white' : 'text-muted hover:text-white'"
          :aria-selected="view === 'racer'"
          @click="view = 'racer'"
        >
          Por corredor
        </button>
      </div>
      <select v-if="view === 'day'" v-model="day" class="a-select w-44" aria-label="Día">
        <option v-for="d in data?.days ?? []" :key="d" :value="d">{{ dayLabel(d) }}</option>
      </select>
      <select v-else v-model="racerId" class="a-select w-44" aria-label="Corredor">
        <option v-for="r in racers" :key="r.id" :value="r.id">{{ r.name }}</option>
      </select>
      <button type="button" class="a-btn" :disabled="loading" @click="load">
        <RefreshCw class="size-4" />Actualizar
      </button>
      <button type="button" class="a-btn" :disabled="!data?.rows.length" @click="download">
        <Download class="size-4" />CSV
      </button>
    </PageHeader>

    <p v-if="!data" class="text-sm text-muted">Cargando…</p>
    <template v-else-if="!data.rows.length">
      <p class="panel p-6 text-center text-sm text-muted">
        Todavía no hay días registrados. Aparecerán en cuanto los corredores empiecen a jugar.
      </p>
    </template>
    <template v-else>
      <div class="mb-6 grid gap-4 sm:grid-cols-2 lg:grid-cols-5">
        <StatCard label="Tiempo jugado" :value="hms(totals.playedSeconds)" tone="good" />
        <StatCard
          label="Objetos"
          :value="totals.items"
          :hint="`${totals.bosses} jefes · ${totals.areas} zonas`"
        />
        <StatCard
          label="Donaciones"
          :value="totals.donations"
          :hint="`+${secondsLabel(totals.donationAddedSeconds)} / −${secondsLabel(totals.donationRemovedSeconds)}`"
        />
        <StatCard
          label="Ajustes manuales"
          :value="signed(totals.adjustSeconds)"
          :tone="totals.adjustSeconds ? 'warn' : 'default'"
        />
        <StatCard
          label="Sin tiempo / cierres"
          :value="`${totals.exhausted} / ${totals.forcedCloses}`"
          :tone="totals.exhausted ? 'warn' : 'default'"
          info="Veces que se quedaron sin tiempo y veces que un organizador cerró su juego."
        />
      </div>

      <section class="panel">
        <div class="overflow-x-auto p-2">
          <table class="a-table">
            <thead>
              <tr>
                <th>{{ view === 'day' ? 'Corredor' : 'Día' }}</th>
                <th>
                  Jugado
                  <InfoTip
                    text="Tiempo realmente jugado (con el juego corriendo). Las donaciones y los ajustes no lo cambian."
                  />
                </th>
                <th>Sesiones</th>
                <th>Progreso</th>
                <th>Objetivos</th>
                <th>Objetos</th>
                <th>Jefes</th>
                <th>Zonas</th>
                <th>
                  Donaciones
                  <InfoTip
                    text="Cantidad de donaciones del día y el tiempo que sumaron y restaron al reloj (ya con los topes aplicados). «Topadas» son las que un límite recortó."
                  />
                </th>
                <th>Sumado</th>
                <th>Restado</th>
                <th>Ajustes</th>
                <th>Sin tiempo</th>
                <th>Cierres</th>
                <th>Viewers pico</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="r in shown" :key="`${r.racerId}-${r.day}`">
                <td class="font-semibold text-white">
                  {{ view === 'day' ? r.racerName : dayLabel(r.day) }}
                  <span
                    v-if="r.partial"
                    class="ml-1 cursor-help text-warning"
                    title="Este día empezó antes de que existieran las estadísticas: el tiempo jugado, las sesiones y el progreso pueden estar incompletos."
                    >~</span
                  >
                </td>
                <td class="num">
                  <div class="font-semibold text-white">{{ hms(r.playedSeconds) }}</div>
                  <div
                    class="mt-1 h-1 rounded bg-white/10"
                    role="presentation"
                    :title="`${Math.round((r.playedSeconds / longest) * 100)}% del que más jugó`"
                  >
                    <div
                      class="h-1 rounded bg-primary"
                      :style="{ width: `${Math.round((r.playedSeconds / longest) * 100)}%` }"
                    />
                  </div>
                </td>
                <td class="num">{{ r.sessions }}</td>
                <td class="num whitespace-nowrap">
                  {{ progress(r) }}
                  <span v-if="gained(r) !== null" class="text-xs text-success"
                    >(+{{ gained(r) }})</span
                  >
                </td>
                <td class="num">{{ r.objectives }}</td>
                <td class="num">{{ r.items }}</td>
                <td class="num">{{ r.bosses }}</td>
                <td class="num">{{ r.areas }}</td>
                <td class="num">
                  {{ r.donations }}
                  <span v-if="r.donationCapped" class="text-xs text-warning"
                    >({{ r.donationCapped }} topadas)</span
                  >
                </td>
                <td class="num text-success">{{ signed(r.donationAddedSeconds) }}</td>
                <td class="num text-[#ff8aa0]">{{ signed(-r.donationRemovedSeconds) }}</td>
                <td class="num" :class="r.adjustSeconds ? 'text-warning' : ''">
                  {{ signed(r.adjustSeconds) }}
                </td>
                <td class="num" :class="r.exhausted ? 'text-warning' : ''">{{ r.exhausted }}</td>
                <td class="num">{{ r.forcedCloses }}</td>
                <td class="num">{{ r.peakViewers ?? '—' }}</td>
              </tr>
              <tr v-if="!shown.length">
                <td colspan="15" class="py-6 text-center text-muted">
                  Sin datos para esta selección.
                </td>
              </tr>
            </tbody>
            <tfoot v-if="shown.length > 1">
              <tr class="border-t border-line font-semibold text-white">
                <td>Total</td>
                <td class="num">{{ hms(totals.playedSeconds) }}</td>
                <td class="num">{{ totals.sessions }}</td>
                <td />
                <td class="num">{{ totals.objectives }}</td>
                <td class="num">{{ totals.items }}</td>
                <td class="num">{{ totals.bosses }}</td>
                <td class="num">{{ totals.areas }}</td>
                <td class="num">{{ totals.donations }}</td>
                <td class="num text-success">{{ signed(totals.donationAddedSeconds) }}</td>
                <td class="num text-[#ff8aa0]">{{ signed(-totals.donationRemovedSeconds) }}</td>
                <td class="num">{{ signed(totals.adjustSeconds) }}</td>
                <td class="num">{{ totals.exhausted }}</td>
                <td class="num">{{ totals.forcedCloses }}</td>
                <td />
              </tr>
            </tfoot>
          </table>
        </div>
      </section>

      <p class="mt-3 text-xs text-muted">
        Un «día» va de un reinicio diario del corredor al siguiente y se nombra por la fecha local
        en que empezó.
        <template v-if="anyPartial"
          ><span class="text-warning">~</span> marca los días que empezaron antes de que existieran
          las estadísticas: sus conteos se reconstruyeron del historial, pero el tiempo jugado, las
          sesiones y el progreso de esa parte no se midieron.</template
        >
      </p>
    </template>
  </div>
</template>
