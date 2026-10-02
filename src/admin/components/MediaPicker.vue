<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { Trash2, Upload } from 'lucide-vue-next'
import ArtImage from '@/components/common/ArtImage.vue'
import { itemIconUrl, shrinkPicture, validIconName } from '@/utils/media'
import { adminApi } from '../api/AdminApi'
import { confirm } from '../composables/useConfirm'
import { messageOf, useToasts } from '../composables/useToasts'
import type { MediaFile } from '../types'
import Modal from './Modal.vue'

/**
 * The picture library for catalog items: pick one that exists or upload a new one. Pictures keep
 * the name they have on the organizer's computer and are shrunk in the browser first.
 */
const props = defineProps<{ open: boolean; selected?: string }>()
const emit = defineEmits<{ close: []; pick: [name: string] }>()

const toasts = useToasts()
const files = ref<MediaFile[]>([])
const query = ref('')
const loading = ref(false)
const uploading = ref(false)
const input = ref<HTMLInputElement | null>(null)
// Bumped after an upload so a replaced picture is fetched again instead of read from the cache.
const stamp = ref(0)

async function load() {
  loading.value = true
  try {
    files.value = await adminApi.mediaItems()
  } catch (e) {
    toasts.error(messageOf(e))
  } finally {
    loading.value = false
  }
}
watch(
  () => props.open,
  (open) => open && void load(),
  { immediate: true },
)

const shown = computed(() => {
  const q = query.value.trim().toLowerCase()
  return q ? files.value.filter((f) => f.name.toLowerCase().includes(q)) : files.value
})

const url = (name: string) => `${itemIconUrl(name)}?v=${stamp.value}`
const kb = (bytes: number) => `${Math.max(1, Math.round(bytes / 1024))} KB`

async function onFiles(e: Event) {
  const target = e.target as HTMLInputElement
  const chosen = [...(target.files ?? [])]
  target.value = ''
  if (!chosen.length) return
  uploading.value = true
  let last = ''
  try {
    for (const file of chosen) {
      if (!validIconName(file.name)) {
        toasts.error(
          `«${file.name}»: el nombre solo puede llevar letras, números, espacios y _ - . ' ( ), y terminar en .png, .jpg o .webp.`,
        )
        continue
      }
      try {
        const picture = await shrinkPicture(file)
        const out = await adminApi.uploadMedia(file.name, picture)
        last = out.name
      } catch (err) {
        toasts.error(`${file.name}: ${messageOf(err)}`)
      }
    }
    if (last) {
      stamp.value++
      toasts.success(chosen.length > 1 ? 'Imágenes subidas.' : `«${last}» subida.`)
      await load()
      // A single upload goes straight to the item that asked for it.
      if (chosen.length === 1) emit('pick', last)
    }
  } finally {
    uploading.value = false
  }
}

async function remove(f: MediaFile) {
  const ok = await confirm({
    title: 'Borrar imagen',
    message: `¿Borrar «${f.name}»? Los ítems que la usan mostrarán su abreviatura.`,
    confirmLabel: 'Borrar',
    danger: true,
  })
  if (!ok) return
  try {
    await adminApi.deleteMedia(f.name)
    toasts.success('Imagen borrada.')
    await load()
  } catch (e) {
    toasts.error(messageOf(e))
  }
}
</script>

<template>
  <Modal :open="open" title="Imágenes de ítems" wide @close="emit('close')">
    <div class="mb-4 flex flex-wrap items-center gap-3">
      <input
        v-model="query"
        class="a-input max-w-56"
        placeholder="Buscar…"
        aria-label="Buscar imagen"
      />
      <input
        ref="input"
        type="file"
        accept="image/png,image/jpeg,image/webp"
        multiple
        class="hidden"
        @change="onFiles"
      />
      <button
        type="button"
        class="a-btn a-btn-primary"
        :disabled="uploading"
        @click="input?.click()"
      >
        <Upload class="size-4" />{{ uploading ? 'Subiendo…' : 'Subir imagen' }}
      </button>
      <span class="text-xs text-muted"
        >PNG, JPG o WebP. Se conserva el nombre del archivo y se reduce a 256 px.</span
      >
    </div>

    <p v-if="loading && !files.length" class="text-sm text-muted">Cargando…</p>
    <ul
      v-else
      class="grid max-h-[55vh] grid-cols-3 gap-2 overflow-y-auto pr-1 sm:grid-cols-4 md:grid-cols-5"
    >
      <li
        v-for="f in shown"
        :key="f.name"
        class="group relative rounded border border-white/10 bg-white/[0.03] transition-colors hover:border-primary/60"
        :class="f.name === selected && 'border-primary bg-primary/10'"
      >
        <button
          type="button"
          class="flex w-full flex-col items-center gap-1 p-2"
          :title="f.name"
          @click="emit('pick', f.name)"
        >
          <ArtImage :src="url(f.name)" alt="" class="size-16 object-contain" />
          <span class="w-full truncate text-center text-[10px] text-secondary">{{ f.name }}</span>
          <span class="text-[10px] text-muted"
            >{{ kb(f.bytes) }}<template v-if="f.uploaded"> · subida</template></span
          >
        </button>
        <button
          v-if="f.uploaded"
          type="button"
          class="absolute right-1 top-1 rounded bg-black/60 p-1 text-muted opacity-0 transition-opacity hover:text-[#ff8aa0] focus:opacity-100 group-hover:opacity-100"
          :aria-label="`Borrar ${f.name}`"
          @click="remove(f)"
        >
          <Trash2 class="size-3.5" />
        </button>
      </li>
      <li v-if="!shown.length" class="col-span-full py-6 text-center text-sm text-muted">
        No hay imágenes{{ query ? ' con ese nombre' : '' }}.
      </li>
    </ul>
  </Modal>
</template>
