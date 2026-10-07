<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import AgeBadge from '@/components/common/AgeBadge.vue'
import LiveBadge from '@/components/common/LiveBadge.vue'
import { AGE_ORDER } from '@/utils/catalog'
import type { CatalogAge, CatalogObjective } from '@/types/catalog'
import type { DonationTimePolicy, EventInfo } from '@/types/event'
import { adminApi } from '../api/AdminApi'
import { isoToLocalInput, localInputToIso, shortDuration } from '../format'
import { messageOf, useToasts } from '../composables/useToasts'
import DiscordCard from '../components/DiscordCard.vue'
import EventControls from '../components/EventControls.vue'
import ResetEventDialog from '../components/ResetEventDialog.vue'
import PageHeader from '../components/PageHeader.vue'

const toasts = useToasts()
const event = ref<EventInfo | null>(null)
const objectives = ref<CatalogObjective[]>([])
const busy = ref(false)
const resetOpen = ref(false)

/** Same defaults as the backend (DonationTimePolicy::default). */
const DEFAULT_DONATIONS: DonationTimePolicy = {
  enabled: true,
  allowAdd: true,
  allowRemove: true,
  maxSecondsPerDonation: 3600,
  maxAddedSecondsPerDay: 4 * 3600,
  maxRemovedSecondsPerDay: 4 * 3600,
}
/** The form edits minutes; the API speaks seconds. */
const donations = ref({
  enabled: true,
  allowAdd: true,
  allowRemove: true,
  perDonationMin: 60,
  addedPerDayMin: 240,
  removedPerDayMin: 240,
  // The server computes a donation's time from these (HiveShock only picks add or remove).
  // Unchecked = the number HiveShock sends is used as it is.
  useDiamondRate: true,
  secondsPerDiamond: 3,
  useBitRate: false,
  secondsPerBit: 1,
})
function loadDonations(p: DonationTimePolicy = DEFAULT_DONATIONS) {
  donations.value = {
    enabled: p.enabled,
    allowAdd: p.allowAdd,
    allowRemove: p.allowRemove,
    perDonationMin: Math.round(p.maxSecondsPerDonation / 60),
    addedPerDayMin: Math.round(p.maxAddedSecondsPerDay / 60),
    removedPerDayMin: Math.round(p.maxRemovedSecondsPerDay / 60),
    // An older server sends no rate fields: the server default is 3 s per diamond.
    useDiamondRate: p.secondsPerDiamond === undefined ? true : p.secondsPerDiamond !== null,
    secondsPerDiamond: p.secondsPerDiamond ?? 3,
    useBitRate: p.secondsPerBit !== undefined && p.secondsPerBit !== null,
    secondsPerBit: p.secondsPerBit ?? 1,
  }
}
const donationPolicy = computed<DonationTimePolicy>(() => ({
  enabled: donations.value.enabled,
  allowAdd: donations.value.allowAdd,
  allowRemove: donations.value.allowRemove,
  maxSecondsPerDonation: Math.round(donations.value.perDonationMin * 60),
  maxAddedSecondsPerDay: Math.round(donations.value.addedPerDayMin * 60),
  maxRemovedSecondsPerDay: Math.round(donations.value.removedPerDayMin * 60),
  secondsPerDiamond: donations.value.useDiamondRate
    ? Math.round(donations.value.secondsPerDiamond)
    : null,
  secondsPerBit: donations.value.useBitRate ? Math.round(donations.value.secondsPerBit) : null,
}))
const donationsError = computed(() => {
  const p = donationPolicy.value
  if (!(p.maxSecondsPerDonation >= 60 && p.maxSecondsPerDonation <= 172800))
    return 'El máximo por donación va de 1 minuto a 48 horas.'
  if (
    !(p.maxAddedSecondsPerDay >= 0 && p.maxAddedSecondsPerDay <= 172800) ||
    !(p.maxRemovedSecondsPerDay >= 0 && p.maxRemovedSecondsPerDay <= 172800)
  )
    return 'Los topes diarios van de 0 a 48 horas.'
  for (const [use, v] of [
    [donations.value.useDiamondRate, donations.value.secondsPerDiamond],
    [donations.value.useBitRate, donations.value.secondsPerBit],
  ] as const)
    if (use && !(Number.isInteger(v) && v >= 1 && v <= 3600))
      return 'La tarifa va de 1 a 3600 segundos por unidad (número entero).'
  return ''
})

