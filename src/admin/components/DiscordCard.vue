<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { Send } from 'lucide-vue-next'
import { adminApi } from '../api/AdminApi'
import { localDateTime } from '../format'
import { messageOf, useToasts } from '../composables/useToasts'
import type {
  DiscordChannelName,
  DiscordKind,
  DiscordPatch,
  DiscordStatus,
  DiscordThresholds,
} from '../types'

/**
 * Discord notifications: two channels (the community and the referees), each with its own webhook
 * (kept in the server's .env, never shown here), a switch, a test button and a list of the notices
 * it receives. The kinds come from the server, so a new notice shows up here by itself.
 */
const toasts = useToasts()
const status = ref<DiscordStatus | null>(null)
const busy = ref(false)

const CHANNELS: {
  id: DiscordChannelName
  title: string
  who: string
  env: string
  flag: 'publicEnabled' | 'staffEnabled'
}[] = [
  {
    id: 'public',
    title: 'Comunidad',
    who: 'El canal público: avisos para quien sigue la carrera. Solo suena con el evento en marcha (nunca en modo ensayo).',
    env: 'DISCORD_WEBHOOK_URL',
    flag: 'publicEnabled',
  },
  {
    id: 'staff',
    title: 'Árbitros',
    who: 'El canal privado del equipo: alertas y acciones de organizadores. También funciona en modo ensayo (con la etiqueta [ENSAYO]).',
    env: 'DISCORD_STAFF_WEBHOOK_URL',
    flag: 'staffEnabled',
  },
]

async function load() {
  try {
    status.value = await adminApi.discord()
  } catch (e) {
    toasts.error(messageOf(e))
  }
}
onMounted(load)

async function apply(patch: DiscordPatch, done: string) {
  busy.value = true
  try {
    status.value = await adminApi.setDiscord(patch)
    toasts.success(done)
  } catch (e) {
    toasts.error(messageOf(e))
    await load()
  } finally {
    busy.value = false
  }
}

async function test(channel: DiscordChannelName) {
  busy.value = true
  try {
    await adminApi.testDiscord(channel)
    toasts.success('Mensaje de prueba enviado: revisa el canal.')
  } catch (e) {
    toasts.error(messageOf(e))
  } finally {
    busy.value = false
    await load()
  }
}

const kindsOf = (channel: DiscordChannelName): DiscordKind[] =>
  status.value?.kinds.filter((k) => k.audience === channel) ?? []

const FIELDS: {
  key: keyof DiscordThresholds
  label: string
  min: number
  max: number
  step: number
  hint: string
}[] = [
  {
    key: 'disconnectMinutes',
    label: 'Desconexión (min)',
    min: 1,
    max: 120,
    step: 1,
    hint: 'Avisa si lleva así de desconectado.',
  },
  {
    key: 'lowTimeMinutes',
    label: 'Poco tiempo (min)',
    min: 1,
    max: 240,
    step: 1,
    hint: 'Avisa cuando le quedan estos minutos.',
  },
  {
    key: 'jumpPercent',
    label: 'Salto de progreso (puntos %)',
    min: 5,
    max: 100,
    step: 1,
    hint: 'Una subida de al menos esto…',
  },
  {
    key: 'jumpWindowSeconds',
    label: 'En menos de (segundos)',
    min: 10,
    max: 3600,
    step: 10,
    hint: '…dentro de este tiempo es sospechosa.',
  },
  {
    key: 'noShowMinutes',
    label: 'No aparece (min)',
    min: 1,
    max: 120,
    step: 1,
    hint: 'Avisa si no hay señal este rato después de la hora de su live.',
  },
  {
    key: 'uncoveredLeadMinutes',
    label: 'Sin árbitro (min antes)',
    min: 5,
    max: 720,
    step: 5,
    hint: 'Avisa si el live empieza pronto y nadie lo arbitra.',
  },
]
const draft = ref<Partial<Record<keyof DiscordThresholds, number>>>({})
const dirty = computed(() =>
  FIELDS.some(
    (f) =>
      draft.value[f.key] !== undefined && draft.value[f.key] !== status.value?.thresholds[f.key],
  ),
)
async function saveThresholds() {
  const patch: Partial<DiscordThresholds> = {}
  for (const f of FIELDS) {
    const v = draft.value[f.key]
    if (v !== undefined && v !== status.value?.thresholds[f.key]) patch[f.key] = Number(v)
  }
  await apply({ thresholds: patch }, 'Umbrales guardados.')
  draft.value = {}
}
</script>

