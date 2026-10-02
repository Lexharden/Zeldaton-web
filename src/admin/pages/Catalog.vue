<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { ArrowDown, ArrowUp, Image as ImageIcon, Pencil, Plus, Trash2 } from 'lucide-vue-next'
import { t } from '@/i18n'
import AgeBadge from '@/components/common/AgeBadge.vue'
import { AGE_ORDER, GROUP_ORDER } from '@/utils/catalog'
import { itemIconUrl, validIconName } from '@/utils/media'
import type { CatalogAge, CatalogItem, CatalogObjective } from '@/types/catalog'
import { adminApi } from '../api/AdminApi'
import { confirm } from '../composables/useConfirm'
import { messageOf, useToasts } from '../composables/useToasts'
import MediaPicker from '../components/MediaPicker.vue'
import Modal from '../components/Modal.vue'
import PageHeader from '../components/PageHeader.vue'

const toasts = useToasts()
const tab = ref<'items' | 'objectives'>('items')
const items = ref<CatalogItem[]>([])
const objectives = ref<CatalogObjective[]>([])
const version = ref('')
const pickerOpen = ref(false)

async function load() {
  try {
    const c = await adminApi.catalog()
    items.value = c.items
    objectives.value = c.objectives
    version.value = c.version
  } catch (e) {
    toasts.error(messageOf(e))
  }
}
onMounted(load)

// ---- filters ----------------------------------------------------------------------------------
const query = ref('')
const ageFilter = ref<CatalogAge | 'all'>('all')
const groupFilter = ref('all')
const showHidden = ref(true)

const bySort = <T extends { sortOrder: number; id: string }>(a: T, b: T) =>
  a.sortOrder - b.sortOrder || a.id.localeCompare(b.id)
const matches = (q: string, ...texts: string[]) => !q || texts.join(' ').toLowerCase().includes(q)

const shownItems = computed(() => {
  const q = query.value.trim().toLowerCase()
  return items.value
    .filter(
      (i) =>
        (showHidden.value || i.enabled) &&
        (ageFilter.value === 'all' || i.age === ageFilter.value) &&
        (groupFilter.value === 'all' || i.group === groupFilter.value) &&
        matches(q, i.id, i.nameEs, i.nameEn),
    )
    .sort(bySort)
})
const shownObjectives = computed(() => {
  const q = query.value.trim().toLowerCase()
  return objectives.value
    .filter(
      (o) =>
        (showHidden.value || o.enabled) &&
        (ageFilter.value === 'all' || o.age === ageFilter.value) &&
        matches(q, o.id, o.nameEs, o.nameEn),
    )
    .sort(bySort)
})
const counts = computed(() =>
  AGE_ORDER.map((age) => ({
    age,
    items: items.value.filter((i) => i.enabled && i.age === age).length,
    objectives: objectives.value.filter((o) => o.enabled && o.age === age).length,
  })),
)
const groups = computed(() => {
  const known = new Set([...GROUP_ORDER, ...items.value.map((i) => i.group)])
  return [...known].sort(
    (a, b) => (GROUP_ORDER.indexOf(a) + 1 || 99) - (GROUP_ORDER.indexOf(b) + 1 || 99),
  )
})
const groupLabel = (g: string) => {
  const key = `itemGroups.${g}`
  const out = t(key)
  return out === key ? g : out
}

// ---- edit -------------------------------------------------------------------------------------
type ItemForm = CatalogItem & { isNew: boolean }
type ObjectiveForm = CatalogObjective & { isNew: boolean }
const itemForm = ref<ItemForm | null>(null)
const objectiveForm = ref<ObjectiveForm | null>(null)
const busy = ref(false)

const nextOrder = (list: { sortOrder: number }[]) =>
  Math.max(0, ...list.map((x) => x.sortOrder)) + 10

