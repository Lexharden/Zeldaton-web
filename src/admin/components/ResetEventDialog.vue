<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type { EventInfo } from '@/types/event'
import { adminApi } from '../api/AdminApi'
import { isoToLocalInput, localInputToIso } from '../format'
import { messageOf, useToasts } from '../composables/useToasts'
import Modal from './Modal.vue'

/**
 * Wipes the test run so the real event can start clean. Racers, their HiveShock tokens, the catalog,
 * pictures and accounts are kept. The organizer must type the confirmation word.
 */
const props = defineProps<{ open: boolean; event: EventInfo }>()
const emit = defineEmits<{ close: []; done: [EventInfo] }>()

const WORD = 'REINICIAR'
const toasts = useToasts()
const start = ref('')
const leaveRehearsal = ref(true)
const typed = ref('')
const busy = ref(false)

watch(
  () => props.open,
  (open) => {
    if (!open) return
    typed.value = ''
    leaveRehearsal.value = true
    // Keep the current start only when it is still ahead; otherwise the organizer must pick one.
    const current = new Date(props.event.startAtUtc).getTime()
    start.value = current > Date.now() ? isoToLocalInput(props.event.startAtUtc) : ''
  },
  { immediate: true },
)

const startIso = computed(() => localInputToIso(start.value))
const problem = computed(() => {
  if (!startIso.value) return 'Elige la fecha y hora de inicio del evento.'
  if (new Date(startIso.value).getTime() <= Date.now()) return 'El inicio debe ser en el futuro.'
  return ''
})
const ready = computed(() => !problem.value && typed.value.trim() === WORD)

async function submit() {
  if (!ready.value) return
  busy.value = true
  try {
    const event = await adminApi.resetEvent({
      confirm: typed.value.trim(),
      startAtUtc: startIso.value,
      leaveRehearsal: leaveRehearsal.value,
    })
    toasts.success('Evento reiniciado. Todo listo para empezar de cero.')
    emit('done', event)
  } catch (e) {
    toasts.error(messageOf(e))
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <Modal :open="open" title="Reiniciar evento" @close="emit('close')">
    <form class="space-y-4" @submit.prevent="submit">
      <div class="text-sm text-secondary">
        <p class="mb-2 font-semibold text-white">Se borra (no se puede deshacer):</p>
        <ul class="list-disc space-y-0.5 pl-5">
          <li>Progreso, ítems, objetivos, estadísticas y tiempo de cada corredor</li>
          <li>Donaciones y el ranking de donadores</li>
          <li>La actividad en vivo y el ganador</li>
        </ul>
        <p class="mb-2 mt-3 font-semibold text-white">Se conserva:</p>
        <ul class="list-disc space-y-0.5 pl-5">
          <li>Corredores, canales y sus tokens de HiveShock (no hay que reconfigurar nada)</li>
          <li>Catálogo, imágenes subidas, reglas, cuentas y la auditoría</li>
        </ul>
      </div>

      <div>
        <label class="a-label" for="reset-start">Inicio del evento (tu hora local)</label>
        <input id="reset-start" v-model="start" type="datetime-local" class="a-input" />
        <p v-if="problem" class="a-error mt-1">{{ problem }}</p>
        <p v-else class="a-hint">El evento queda como «próximamente» hasta esa hora.</p>
      </div>

      <label v-if="event.rehearsal" class="flex items-center gap-2 text-sm"
        ><input v-model="leaveRehearsal" type="checkbox" />Terminar el modo ensayo (quita el aviso
        de la web)</label
      >

      <div>
        <label class="a-label" for="reset-word">Escribe {{ WORD }} para confirmar</label>
        <input
          id="reset-word"
          v-model="typed"
          class="a-input"
          autocomplete="off"
          :placeholder="WORD"
        />
      </div>

      <div class="flex justify-end gap-3">
        <button type="button" class="a-btn" @click="emit('close')">Cancelar</button>
        <button type="submit" class="a-btn a-btn-danger" :disabled="busy || !ready">
          Reiniciar evento
        </button>
      </div>
    </form>
  </Modal>
</template>