const form = ref({
  name: '',
  start: '',
  end: '',
  hours: 4,
  minutes: 0,
  reset: '06:00',
  winCondition: '',
  required: [] as string[],
  rehearsal: false,
})

async function load() {
  try {
    const [overview, catalog] = await Promise.all([adminApi.overview(), adminApi.catalog()])
    event.value = overview.event
    objectives.value = catalog.objectives.filter((o) => o.enabled)
    const e = overview.event
    form.value = {
      name: e.name,
      start: isoToLocalInput(e.startAtUtc),
      end: isoToLocalInput(e.endAtUtc),
      hours: Math.floor(e.dailyBudgetSeconds / 3600),
      minutes: Math.floor((e.dailyBudgetSeconds % 3600) / 60),
      reset: e.dailyResetLocalTime,
      winCondition: e.rules.winCondition,
      required: [...e.rules.requiredObjectiveIds],
      rehearsal: !!e.rehearsal,
    }
    loadDonations(e.donationTime)
  } catch (e) {
    toasts.error(messageOf(e))
  }
}
onMounted(load)

/** A real event that is running can never be reset; a rehearsal can, at any moment. */
const canReset = computed(
  () => !!event.value && (event.value.rehearsal || event.value.status !== 'live'),
)

function onReset(next: EventInfo) {
  resetOpen.value = false
  event.value = next
  void load()
}

const budgetSeconds = computed(() => form.value.hours * 3600 + form.value.minutes * 60)
const budgetError = computed(() =>
  budgetSeconds.value < 60 || budgetSeconds.value > 86400 ? 'Entre 1 minuto y 24 horas.' : '',
)
const sections = computed(() =>
  AGE_ORDER.flatMap((age: CatalogAge) => {
    const list = objectives.value
      .filter((o) => o.age === age)
      .sort((a, b) => a.sortOrder - b.sortOrder)
    return list.length ? [{ age, list }] : []
  }),
)
const valid = computed(
  () =>
    !budgetError.value &&
    !donationsError.value &&
    form.value.name.trim() &&
    form.value.required.length > 0 &&
    /^\d{2}:\d{2}$/.test(form.value.reset),
)