function newItem() {
  itemForm.value = {
    isNew: true,
    id: '',
    group: 'tool',
    age: 'both',
    nameEs: '',
    nameEn: '',
    short: '',
    icon: '',
    sortOrder: nextOrder(items.value),
    enabled: true,
  }
}
function newObjective() {
  objectiveForm.value = {
    isNew: true,
    id: '',
    age: 'adult',
    nameEs: '',
    nameEn: '',
    sortOrder: nextOrder(objectives.value),
    required: false,
    enabled: true,
  }
}

/** The form carries an `isNew` flag the API must not receive. */
function withoutNew<T extends { isNew: boolean }>(form: T): Omit<T, 'isNew'> {
  const { isNew, ...rest } = form
  void isNew
  return rest
}

const idValid = (id: string) => /^[a-z0-9-]{1,40}$/.test(id)
const itemFormError = computed(() => {
  const f = itemForm.value
  if (!f) return ''
  if (f.isNew && !idValid(f.id)) return 'El identificador: minúsculas, números y guiones (máx. 40).'
  if (f.isNew && items.value.some((i) => i.id === f.id))
    return 'Ya existe un ítem con ese identificador.'
  if (!f.nameEs.trim() || !f.nameEn.trim()) return 'Falta el nombre en español o inglés.'
  if (!f.short.trim() || f.short.trim().length > 4) return 'La abreviatura es de 1 a 4 caracteres.'
  // Older items may still hold a site path or an https URL; new ones are just a file name.
  if (f.icon && !/^(\/|https:\/\/)/.test(f.icon) && !validIconName(f.icon.trim()))
    return "El icono es el nombre de un archivo (Hookshot-Art.png): letras, números, espacios y _ - . ' ( ), terminado en .png, .jpg o .webp."
  return ''
})
const objectiveFormError = computed(() => {
  const f = objectiveForm.value
  if (!f) return ''
  if (f.isNew && !idValid(f.id)) return 'El identificador: minúsculas, números y guiones (máx. 40).'
  if (f.isNew && objectives.value.some((o) => o.id === f.id))
    return 'Ya existe un objetivo con ese identificador.'
  if (!f.nameEs.trim() || !f.nameEn.trim()) return 'Falta el nombre en español o inglés.'
  return ''
})

async function saveItem() {
  const f = itemForm.value
  if (!f) return
  busy.value = true
  try {
    const item = withoutNew(f)
    await adminApi.saveItem({ ...item, icon: item.icon?.trim() || undefined })
    toasts.success('Ítem guardado. La web se actualiza sola.')
    itemForm.value = null
    await load()
  } catch (e) {
    toasts.error(messageOf(e))
  } finally {
    busy.value = false
  }
}
async function saveObjective() {
  const f = objectiveForm.value
  if (!f) return
  busy.value = true
  try {
    await adminApi.saveObjective(withoutNew(f))
    toasts.success('Objetivo guardado.')
    objectiveForm.value = null
    await load()
  } catch (e) {
    toasts.error(messageOf(e))
  } finally {
    busy.value = false
  }
}

async function toggleItem(i: CatalogItem) {
  try {
    await adminApi.saveItem({ ...i, enabled: !i.enabled })
    await load()
  } catch (e) {
    toasts.error(messageOf(e))
  }
}
async function toggleObjective(o: CatalogObjective) {
  try {
    await adminApi.saveObjective({ ...o, enabled: !o.enabled })
    await load()
  } catch (e) {
    toasts.error(messageOf(e))
  }
}

