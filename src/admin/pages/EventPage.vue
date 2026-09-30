<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import AgeBadge from '@/components/common/AgeBadge.vue'
import LiveBadge from '@/components/common/LiveBadge.vue'
import { AGE_ORDER } from '@/utils/catalog'
import type { CatalogAge, CatalogObjective } from '@/types/catalog'
import type { EventInfo } from '@/types/event'
import { adminApi } from '../api/AdminApi'
import { isoToLocalInput, localInputToIso, shortDuration } from '../format'
import { messageOf, useToasts } from '../composables/useToasts'
import EventControls from '../components/EventControls.vue'
import PageHeader from '../components/PageHeader.vue'

const toasts = useToasts()
const event = ref<EventInfo | null>(null)
const objectives = ref<CatalogObjective[]>([])
const busy = ref(false)

const form = ref({
  name: '',
  start: '',
  end: '',
  hours: 4,
  minutes: 0,
  reset: '06:00',
  winCondition: '',
  required: [] as string[],
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
    }
  } catch (e) {
    toasts.error(messageOf(e))
  }
}
onMounted(load)

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

      <div class="flex justify-end gap-3 xl:col-span-2">
        <button type="button" class="a-btn" @click="load">Descartar cambios</button>
        <button type="submit" class="a-btn a-btn-primary" :disabled="busy || !valid">
          Guardar evento
        </button>
      </div>
    </form>
    <p v-else class="text-sm text-muted">Cargando…</p>
  </div>
</template>