<template>
  <section class="panel xl:col-span-2">
    <div class="space-y-5 p-5">
      <h2 class="hud-label">NOTIFICACIONES DE DISCORD</h2>

      <template v-if="status">
        <p v-if="status.rehearsal" class="a-hint text-warning">
          Modo ensayo activo: el canal de la comunidad no recibe nada hasta reiniciar el evento.
        </p>

        <div class="grid gap-6 lg:grid-cols-2">
          <div v-for="ch in CHANNELS" :key="ch.id" class="space-y-3 rounded border border-line p-4">
            <h3 class="display text-2xl text-white">{{ ch.title }}</h3>
            <p class="a-hint">{{ ch.who }}</p>

            <p v-if="!status.channels[ch.id].configured" class="a-error">
              Falta el webhook. En Discord: Ajustes del canal → Integraciones → Webhooks → Copiar
              URL; ponla en el .env del servidor como {{ ch.env }} y reconstruye. Es un secreto: no
              la compartas.
            </p>
            <template v-else>
              <div class="flex flex-wrap items-center gap-4">
                <label class="flex items-center gap-2 text-sm">
                  <input
                    type="checkbox"
                    :checked="status.channels[ch.id].enabled"
                    :disabled="busy"
                    @change="
                      apply(
                        { [ch.flag]: ($event.target as HTMLInputElement).checked },
                        ($event.target as HTMLInputElement).checked
                          ? `Canal «${ch.title}» activado.`
                          : `Canal «${ch.title}» desactivado.`,
                      )
                    "
                  />
                  <span class="text-white">Canal activo</span>
                </label>
                <button type="button" class="a-btn" :disabled="busy" @click="test(ch.id)">
                  <Send class="size-4" />Enviar prueba
                </button>
              </div>

              <ul class="space-y-1.5">
                <li v-for="k in kindsOf(ch.id)" :key="k.kind">
                  <label class="flex items-start gap-2 text-sm">
                    <input
                      type="checkbox"
                      class="mt-0.5"
                      :checked="k.enabled"
                      :disabled="busy"
                      @change="
                        apply(
                          { kinds: { [k.kind]: ($event.target as HTMLInputElement).checked } },
                          'Guardado.',
                        )
                      "
                    />
                    <span :class="k.enabled ? 'text-white' : 'text-muted'">
                      {{ k.label }}
                      <span
                        v-if="k.critical && status.channels.staff.mentionsRole"
                        class="ml-1 text-xs text-warning"
                        title="Menciona al rol de árbitros"
                        >@rol</span
                      >
                    </span>
                  </label>
                </li>
              </ul>

              <p v-if="status.channels[ch.id].lastSentAt" class="a-hint">
                Último mensaje: {{ localDateTime(status.channels[ch.id].lastSentAt!) }}.
              </p>
              <p v-if="status.channels[ch.id].queued" class="a-hint">
                {{ status.channels[ch.id].queued }} en cola.
              </p>
              <p v-if="status.channels[ch.id].dropped" class="a-hint text-warning">
                {{ status.channels[ch.id].dropped }} avisos descartados por exceso de mensajes.
              </p>
              <p v-if="status.channels[ch.id].lastError" class="a-error">
                Último error: {{ status.channels[ch.id].lastError }}
              </p>
            </template>
          </div>
        </div>

        <div class="space-y-3 rounded border border-line p-4">
          <h3 class="display text-2xl text-white">Umbrales de las alertas</h3>
          <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
            <div v-for="f in FIELDS" :key="f.key">
              <label class="a-label" :for="`th-${f.key}`">{{ f.label }}</label>
              <input
                :id="`th-${f.key}`"
                type="number"
                class="a-input"
                :min="f.min"
                :max="f.max"
                :step="f.step"
                :value="draft[f.key] ?? status.thresholds[f.key]"
                @input="draft[f.key] = Number(($event.target as HTMLInputElement).value)"
              />
              <p class="a-hint">{{ f.hint }}</p>
            </div>
          </div>
          <button
            type="button"
            class="a-btn a-btn-primary"
            :disabled="busy || !dirty"
            @click="saveThresholds"
          >
            Guardar umbrales
          </button>
        </div>
      </template>
      <p v-else class="text-sm text-muted">Cargando…</p>
    </div>
  </section>
</template>