async function removeItem(i: CatalogItem) {
  const ok = await confirm({
    title: 'Eliminar ítem',
    message: `«${i.nameEs}» desaparece del catálogo. Si solo quieres ocultarlo, desactívalo: los corredores conservan lo que ya tienen.`,
    confirmLabel: 'Eliminar',
    danger: true,
    requireText: i.id,
  })
  if (!ok) return
  try {
    await adminApi.deleteItem(i.id)
    toasts.success('Ítem eliminado.')
    await load()
  } catch (e) {
    toasts.error(messageOf(e))
  }
}
async function removeObjective(o: CatalogObjective) {
  const ok = await confirm({
    title: 'Eliminar objetivo',
    message: `«${o.nameEs}» desaparece del catálogo.`,
    confirmLabel: 'Eliminar',
    danger: true,
    requireText: o.id,
  })
  if (!ok) return
  try {
    await adminApi.deleteObjective(o.id)
    toasts.success('Objetivo eliminado.')
    await load()
  } catch (e) {
    toasts.error(messageOf(e))
  }
}

/** Swap the display order with the neighbour (in the same Link) above or below. */
async function move<T extends { id: string; age: CatalogAge; sortOrder: number }>(
  list: T[],
  entry: T,
  dir: -1 | 1,
  save: (x: T) => Promise<unknown>,
) {
  const peers = list.filter((x) => x.age === entry.age).sort(bySort)
  const at = peers.findIndex((x) => x.id === entry.id)
  const other = peers[at + dir]
  if (!other) return
  try {
    await Promise.all([
      save({ ...entry, sortOrder: other.sortOrder }),
      save({ ...other, sortOrder: entry.sortOrder }),
    ])
    await load()
  } catch (e) {
    toasts.error(messageOf(e))
  }
}
</script>

