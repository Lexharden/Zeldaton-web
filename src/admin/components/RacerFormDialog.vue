<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { adminApi } from '../api/AdminApi'
import { messageOf, useToasts } from '../composables/useToasts'
import type { ChannelInput, NewRacerInput } from '../types'
import type { Racer } from '@/types/racer'
import Modal from './Modal.vue'

/** Create (no `racer`) or edit a racer. On create the server returns the ingest token, shown once by the parent. */
const props = defineProps<{ open: boolean; racer?: Racer | null }>()
const emit = defineEmits<{ close: []; saved: [token?: string] }>()
const toasts = useToasts()
const editing = computed(() => !!props.racer)

const form = ref({
  id: '',
  displayName: '',
  timezone: 'America/Mexico_City',
  country: '',
  avatarUrl: '',
  twitch: '',
  tiktok: '',
  youtube: '',
})
const busy = ref(false)

const zones = (() => {
  try {
    return (
      (Intl as unknown as { supportedValuesOf?: (k: string) => string[] }).supportedValuesOf?.(
        'timeZone',
      ) ?? []
    )
  } catch {
    return []
  }
})()

watch(
  () => [props.open, props.racer] as const,
  ([open, racer]) => {
    if (!open) return
    const handle = (p: string) => racer?.channels.find((c) => c.platform === p)?.handle ?? ''
    form.value = {
      id: racer?.id ?? '',
      displayName: racer?.displayName ?? '',
      timezone: racer?.timezone ?? 'America/Mexico_City',
      country: racer?.country ?? '',
      avatarUrl: racer?.avatarUrl ?? '',
      twitch: handle('twitch'),
      tiktok: handle('tiktok'),
      youtube: handle('youtube'),
    }
  },
  { immediate: true },
)

const idError = computed(() =>
  editing.value || /^[a-z0-9-]{1,40}$/.test(form.value.id)
    ? ''
    : 'Solo minúsculas, números y guiones (máx. 40).',
)
const zoneError = computed(() => {
  try {
    new Intl.DateTimeFormat('es', { timeZone: form.value.timezone })
    return ''
  } catch {
    return 'Zona horaria no válida (ejemplo: America/Mexico_City).'
  }
})
const valid = computed(
  () =>
    !idError.value &&
    !zoneError.value &&
    form.value.displayName.trim().length > 0 &&
    (editing.value || form.value.id.length > 0),
)

function channels(): ChannelInput[] {
  return (['twitch', 'tiktok', 'youtube'] as const)
    .map((platform) => ({ platform, handle: form.value[platform].trim().replace(/^@/, '') }))
    .filter((c) => c.handle)
}

async function save() {
  busy.value = true
  try {
    const base = {
      displayName: form.value.displayName.trim(),
      timezone: form.value.timezone.trim(),
      country: form.value.country.trim() || undefined,
      avatarUrl: form.value.avatarUrl.trim() || undefined,
      channels: channels(),
    }
    if (editing.value && props.racer) {
      await adminApi.updateRacer(props.racer.id, base)
      toasts.success('Corredor guardado.')
      emit('saved')
    } else {
      const input: NewRacerInput = { id: form.value.id.trim(), ...base }
      const out = await adminApi.createRacer(input)
      toasts.success('Corredor creado.')
      emit('saved', out.token)
    }
    emit('close')
  } catch (e) {
    toasts.error(messageOf(e))
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <Modal
    :open="open"
    :title="editing ? `Editar a ${racer?.displayName}` : 'Nuevo corredor'"
    @close="emit('close')"
  >
    <form class="space-y-4" @submit.prevent="save">
      <div v-if="!editing">
        <label class="a-label" for="rf-id">Identificador</label>
        <input
          id="rf-id"
          v-model="form.id"
          class="a-input"
          placeholder="ralbat"
          autocomplete="off"
        />
        <p v-if="idError" class="a-error">{{ idError }}</p>
        <p v-else class="a-hint">
          No se puede cambiar después. Aparece en la dirección del corredor.
        </p>
      </div>
      <div>
        <label class="a-label" for="rf-name">Nombre</label>
        <input id="rf-name" v-model="form.displayName" class="a-input" required />
      </div>
      <div class="grid gap-4 sm:grid-cols-2">
        <div>
          <label class="a-label" for="rf-tz">Zona horaria</label>
          <input id="rf-tz" v-model="form.timezone" class="a-input" list="rf-zones" />
          <datalist id="rf-zones"><option v-for="z in zones" :key="z" :value="z" /></datalist>
          <p v-if="zoneError" class="a-error">{{ zoneError }}</p>
          <p v-else class="a-hint">Su día de 4 h se reinicia a las 06:00 de esta zona.</p>
        </div>
        <div>
          <label class="a-label" for="rf-country">País (2 letras)</label>
          <input
            id="rf-country"
            v-model="form.country"
            class="a-input"
            maxlength="2"
            placeholder="MX"
          />
        </div>
      </div>
      <div>
        <label class="a-label" for="rf-avatar">Foto (URL)</label>
        <input id="rf-avatar" v-model="form.avatarUrl" class="a-input" placeholder="https://…" />
      </div>
      <fieldset class="grid gap-4 sm:grid-cols-3">
        <legend class="a-label">Canales (usuario, sin @)</legend>
        <input v-model="form.twitch" class="a-input" placeholder="Twitch" aria-label="Twitch" />
        <input v-model="form.tiktok" class="a-input" placeholder="TikTok" aria-label="TikTok" />
        <input v-model="form.youtube" class="a-input" placeholder="YouTube" aria-label="YouTube" />
      </fieldset>
      <div class="flex justify-end gap-3">
        <button type="button" class="a-btn" @click="emit('close')">Cancelar</button>
        <button type="submit" class="a-btn a-btn-primary" :disabled="busy || !valid">
          {{ editing ? 'Guardar' : 'Crear corredor' }}
        </button>
      </div>
    </form>
  </Modal>
</template>
