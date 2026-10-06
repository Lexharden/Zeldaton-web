<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { Send } from 'lucide-vue-next'
import { adminApi } from '../api/AdminApi'
import { localDateTime } from '../format'
import { messageOf, useToasts } from '../composables/useToasts'
import type { DiscordStatus } from '../types'

/**
 * Discord announcements ("X is live" with the racer's links). The webhook URL lives only in the
 * server's .env; here the organizer sends a test message and switches announcements on or off.
 */
const toasts = useToasts()
const status = ref<DiscordStatus | null>(null)
const busy = ref(false)

async function load() {
  try {
    status.value = await adminApi.discord()
  } catch (e) {
    toasts.error(messageOf(e))
  }
}
onMounted(load)

async function toggle(enabled: boolean) {
  busy.value = true
  try {
    await adminApi.setDiscord(enabled)
    toasts.success(enabled ? 'Avisos a Discord activados.' : 'Avisos a Discord desactivados.')
  } catch (e) {
    toasts.error(messageOf(e))
  } finally {
    busy.value = false
    await load()
  }
}

async function test() {
  busy.value = true
  try {
    await adminApi.testDiscord()
    toasts.success('Mensaje de prueba enviado: revisa el canal.')
  } catch (e) {
    toasts.error(messageOf(e))
  } finally {
    busy.value = false
    await load()
  }
}
</script>

<template>
  <section class="panel xl:col-span-2">
    <div class="space-y-4 p-5">
      <h2 class="hud-label">DISCORD</h2>
      <p class="a-hint">
        Cuando un corredor entra en vivo se publica un mensaje en tu canal con su progreso, un
        enlace a su página en el sitio y a su TikTok / Twitch. Se avisa una vez por directo (tiene
        que mantenerse en vivo 30 s y no se repite en 30 min), y nunca durante el modo ensayo ni con
        el evento sin iniciar.
      </p>

      <template v-if="status">
        <p v-if="!status.configured" class="a-error">
          Falta el webhook. En Discord: Ajustes del canal → Integraciones → Webhooks → Copiar URL;
          ponla en el .env del servidor como DISCORD_WEBHOOK_URL y reconstruye. Es un secreto: no la
          compartas.
        </p>
        <template v-else>
          <div class="flex flex-wrap items-center gap-4">
            <label class="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                :checked="status.enabled"
                :disabled="busy"
                @change="toggle(($event.target as HTMLInputElement).checked)"
              />
              <span class="text-white">Avisar cuando un corredor esté en vivo</span>
            </label>
            <button type="button" class="a-btn" :disabled="busy" @click="test">
              <Send class="size-4" />Enviar mensaje de prueba
            </button>
          </div>
          <p v-if="status.enabled && status.rehearsal" class="a-hint text-warning">
            El modo ensayo está activo: no se enviará nada hasta reiniciar el evento.
          </p>
          <p v-if="status.lastSentAt" class="a-hint">
            Último mensaje enviado: {{ localDateTime(status.lastSentAt) }}.
          </p>
          <p v-if="status.lastError" class="a-error">Último error: {{ status.lastError }}</p>
        </template>
      </template>
      <p v-else class="text-sm text-muted">Cargando…</p>
    </div>
  </section>
</template>
