<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { Eye, EyeOff, RefreshCw } from 'lucide-vue-next'
import PlatformIcon from '@/components/racers/PlatformIcon.vue'
import { adminApi } from '../api/AdminApi'
import { localDateTime, secondsLabel, shortDuration } from '../format'
import { messageOf, useToasts } from '../composables/useToasts'
import PageHeader from '../components/PageHeader.vue'
import StatCard from '../components/StatCard.vue'
import type { DonationRow, DonationsResponse, TopDonor } from '../types'

const toasts = useToasts()
const data = ref<DonationsResponse | null>(null)
const names = ref<Record<string, string>>({})
const racer = ref('')
const limit = ref(200)
const loading = ref(false)

async function load() {
  loading.value = true
  try {
    const [out, racers] = await Promise.all([
      adminApi.donations(limit.value, racer.value || undefined),
      Object.keys(names.value).length ? Promise.resolve(null) : adminApi.listRacers(),
    ])
    data.value = out
    if (racers) {
      names.value = Object.fromEntries(racers.map((r) => [r.racer.id, r.racer.displayName]))
    }
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

const nameOf = (id: string) => names.value[id] ?? id

/** One row per racer: today (what counts against the limits) and the whole event. */
const perRacer = computed(() => {
  const d = data.value
  if (!d) return []
  return Object.keys(names.value)
    .map((id) => {
      const today = d.today.find((t) => t.racerId === id)
      const total = d.totals.find((t) => t.racerId === id)
      return {
        id,
        todayAdded: today?.addedSeconds ?? 0,
        todayRemoved: today?.removedSeconds ?? 0,
        donations: total?.donations ?? 0,
        added: total?.addedSeconds ?? 0,
        removed: total?.removedSeconds ?? 0,
      }
    })
    .filter((r) => r.donations > 0 || r.todayAdded > 0 || r.todayRemoved > 0)
    .sort((a, b) => b.added + b.removed - (a.added + a.removed))
})

const totals = computed(() =>
  perRacer.value.reduce(
    (acc, r) => ({ added: acc.added + r.added, removed: acc.removed + r.removed }),
    { added: 0, removed: 0 },
  ),
)

function donationText(d: DonationRow): string {
  const unit = d.currency === 'bits' ? 'bits' : 'diamantes'
  const paid = `${d.amount.toLocaleString('es-MX')} ${unit}`
  if (!d.gift) return paid
  return `${d.gift}${d.giftCount && d.giftCount > 1 ? ` x${d.giftCount}` : ''} · ${paid}`
}

const MEDALS = ['🥇', '🥈', '🥉']

function paidText(d: TopDonor): string {
  const unit = d.currency === 'bits' ? 'bits' : 'diamantes'
  return `${d.amount.toLocaleString('es-MX')} ${unit}`
}

async function toggleDonor(d: TopDonor) {
  try {
    await adminApi.setDonorHidden(d.platform, d.viewer, !d.hidden)
    toasts.success(
      d.hidden ? `«${d.viewer}» vuelve a verse en la web.` : `«${d.viewer}» ya no se ve en la web.`,
    )
    await load()
  } catch (e) {
    toasts.error(messageOf(e))
  }
}

async function togglePublic(enabled: boolean) {
  try {
    await adminApi.setDonorsVisible(enabled)
    toasts.success(
      enabled
        ? 'El top de donadores se muestra en la web.'
        : 'El top de donadores ya no se muestra en la web.',
    )
    await load()
  } catch (e) {
    toasts.error(messageOf(e))
  }
}

function signed(seconds: number): string {
  if (seconds === 0) return '0 s'
  return `${seconds > 0 ? '+' : '−'}${secondsLabel(seconds)}`
}

const LIMITS: Record<string, string> = {
  per_donation: 'Tope por donación',
  daily_limit: 'Tope diario alcanzado',
  clock_max: 'Reloj al máximo',
  clock_zero: 'Reloj en cero',
}
</script>

<template>
  <div>
    <PageHeader
      title="Donaciones"
      subtitle="Tiempo que sumaron o restaron los regalos de TikTok y los bits de Twitch."
    >
      <select v-model="racer" class="a-select w-44" aria-label="Corredor" @change="load">
        <option value="">Todos los corredores</option>
        <option v-for="(name, id) in names" :key="id" :value="id">{{ name }}</option>
      </select>
      <select v-model.number="limit" class="a-select w-32" aria-label="Cuántas" @change="load">
        <option :value="100">Últimas 100</option>
        <option :value="200">Últimas 200</option>
        <option :value="1000">Últimas 1000</option>
      </select>
      <button type="button" class="a-btn" :disabled="loading" @click="load">
        <RefreshCw class="size-4" />Actualizar
      </button>
    </PageHeader>

    <template v-if="data">
      <p v-if="!data.policy.enabled" class="a-error mb-4">
        El tiempo por donaciones está desactivado (Evento → Tiempo por donaciones).
      </p>
      <div class="mb-6 grid gap-4 sm:grid-cols-3">
        <StatCard
          label="Sumado en el evento"
          :value="signed(totals.added)"
          :tone="totals.added ? 'good' : 'default'"
        />
        <StatCard
          label="Restado en el evento"
          :value="signed(-totals.removed)"
          :tone="totals.removed ? 'bad' : 'default'"
        />
        <StatCard
          label="Límites"
          :value="shortDuration(data.policy.maxSecondsPerDonation)"
          :hint="`por donación · al día +${shortDuration(data.policy.maxAddedSecondsPerDay)} / −${shortDuration(data.policy.maxRemovedSecondsPerDay)}`"
        />
      </div>

      <section v-if="perRacer.length" class="panel mb-6">
        <div class="overflow-x-auto p-2">
          <table class="a-table">
            <thead>
              <tr>
                <th>Corredor</th>
                <th>Hoy sumado</th>
                <th>Hoy restado</th>
                <th>Donaciones</th>
                <th>Total sumado</th>
                <th>Total restado</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="r in perRacer" :key="r.id">
                <td class="font-semibold text-white">{{ nameOf(r.id) }}</td>
                <td class="num text-success">{{ signed(r.todayAdded) }}</td>
                <td class="num text-[#ff8aa0]">{{ signed(-r.todayRemoved) }}</td>
                <td class="num">{{ r.donations }}</td>
                <td class="num text-success">{{ signed(r.added) }}</td>
                <td class="num text-[#ff8aa0]">{{ signed(-r.removed) }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>

      <section class="panel mb-6">
        <header class="px-4 pt-4">
          <div class="flex flex-wrap items-center justify-between gap-3">
            <h2 class="display text-2xl text-white">Top donadores</h2>
            <label class="flex items-center gap-2 text-sm text-secondary">
              <input
                type="checkbox"
                :checked="data.donorsPublic"
                @change="togglePublic(($event.target as HTMLInputElement).checked)"
              />Mostrar en la web
            </label>
          </div>
          <p class="text-xs text-muted">
            Quién movió más el reloj en todo el evento (suma de tiempo aplicado, sumado y restado).
            Los diamantes de TikTok y los bits de Twitch no se pueden comparar, por eso se ordena
            por tiempo.
          </p>
        </header>
        <div class="overflow-x-auto p-2">
          <table class="a-table">
            <thead>
              <tr>
                <th>#</th>
                <th>Quién</th>
                <th>Donaciones</th>
                <th>Pagó</th>
                <th>Sumó</th>
                <th>Restó</th>
                <th>Corredores</th>
                <th>En la web</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="(d, i) in data.donors"
                :key="`${d.platform}-${d.viewer}`"
                :class="d.hidden && 'opacity-50'"
              >
                <td class="num text-lg">{{ MEDALS[i] ?? i + 1 }}</td>
                <td>
                  <span class="inline-flex items-center gap-1.5 font-semibold text-white">
                    <PlatformIcon :platform="d.platform" />{{ d.viewer }}
                  </span>
                </td>
                <td class="num">{{ d.donations }}</td>
                <td class="num text-secondary">{{ paidText(d) }}</td>
                <td class="num text-success">{{ signed(d.addedSeconds) }}</td>
                <td class="num text-[#ff8aa0]">{{ signed(-d.removedSeconds) }}</td>
                <td class="num">{{ d.racers }}</td>
                <td>
                  <button
                    type="button"
                    class="a-btn a-btn-sm"
                    :aria-label="d.hidden ? `Mostrar a ${d.viewer}` : `Ocultar a ${d.viewer}`"
                    :title="
                      d.hidden ? 'Oculto: pulsa para mostrarlo' : 'Visible: pulsa para ocultarlo'
                    "
                    @click="toggleDonor(d)"
                  >
                    <EyeOff v-if="d.hidden" class="size-3.5" /><Eye v-else class="size-3.5" />
                  </button>
                </td>
              </tr>
              <tr v-if="!data.donors.length">
                <td colspan="8" class="py-6 text-center text-muted">
                  Todavía no hay donadores con nombre.
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>

      <section class="panel">
        <div class="overflow-x-auto p-2">
          <table class="a-table">
            <thead>
              <tr>
                <th>Cuándo</th>
                <th>Corredor</th>
                <th>Donación</th>
                <th>Quién</th>
                <th>Pedido</th>
                <th>Aplicado</th>
                <th>Nota</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="d in data.recent" :key="d.id">
                <td class="num whitespace-nowrap text-xs text-muted">{{ localDateTime(d.ts) }}</td>
                <td class="font-semibold text-white">{{ nameOf(d.racerId) }}</td>
                <td>
                  <span class="inline-flex items-center gap-1.5">
                    <PlatformIcon :platform="d.platform" />{{ donationText(d) }}
                  </span>
                </td>
                <td class="text-xs text-secondary">{{ d.viewer ?? '—' }}</td>
                <td class="num text-muted">{{ signed(d.requestedSeconds) }}</td>
                <td
                  class="num font-semibold"
                  :class="
                    d.appliedSeconds > 0
                      ? 'text-success'
                      : d.appliedSeconds < 0
                        ? 'text-[#ff8aa0]'
                        : 'text-muted'
                  "
                >
                  {{ signed(d.appliedSeconds) }}
                </td>
                <td class="text-xs text-warning">
                  {{ d.limitedBy ? (LIMITS[d.limitedBy] ?? d.limitedBy) : '' }}
                </td>
              </tr>
              <tr v-if="!data.recent.length">
                <td colspan="7" class="py-6 text-center text-muted">Todavía no hay donaciones.</td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>
    </template>
    <p v-else class="text-sm text-muted">Cargando…</p>
  </div>
</template>
