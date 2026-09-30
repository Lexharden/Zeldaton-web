<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { RefreshCw } from 'lucide-vue-next'
import { adminApi } from '../api/AdminApi'
import { actionLabel, localDateTime } from '../format'
import { messageOf, useToasts } from '../composables/useToasts'
import PageHeader from '../components/PageHeader.vue'
import type { AuditRow } from '../types'

const toasts = useToasts()
const rows = ref<AuditRow[]>([])
const limit = ref(200)
const query = ref('')
const loading = ref(false)

async function load() {
  loading.value = true
  try {
    rows.value = await adminApi.audit(limit.value)
  } catch (e) {
    toasts.error(messageOf(e))
  } finally {
    loading.value = false
  }
}
onMounted(load)

const filtered = computed(() => {
  const q = query.value.trim().toLowerCase()
  if (!q) return rows.value
  return rows.value.filter((r) =>
    [r.actor, r.action, actionLabel(r.action), r.racerId ?? '', JSON.stringify(r.payload)]
      .join(' ')
      .toLowerCase()
      .includes(q),
  )
})

/** The interesting bits of the payload, e.g. the reason for a time adjustment. */
function detail(r: AuditRow): string {
  const p = r.payload
  const bits: string[] = []
  if (typeof p.reason === 'string' && p.reason) bits.push(`Motivo: ${p.reason}`)
  if (typeof p.deltaSeconds === 'number')
    bits.push(`${p.deltaSeconds > 0 ? '+' : ''}${Math.round(p.deltaSeconds / 60)} min`)
  if (typeof p.username === 'string') bits.push(String(p.username))
  if (typeof p.id === 'string') bits.push(p.id)
  if (typeof p.ip === 'string') bits.push(`IP ${p.ip}`)
  return bits.join(' · ')
}
</script>

<template>
  <div>
    <PageHeader title="Auditoría" subtitle="El registro oficial: quién hizo qué y cuándo.">
      <input
        v-model="query"
        class="a-input w-64"
        placeholder="Buscar (persona, acción, corredor…)"
        aria-label="Buscar"
      />
      <select v-model.number="limit" class="a-select w-32" aria-label="Cuántos" @change="load">
        <option :value="100">Últimos 100</option>
        <option :value="200">Últimos 200</option>
        <option :value="500">Últimos 500</option>
      </select>
      <button type="button" class="a-btn" :disabled="loading" @click="load">
        <RefreshCw class="size-4" />Actualizar
      </button>
    </PageHeader>

    <section class="panel">
      <div class="overflow-x-auto p-2">
        <table class="a-table">
          <thead>
            <tr>
              <th>Cuándo</th>
              <th>Quién</th>
              <th>Qué</th>
              <th>Corredor</th>
              <th>Detalle</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="r in filtered" :key="r.id">
              <td class="num whitespace-nowrap text-xs text-muted">{{ localDateTime(r.ts) }}</td>
              <td
                class="font-semibold"
                :class="r.actor === 'anonymous' ? 'text-warning' : 'text-white'"
              >
                {{ r.actor }}
              </td>
              <td :title="r.action">{{ actionLabel(r.action) }}</td>
              <td class="text-xs text-secondary">{{ r.racerId ?? '' }}</td>
              <td class="text-xs text-muted">{{ detail(r) }}</td>
            </tr>
            <tr v-if="!filtered.length">
              <td colspan="5" class="py-6 text-center text-muted">Sin resultados.</td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>
  </div>
</template>
