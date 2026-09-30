<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { adminApi } from '../api/AdminApi'
import { confirm } from '../composables/useConfirm'
import { messageOf, useToasts } from '../composables/useToasts'
import { useAdminStore } from '../stores/admin'
import type { RacerAction } from '../types'
import Modal from './Modal.vue'

/** Everything an organizer can do to one racer during the event, with a reason where it matters. */
const props = defineProps<{ open: boolean; racerId: string; name: string; status: string }>()
const emit = defineEmits<{ close: []; done: [] }>()
const admin = useAdminStore()
const toasts = useToasts()

const busy = ref(false)
const minutes = ref(10)
const direction = ref<1 | -1>(-1)
const reason = ref('')
const finalTime = ref('')

watch(
  () => props.open,
  (open) => {
    if (open) {
      minutes.value = 10
      direction.value = -1
      reason.value = ''
      finalTime.value = ''
    }
  },
)

const canPause = computed(() => props.status === 'live')
const canResume = computed(() => props.status === 'paused' || props.status === 'online')
const reasonOk = computed(() => reason.value.trim().length >= 3)

async function run(
  action: RacerAction,
  body = {},
  ask?: { title: string; message: string; danger?: boolean },
) {
  if (ask && !(await confirm({ ...ask, confirmLabel: ask.title }))) return
  busy.value = true
  try {
    await adminApi.racerAction(props.racerId, action, body)
    toasts.success('Hecho.')
    emit('done')
    emit('close')
  } catch (e) {
    toasts.error(messageOf(e))
  } finally {
    busy.value = false
  }
}

const adjust = () =>
  run('adjust-time', {
    deltaSeconds: direction.value * Math.round(minutes.value * 60),
    reason: reason.value.trim(),
  })

function finish() {
  const t = finalTime.value.trim()
  let seconds: number | undefined
  if (t) {
    const [h = '0', m = '0'] = t.split(':')
    seconds = Number(h) * 3600 + Number(m) * 60
  }
  return run(
    'finish',
    { finalTimeSeconds: seconds, reason: reason.value.trim() },
    {
      title: 'Marcar como terminado',
      message: `${props.name} quedará como finalista${seconds ? ` con ${finalTime.value} (h:mm)` : ''}. Esto decide el ranking.`,
      danger: true,
    },
  )
}
</script>

<template>
  <Modal :open="open" :title="`Controlar a ${name}`" @close="emit('close')">
    <div class="space-y-6">
      <section>
        <h3 class="hud-label mb-2">SESIÓN</h3>
        <div class="flex flex-wrap gap-2">
          <button type="button" class="a-btn" :disabled="busy || !canPause" @click="run('pause')">
            Pausar
          </button>
          <button
            type="button"
            class="a-btn a-btn-primary"
            :disabled="busy || !canResume"
            @click="run('resume')"
          >
            Reanudar
          </button>
          <button
            type="button"
            class="a-btn a-btn-danger"
            :disabled="busy"
            @click="
              run(
                'force-close',
                {},
                {
                  title: 'Cerrar el juego',
                  message: `HiveShock de ${name} cerrará el juego de inmediato.`,
                  danger: true,
                },
              )
            "
          >
            Cerrar el juego
          </button>
          <button
            type="button"
            class="a-btn"
            :disabled="busy"
            @click="
              run(
                'reset-day',
                {},
                {
                  title: 'Reiniciar el día',
                  message: `${name} recupera el tiempo completo de hoy.`,
                },
              )
            "
          >
            Reiniciar el día
          </button>
        </div>
      </section>

      <section class="space-y-3">
        <h3 class="hud-label">AJUSTAR TIEMPO / TERMINAR</h3>
        <div>
          <label class="a-label" for="rc-reason">Motivo (obligatorio, queda en la auditoría)</label>
          <input
            id="rc-reason"
            v-model="reason"
            class="a-input"
            placeholder="p. ej. fallo de conexión de 10 min"
          />
        </div>
        <div class="flex flex-wrap items-end gap-3">
          <div class="w-28">
            <label class="a-label" for="rc-min">Minutos</label>
            <input
              id="rc-min"
              v-model.number="minutes"
              type="number"
              min="1"
              max="600"
              class="a-input"
            />
          </div>
          <select v-model.number="direction" class="a-select w-44" aria-label="Sumar o restar">
            <option :value="1">Devolver tiempo (+)</option>
            <option :value="-1">Quitar tiempo (−)</option>
          </select>
          <button
            type="button"
            class="a-btn"
            :disabled="busy || !reasonOk || !(minutes > 0)"
            @click="adjust"
          >
            Aplicar ajuste
          </button>
        </div>
        <div v-if="admin.isAdmin" class="flex flex-wrap items-end gap-3 border-t border-line pt-3">
          <div class="w-40">
            <label class="a-label" for="rc-final">Tiempo final (h:mm)</label>
            <input id="rc-final" v-model="finalTime" class="a-input" placeholder="opcional" />
          </div>
          <button
            type="button"
            class="a-btn a-btn-danger"
            :disabled="busy || !reasonOk"
            @click="finish"
          >
            Marcar como terminado
          </button>
        </div>
      </section>
    </div>
  </Modal>
</template>