async function save() {
  busy.value = true
  try {
    event.value = await adminApi.updateEvent({
      name: form.value.name.trim(),
      startAtUtc: localInputToIso(form.value.start),
      endAtUtc: localInputToIso(form.value.end),
      dailyBudgetSeconds: budgetSeconds.value,
      dailyResetLocalTime: form.value.reset,
      winCondition: form.value.winCondition.trim(),
      requiredObjectiveIds: form.value.required,
      donationTime: donationPolicy.value,
      rehearsal: form.value.rehearsal,
    })
    toasts.success('Evento guardado.')
  } catch (e) {
    toasts.error(messageOf(e))
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <div>
    <PageHeader title="Evento" subtitle="Fechas, tiempo diario y reglas de la carrera.">
      <LiveBadge v-if="event" :status="event.status" />
      <span
        v-if="event?.rehearsal"
        class="hud-label rounded border border-warning/60 px-2 py-1 text-warning"
        >ENSAYO</span
      >
      <EventControls v-if="event" :status="event.status" @changed="load" />
    </PageHeader>

    <form v-if="event" class="grid gap-6 xl:grid-cols-2" @submit.prevent="save">
      <section class="panel">
        <div class="space-y-4 p-5">
          <h2 class="hud-label">GENERAL</h2>
          <div>
            <label class="a-label" for="ev-name">Nombre</label>
            <input id="ev-name" v-model="form.name" class="a-input" required />
          </div>
          <div class="grid gap-4 sm:grid-cols-2">
            <div>
              <label class="a-label" for="ev-start">Inicio (tu hora local)</label>
              <input id="ev-start" v-model="form.start" type="datetime-local" class="a-input" />
              <p class="a-hint">UTC: {{ event.startAtUtc }}</p>
            </div>
            <div>
              <label class="a-label" for="ev-end">Fin (opcional)</label>
              <input id="ev-end" v-model="form.end" type="datetime-local" class="a-input" />
            </div>
          </div>
          <div>
            <label class="a-label" for="ev-win">Condición de victoria</label>
            <textarea id="ev-win" v-model="form.winCondition" class="a-textarea" />
          </div>
        </div>
      </section>

      <section class="panel">
        <div class="space-y-4 p-5">
          <h2 class="hud-label">TIEMPO DIARIO</h2>
          <div class="flex items-end gap-3">
            <div class="w-24">
              <label class="a-label" for="ev-h">Horas</label>
              <input
                id="ev-h"
                v-model.number="form.hours"
                type="number"
                min="0"
                max="24"
                class="a-input"
              />
            </div>
            <div class="w-24">
              <label class="a-label" for="ev-m">Minutos</label>
              <input
                id="ev-m"
                v-model.number="form.minutes"
                type="number"
                min="0"
                max="59"
                class="a-input"
              />
            </div>
            <div class="w-32">
              <label class="a-label" for="ev-reset">Reinicio (hora local del corredor)</label>
              <input id="ev-reset" v-model="form.reset" type="time" class="a-input" />
            </div>
          </div>
          <p v-if="budgetError" class="a-error">{{ budgetError }}</p>
          <p v-else class="a-hint">
            Cada corredor juega hasta {{ shortDuration(budgetSeconds) }} al día; se reinicia a las
            {{ form.reset }} de su zona horaria.
          </p>
        </div>
      </section>

      <section class="panel xl:col-span-2">
        <div class="space-y-4 p-5">
          <h2 class="hud-label">TIEMPO POR DONACIONES</h2>
          <p class="a-hint">
            Cada streamer decide en HiveShock cuántos segundos vale cada diamante de TikTok o bit de
            Twitch y si suman o restan tiempo. Aquí pones los límites: el servidor nunca aplica más.
          </p>
          <div class="flex flex-wrap gap-6 text-sm">
            <label class="flex items-center gap-2">
              <input v-model="donations.enabled" type="checkbox" />
              <span class="text-white">Permitir tiempo por donaciones</span>
            </label>
            <label class="flex items-center gap-2">
              <input v-model="donations.allowAdd" type="checkbox" :disabled="!donations.enabled" />
              <span>Pueden sumar tiempo</span>
            </label>
            <label class="flex items-center gap-2">
              <input
                v-model="donations.allowRemove"
                type="checkbox"
                :disabled="!donations.enabled"
              />
              <span>Pueden restar tiempo</span>
            </label>
          </div>
          <div class="grid gap-4 sm:grid-cols-3">
            <div>
              <label class="a-label" for="don-per">Máximo por donación (min)</label>
              <input
                id="don-per"
                v-model.number="donations.perDonationMin"
                type="number"
                min="1"
                max="2880"
                class="a-input"
                :disabled="!donations.enabled"
              />
            </div>
            <div>
              <label class="a-label" for="don-add">Máximo que suman al día (min)</label>
              <input
                id="don-add"
                v-model.number="donations.addedPerDayMin"
                type="number"
                min="0"
                max="2880"
                class="a-input"
                :disabled="!donations.enabled || !donations.allowAdd"
              />
            </div>
            <div>
              <label class="a-label" for="don-remove">Máximo que restan al día (min)</label>
              <input
                id="don-remove"
                v-model.number="donations.removedPerDayMin"
                type="number"
                min="0"
                max="2880"
                class="a-input"
                :disabled="!donations.enabled || !donations.allowRemove"
              />
            </div>
          </div>
          <div class="grid gap-4 border-t border-line pt-4 sm:grid-cols-2">
            <div>
              <label class="flex items-center gap-2 text-sm">
                <input
                  v-model="donations.useDiamondRate"
                  type="checkbox"
                  :disabled="!donations.enabled"
                />
                <span class="text-white">Tarifa fija por diamante de TikTok</span>
              </label>
              <label class="a-label mt-2" for="don-diamond">Segundos por diamante</label>
              <input
                id="don-diamond"
                v-model.number="donations.secondsPerDiamond"
                type="number"
                min="1"
                max="3600"
                class="a-input"
                :disabled="!donations.enabled || !donations.useDiamondRate"
              />
            </div>
            <div>
              <label class="flex items-center gap-2 text-sm">
                <input
                  v-model="donations.useBitRate"
                  type="checkbox"
                  :disabled="!donations.enabled"
                />
                <span class="text-white">Tarifa fija por bit de Twitch</span>
              </label>
              <label class="a-label mt-2" for="don-bit">Segundos por bit</label>
              <input
                id="don-bit"
                v-model.number="donations.secondsPerBit"
                type="number"
                min="1"
                max="3600"
                class="a-input"
                :disabled="!donations.enabled || !donations.useBitRate"
              />
            </div>
            <p class="a-hint sm:col-span-2">
              Con la tarifa fija, <strong>el servidor calcula el tiempo</strong> (cantidad ×
              segundos) y a HiveShock solo le toma si suma o resta, así todos los corredores valen
              igual aunque su HiveShock tenga otra tarifa. Sin la casilla, se usa el número que
              mande HiveShock (siempre limitado por los topes).
            </p>
          </div>
          <p v-if="donationsError" class="a-error">{{ donationsError }}</p>
          <p v-else-if="donations.enabled" class="a-hint">
            Una donación cambia como mucho
            {{ shortDuration(donationPolicy.maxSecondsPerDonation) }}; al día, por corredor, suman
            hasta {{ shortDuration(donationPolicy.maxAddedSecondsPerDay) }} y restan hasta
            {{ shortDuration(donationPolicy.maxRemovedSecondsPerDay) }}. Los topes diarios se
            reinician con el tiempo del día. Lo aplicado queda en «Donaciones».
          </p>
          <p v-else class="a-hint">Las donaciones no cambian el tiempo de nadie.</p>
        </div>
      </section>

      <section class="panel xl:col-span-2">
        <div class="space-y-4 p-5">
          <h2 class="hud-label">OBJETIVOS QUE HAY QUE COMPLETAR PARA TERMINAR</h2>
          <div class="grid gap-6 sm:grid-cols-2">
            <div v-for="s in sections" :key="s.age">
              <AgeBadge :age="s.age" class="mb-2" />
              <label v-for="o in s.list" :key="o.id" class="mb-1.5 flex items-center gap-2 text-sm">
                <input v-model="form.required" type="checkbox" :value="o.id" />
                <span class="text-white">{{ o.nameEs }}</span>
                <span class="text-xs text-muted">{{ o.id }}</span>
              </label>
            </div>
          </div>
          <p v-if="!form.required.length" class="a-error">Elige al menos un objetivo.</p>
        </div>
      </section>

      <section class="panel xl:col-span-2">
        <div class="space-y-4 p-5">
          <h2 class="hud-label">ENSAYO Y REINICIO</h2>
          <label class="flex items-start gap-2 text-sm">
            <input v-model="form.rehearsal" type="checkbox" class="mt-1" />
            <span
              ><span class="text-white">Modo ensayo</span><br /><span class="a-hint"
                >Los corredores prueban con HiveShock de verdad. La web muestra un aviso de que son
                datos de prueba y, mientras esté activo, el evento se puede reiniciar aunque esté en
                vivo. Se apaga al reiniciar para el evento real. No se puede activar con el evento
                en vivo: pausa primero. Se aplica al guardar.</span
              ></span
            >
          </label>
          <div class="flex flex-wrap items-center gap-3 border-t border-line pt-4">
            <button
              type="button"
              class="a-btn a-btn-danger"
              :disabled="!canReset"
              @click="resetOpen = true"
            >
              Reiniciar evento…
            </button>
            <p v-if="canReset" class="a-hint">
              Borra todo lo de las pruebas y deja corredores, tokens y catálogo para empezar de
              cero.
            </p>
            <p v-else class="a-hint">
              Un evento real en vivo no se puede reiniciar. Pausa o finaliza el evento primero.
            </p>
          </div>
        </div>
      </section>

      <DiscordCard />

      <div class="flex justify-end gap-3 xl:col-span-2">
        <button type="button" class="a-btn" @click="load">Descartar cambios</button>
        <button type="submit" class="a-btn a-btn-primary" :disabled="busy || !valid">
          Guardar evento
        </button>
      </div>
    </form>
    <p v-else class="text-sm text-muted">Cargando…</p>

    <ResetEventDialog
      v-if="event"
      :open="resetOpen"
      :event="event"
      @close="resetOpen = false"
      @done="onReset"
    />
  </div>
</template>
