<script setup lang="ts">
import { computed, ref } from 'vue'
import { Flag, Pause, Play, Rocket } from 'lucide-vue-next'
import { adminApi } from '../api/AdminApi'
import { confirm } from '../composables/useConfirm'
import { messageOf, useToasts } from '../composables/useToasts'
import type { EventAction } from '../types'
import type { EventStatus } from '@/types/event'

/** Start / pause / resume / finish the whole event, always behind a confirmation. Admin only. */
const props = defineProps<{ status: EventStatus }>()
const emit = defineEmits<{ changed: [] }>()
const toasts = useToasts()
const busy = ref(false)

const actions = computed(() => {
  const s = props.status
  return [
    {
      id: 'start' as const,
      label: 'Iniciar evento',
      icon: Rocket,
      show: s === 'upcoming' || s === 'paused',
      tone: 'a-btn-gold',
    },
    { id: 'pause' as const, label: 'Pausar evento', icon: Pause, show: s === 'live', tone: '' },
    {
      id: 'resume' as const,
      label: 'Reanudar',
      icon: Play,
      show: s === 'paused',
      tone: 'a-btn-primary',
    },
    {
      id: 'finish' as const,
      label: 'Terminar evento',
      icon: Flag,
      show: s !== 'finished',
      tone: 'a-btn-danger',
    },
  ].filter((a) => a.show)
})

const TEXT: Record<
  EventAction,
  { title: string; message: string; done: string; danger?: boolean }
> = {
  start: {
    title: 'Iniciar el evento',
    message:
      'Los corredores podrán empezar a jugar y sus relojes del día empezarán a contar. Si la hora de inicio aún no llega, se adelanta a ahora.',
    done: 'Evento iniciado.',
  },
  pause: {
    title: 'Pausar el evento',
    message:
      'Nadie podrá iniciar sesiones nuevas mientras esté en pausa. Puedes reanudarlo cuando quieras.',
    done: 'Evento en pausa.',
  },
  resume: {
    title: 'Reanudar el evento',
    message: 'Los corredores podrán volver a jugar.',
    done: 'Evento reanudado.',
  },
  finish: {
    title: 'Terminar el evento',
    message: 'Esto cierra la carrera y no se puede deshacer desde el panel.',
    done: 'Evento terminado.',
    danger: true,
  },
}

async function run(action: EventAction) {
  const text = TEXT[action]
  const ok = await confirm({
    title: text.title,
    message: text.message,
    confirmLabel: text.title,
    danger: text.danger,
    requireText: action === 'finish' ? 'TERMINAR' : undefined,
  })
  if (!ok) return
  busy.value = true
  try {
    await adminApi.eventAction(action)
    toasts.success(text.done)
    emit('changed')
  } catch (e) {
    toasts.error(messageOf(e))
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <div class="flex flex-wrap gap-2">
    <button
      v-for="a in actions"
      :key="a.id"
      type="button"
      class="a-btn"
      :class="a.tone"
      :disabled="busy"
      @click="run(a.id)"
    >
      <component :is="a.icon" class="size-4" aria-hidden="true" />{{ a.label }}
    </button>
  </div>
</template>