<template>
  <div>
    <PageHeader
      title="Catálogo"
      subtitle="Ítems y objetivos de la carrera, organizados por Link. Los cambios llegan a la web y a HiveShock al momento."
    >
      <button v-if="tab === 'items'" type="button" class="a-btn a-btn-primary" @click="newItem">
        <Plus class="size-4" />Nuevo ítem
      </button>
      <button v-else type="button" class="a-btn a-btn-primary" @click="newObjective">
        <Plus class="size-4" />Nuevo objetivo
      </button>
    </PageHeader>

    <section class="mb-5 grid gap-3 sm:grid-cols-3" aria-label="Resumen">
      <div v-for="c in counts" :key="c.age" class="panel panel-sm">
        <div class="flex items-center justify-between p-4">
          <AgeBadge :age="c.age" />
          <span class="num text-sm text-white"
            >{{ c.items }} <span class="text-muted">ítems</span> · {{ c.objectives }}
            <span class="text-muted">objetivos</span></span
          >
        </div>
      </div>
    </section>

    <div class="mb-4 flex flex-wrap items-center gap-x-6 gap-y-3 border-b border-line">
      <div role="tablist" class="flex">
        <button
          role="tab"
          type="button"
          class="a-tab"
          :class="tab === 'items' && 'a-tab-active'"
          :aria-selected="tab === 'items'"
          @click="tab = 'items'"
        >
          Ítems ({{ items.length }})
        </button>
        <button
          role="tab"
          type="button"
          class="a-tab"
          :class="tab === 'objectives' && 'a-tab-active'"
          :aria-selected="tab === 'objectives'"
          @click="tab = 'objectives'"
        >
          Objetivos ({{ objectives.length }})
        </button>
      </div>
      <div class="flex flex-1 flex-wrap items-center gap-3 pb-2">
        <input v-model="query" class="a-input max-w-56" placeholder="Buscar…" aria-label="Buscar" />
        <select v-model="ageFilter" class="a-select w-36" aria-label="Link">
          <option value="all">Todos los Link</option>
          <option v-for="a in AGE_ORDER" :key="a" :value="a">{{ t(`age.${a}`) }}</option>
        </select>
        <select
          v-if="tab === 'items'"
          v-model="groupFilter"
          class="a-select w-44"
          aria-label="Categoría"
        >
          <option value="all">Todas las categorías</option>
          <option v-for="g in groups" :key="g" :value="g">{{ groupLabel(g) }}</option>
        </select>
        <label class="flex items-center gap-2 text-sm text-muted"
          ><input v-model="showHidden" type="checkbox" />Mostrar ocultos</label
        >
        <button v-if="tab === 'items'" type="button" class="a-btn" @click="pickerOpen = true">
          <ImageIcon class="size-4" />Imágenes
        </button>
      </div>
    </div>

    <section v-if="tab === 'items'" class="panel">
      <div class="overflow-x-auto p-2">
        <table class="a-table">
          <thead>
            <tr>
              <th>Orden</th>
              <th>Ítem</th>
              <th>Link</th>
              <th>Categoría</th>
              <th>Visible</th>
              <th />
            </tr>
          </thead>
          <tbody>
            <tr v-for="i in shownItems" :key="i.id" :class="!i.enabled && 'opacity-50'">
              <td>
                <div class="flex items-center gap-1">
                  <button
                    type="button"
                    class="a-btn a-btn-sm"
                    aria-label="Subir"
                    @click="move(items, i, -1, adminApi.saveItem)"
                  >
                    <ArrowUp class="size-3.5" />
                  </button>
                  <button
                    type="button"
                    class="a-btn a-btn-sm"
                    aria-label="Bajar"
                    @click="move(items, i, 1, adminApi.saveItem)"
                  >
                    <ArrowDown class="size-3.5" />
                  </button>
                </div>
              </td>
              <td>
                <div class="flex items-center gap-3">
                  <img
                    v-if="i.icon"
                    :src="itemIconUrl(i.icon)"
                    alt=""
                    class="size-8 object-contain"
                  />
                  <span
                    v-else
                    class="num grid size-8 place-items-center rounded bg-white/5 text-sm font-bold text-white"
                    >{{ i.short }}</span
                  >
                  <div>
                    <div class="font-semibold text-white">{{ i.nameEs }}</div>
                    <div class="text-xs text-muted">{{ i.nameEn }} · {{ i.id }}</div>
                  </div>
                </div>
              </td>
              <td><AgeBadge :age="i.age" short /></td>
              <td class="text-sm">{{ groupLabel(i.group) }}</td>
              <td>
                <button
                  type="button"
                  class="a-switch"
                  :class="i.enabled && 'a-switch-on'"
                  role="switch"
                  :aria-checked="i.enabled"
                  :aria-label="`Visible: ${i.nameEs}`"
                  @click="toggleItem(i)"
                />
              </td>
              <td>
                <div class="flex justify-end gap-2">
                  <button
                    type="button"
                    class="a-btn a-btn-sm"
                    @click="itemForm = { ...i, icon: i.icon ?? '', isNew: false }"
                  >
                    <Pencil class="size-3.5" />Editar
                  </button>
                  <button
                    type="button"
                    class="a-btn a-btn-sm a-btn-danger"
                    aria-label="Eliminar"
                    @click="removeItem(i)"
                  >
                    <Trash2 class="size-3.5" />
                  </button>
                </div>
              </td>
            </tr>
            <tr v-if="!shownItems.length">
              <td colspan="6" class="py-6 text-center text-muted">
                Nada coincide con los filtros.
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>

    <section v-else class="panel">
      <div class="overflow-x-auto p-2">
        <table class="a-table">
          <thead>
            <tr>
              <th>Orden</th>
              <th>Objetivo</th>
              <th>Link</th>
              <th>Requerido</th>
              <th>Visible</th>
              <th />
            </tr>
          </thead>
          <tbody>
            <tr v-for="o in shownObjectives" :key="o.id" :class="!o.enabled && 'opacity-50'">
              <td>
                <div class="flex items-center gap-1">
                  <button
                    type="button"
                    class="a-btn a-btn-sm"
                    aria-label="Subir"
                    @click="move(objectives, o, -1, adminApi.saveObjective)"
                  >
                    <ArrowUp class="size-3.5" />
                  </button>
                  <button
                    type="button"
                    class="a-btn a-btn-sm"
                    aria-label="Bajar"
                    @click="move(objectives, o, 1, adminApi.saveObjective)"
                  >
                    <ArrowDown class="size-3.5" />
                  </button>
                </div>
              </td>
              <td>
                <div class="font-semibold text-white">{{ o.nameEs }}</div>
                <div class="text-xs text-muted">{{ o.nameEn }} · {{ o.id }}</div>
              </td>
              <td><AgeBadge :age="o.age" short /></td>
              <td class="text-sm">{{ o.required ? 'Sí' : 'No' }}</td>
              <td>
                <button
                  type="button"
                  class="a-switch"
                  :class="o.enabled && 'a-switch-on'"
                  role="switch"
                  :aria-checked="o.enabled"
                  :aria-label="`Visible: ${o.nameEs}`"
                  @click="toggleObjective(o)"
                />
              </td>
              <td>
                <div class="flex justify-end gap-2">
                  <button
                    type="button"
                    class="a-btn a-btn-sm"
                    @click="objectiveForm = { ...o, isNew: false }"
                  >
                    <Pencil class="size-3.5" />Editar
                  </button>
                  <button
                    type="button"
                    class="a-btn a-btn-sm a-btn-danger"
                    aria-label="Eliminar"
                    @click="removeObjective(o)"
                  >
                    <Trash2 class="size-3.5" />
                  </button>
                </div>
              </td>
            </tr>
            <tr v-if="!shownObjectives.length">
              <td colspan="6" class="py-6 text-center text-muted">
                Nada coincide con los filtros.
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>
    <p class="mt-3 text-xs text-muted">
      Versión del catálogo: <span class="font-mono">{{ version }}</span
      >. Un objetivo que exige el evento no se puede ocultar ni borrar: quítalo primero de las
      reglas del evento.
    </p>

    <Modal
      :open="!!itemForm"
      :title="itemForm?.isNew ? 'Nuevo ítem' : `Editar «${itemForm?.nameEs}»`"
      @close="itemForm = null"
    >
      <form v-if="itemForm" class="space-y-4" @submit.prevent="saveItem">
        <div v-if="itemForm.isNew">
          <label class="a-label" for="it-id">Identificador</label>
          <input
            id="it-id"
            v-model="itemForm.id"
            class="a-input"
            placeholder="magic-beans"
            autocomplete="off"
          />
          <p class="a-hint">
            Es el id que reporta HiveShock (zeldathon.json). No se cambia después.
          </p>
        </div>
        <div class="grid gap-4 sm:grid-cols-2">
          <div>
            <label class="a-label" for="it-es">Nombre (español)</label
            ><input id="it-es" v-model="itemForm.nameEs" class="a-input" />
          </div>
          <div>
            <label class="a-label" for="it-en">Nombre (inglés)</label
            ><input id="it-en" v-model="itemForm.nameEn" class="a-input" />
          </div>
        </div>
        <div class="grid gap-4 sm:grid-cols-3">
          <div>
            <label class="a-label" for="it-age">Link</label>
            <select id="it-age" v-model="itemForm.age" class="a-select">
              <option v-for="a in AGE_ORDER" :key="a" :value="a">{{ t(`age.${a}`) }}</option>
            </select>
          </div>
          <div>
            <label class="a-label" for="it-group">Categoría</label>
            <select id="it-group" v-model="itemForm.group" class="a-select">
              <option v-for="g in groups" :key="g" :value="g">{{ groupLabel(g) }}</option>
            </select>
          </div>
          <div>
            <label class="a-label" for="it-short">Abreviatura</label
            ><input id="it-short" v-model="itemForm.short" class="a-input" maxlength="4" />
          </div>
        </div>
        <div class="grid gap-4 sm:grid-cols-2">
          <div>
            <label class="a-label" for="it-icon">Icono (opcional)</label>
            <div class="flex items-center gap-2">
              <img
                v-if="itemForm.icon && !itemFormError.startsWith('El icono')"
                :src="itemIconUrl(itemForm.icon.trim())"
                alt=""
                class="size-10 shrink-0 rounded bg-white/5 object-contain"
              />
              <input
                id="it-icon"
                v-model="itemForm.icon"
                class="a-input"
                placeholder="Hookshot-Art.png"
                autocomplete="off"
              />
              <button type="button" class="a-btn shrink-0" @click="pickerOpen = true">
                <ImageIcon class="size-4" />Elegir
              </button>
            </div>
            <p class="a-hint">Solo el nombre del archivo. Elige una imagen o sube una nueva.</p>
          </div>
          <div>
            <label class="a-label" for="it-order">Orden</label
            ><input
              id="it-order"
              v-model.number="itemForm.sortOrder"
              type="number"
              class="a-input"
            />
          </div>
        </div>
        <label class="flex items-center gap-2 text-sm"
          ><input v-model="itemForm.enabled" type="checkbox" />Visible y aceptado por el
          servidor</label
        >
        <p v-if="itemFormError" class="a-error">{{ itemFormError }}</p>
        <div class="flex justify-end gap-3">
          <button type="button" class="a-btn" @click="itemForm = null">Cancelar</button>
          <button type="submit" class="a-btn a-btn-primary" :disabled="busy || !!itemFormError">
            Guardar
          </button>
        </div>
      </form>
    </Modal>

    <MediaPicker
      :open="pickerOpen"
      :selected="itemForm?.icon"
      @close="pickerOpen = false"
      @pick="
        (name) => {
          if (itemForm) {
            itemForm.icon = name
            pickerOpen = false
          }
        }
      "
    />

    <Modal
      :open="!!objectiveForm"
      :title="objectiveForm?.isNew ? 'Nuevo objetivo' : `Editar «${objectiveForm?.nameEs}»`"
      @close="objectiveForm = null"
    >
      <form v-if="objectiveForm" class="space-y-4" @submit.prevent="saveObjective">
        <div v-if="objectiveForm.isNew">
          <label class="a-label" for="ob-id">Identificador</label>
          <input id="ob-id" v-model="objectiveForm.id" class="a-input" autocomplete="off" />
        </div>
        <div class="grid gap-4 sm:grid-cols-2">
          <div>
            <label class="a-label" for="ob-es">Nombre (español)</label
            ><input id="ob-es" v-model="objectiveForm.nameEs" class="a-input" />
          </div>
          <div>
            <label class="a-label" for="ob-en">Nombre (inglés)</label
            ><input id="ob-en" v-model="objectiveForm.nameEn" class="a-input" />
          </div>
        </div>
        <div class="grid gap-4 sm:grid-cols-2">
          <div>
            <label class="a-label" for="ob-age">Link</label>
            <select id="ob-age" v-model="objectiveForm.age" class="a-select">
              <option v-for="a in AGE_ORDER" :key="a" :value="a">{{ t(`age.${a}`) }}</option>
            </select>
          </div>
          <div>
            <label class="a-label" for="ob-order">Orden</label
            ><input
              id="ob-order"
              v-model.number="objectiveForm.sortOrder"
              type="number"
              class="a-input"
            />
          </div>
        </div>
        <label class="flex items-center gap-2 text-sm"
          ><input v-model="objectiveForm.required" type="checkbox" />Cuenta para terminar por
          defecto</label
        >
        <label class="flex items-center gap-2 text-sm"
          ><input v-model="objectiveForm.enabled" type="checkbox" />Visible y aceptado por el
          servidor</label
        >
        <p v-if="objectiveFormError" class="a-error">{{ objectiveFormError }}</p>
        <div class="flex justify-end gap-3">
          <button type="button" class="a-btn" @click="objectiveForm = null">Cancelar</button>
          <button
            type="submit"
            class="a-btn a-btn-primary"
            :disabled="busy || !!objectiveFormError"
          >
            Guardar
          </button>
        </div>
      </form>
    </Modal>
  </div>
</template>
