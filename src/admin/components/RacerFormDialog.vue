<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { Trash2, Upload } from 'lucide-vue-next'
import { adminApi } from '../api/AdminApi'
import { messageOf, useToasts } from '../composables/useToasts'
import type { ChannelInput, NewRacerInput } from '../types'
import type { Racer } from '@/types/racer'
import { AMERICAS_TIMEZONES, currentOffset, findZone } from '@/config/timezones'
import Avatar from '@/components/common/Avatar.vue'
import { shrinkPicture } from '@/utils/media'
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
  twitch: '',
  tiktok: '',
  youtube: '',
})
const busy = ref(false)

// The photo is uploaded, not typed: the chosen file waits here until the racer is saved.
const PHOTO_TYPES = ['image/png', 'image/jpeg', 'image/webp']
const fileInput = ref<HTMLInputElement | null>(null)
const pendingFile = ref<File | null>(null)
const previewUrl = ref('')
const removePhoto = ref(false)
const currentPhoto = computed(() => (removePhoto.value ? '' : (props.racer?.avatarUrl ?? '')))
const shownPhoto = computed(() => previewUrl.value || currentPhoto.value || undefined)

function clearPending() {
  if (previewUrl.value) URL.revokeObjectURL(previewUrl.value)
  previewUrl.value = ''
  pendingFile.value = null
}
function onPhoto(e: Event) {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0]
  input.value = ''
  if (!file) return
  if (!PHOTO_TYPES.includes(file.type)) {
    toasts.error('La foto debe ser PNG, JPG o WebP.')
    return
  }
  clearPending()
  pendingFile.value = file
  previewUrl.value = URL.createObjectURL(file)
  removePhoto.value = false
}
function dropPhoto() {
  clearPending()
  removePhoto.value = !!props.racer?.avatarUrl
}
onBeforeUnmount(clearPending)

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

/** Picker value for "not in the list": the organizer types an IANA name instead. */
const OTHER = '__other'
const otherZone = ref(false)
const zoneChoice = computed({
  get: () => (otherZone.value || !findZone(form.value.timezone) ? OTHER : form.value.timezone),
  set: (value: string) => {
    if (value === OTHER) {
      otherZone.value = true
      return
    }
    otherZone.value = false
    form.value.timezone = value
    // Prefill the country from the zone when it is still empty.
    const zone = findZone(value)
    if (zone && !form.value.country.trim()) form.value.country = zone.country
  },
})
/** Current offset of every listed zone (already with daylight saving), computed when the dialog opens. */
const offsets = ref<Record<string, string>>({})
const localNow = computed(() => {
  try {
    return new Intl.DateTimeFormat('es-MX', {
      timeZone: form.value.timezone,
      hour: '2-digit',
      minute: '2-digit',
      hourCycle: 'h23',
    }).format(new Date())
  } catch {
    return ''
  }
})

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
      twitch: handle('twitch'),
      tiktok: handle('tiktok'),
      youtube: handle('youtube'),
    }
    clearPending()
    removePhoto.value = false
    otherZone.value = !findZone(form.value.timezone)
    const now = new Date()
    offsets.value = Object.fromEntries(
      AMERICAS_TIMEZONES.flatMap((g) => g.zones).map((z) => [z.id, currentOffset(z.id, now)]),
    )
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
      channels: channels(),
    }
    let id = props.racer?.id ?? ''
    let token: string | undefined
    if (editing.value && props.racer) {
      await adminApi.updateRacer(props.racer.id, base)
    } else {
      const input: NewRacerInput = { id: form.value.id.trim(), ...base }
      const out = await adminApi.createRacer(input)
      id = input.id
      token = out.token
    }
    // The photo goes last: the racer exists and is saved even if the picture fails.
    let photoError = ''
    try {
      if (pendingFile.value) {
        await adminApi.uploadRacerPhoto(id, await shrinkPicture(pendingFile.value))
      } else if (removePhoto.value) {
        await adminApi.deleteRacerPhoto(id)
      }
    } catch (e) {
      photoError = messageOf(e)
    }
    toasts.success(editing.value ? 'Corredor guardado.' : 'Corredor creado.')
    if (photoError) toasts.error(`La foto no se pudo guardar: ${photoError}`)
    emit('saved', token)
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
          <select id="rf-tz" v-model="zoneChoice" class="a-select">
            <optgroup v-for="g in AMERICAS_TIMEZONES" :key="g.label" :label="g.label">
              <option v-for="z in g.zones" :key="z.id" :value="z.id">
                {{ z.city }} · {{ z.area }}{{ offsets[z.id] ? ` (${offsets[z.id]})` : '' }}
              </option>
            </optgroup>
            <option :value="OTHER">Otra zona (escribir el nombre IANA)…</option>
          </select>
          <template v-if="zoneChoice === OTHER">
            <input
              v-model="form.timezone"
              class="a-input mt-2"
              list="rf-zones"
              placeholder="Europe/Madrid"
              aria-label="Zona horaria (IANA)"
            />
            <datalist id="rf-zones"><option v-for="z in zones" :key="z" :value="z" /></datalist>
          </template>
          <p v-if="zoneError" class="a-error">{{ zoneError }}</p>
          <p v-else class="a-hint">
            Allí son las {{ localNow }}. Su día se reinicia a la hora del evento en esta zona (el
            horario de verano se aplica solo).
          </p>
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
        <span class="a-label">Foto</span>
        <div class="flex items-center gap-4">
          <Avatar
            :name="form.displayName || form.id || '?'"
            :src="shownPhoto"
            :seed="form.id || racer?.id"
            :size="72"
          />
          <input
            ref="fileInput"
            type="file"
            accept="image/png,image/jpeg,image/webp"
            class="hidden"
            @change="onPhoto"
          />
          <div class="flex flex-wrap gap-2">
            <button type="button" class="a-btn" @click="fileInput?.click()">
              <Upload class="size-4" />{{ shownPhoto ? 'Cambiar foto' : 'Subir foto' }}
            </button>
            <button v-if="shownPhoto" type="button" class="a-btn" @click="dropPhoto">
              <Trash2 class="size-4" />Quitar
            </button>
          </div>
        </div>
        <p class="a-hint">
          PNG, JPG o WebP; se ajusta a 512 px. Se guarda al pulsar «{{
            editing ? 'Guardar' : 'Crear corredor'
          }}».
        </p>
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
