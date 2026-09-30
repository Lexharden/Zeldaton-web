<script setup lang="ts">
import { ref } from 'vue'
import { Check, Copy } from 'lucide-vue-next'
import Modal from './Modal.vue'

/** Shows a secret exactly once (a racer's ingest token, a generated password) with a copy button. */
defineProps<{ open: boolean; title: string; secret: string; note?: string }>()
const emit = defineEmits<{ close: [] }>()
const copied = ref(false)

async function copy(text: string) {
  try {
    await navigator.clipboard.writeText(text)
    copied.value = true
    setTimeout(() => (copied.value = false), 2000)
  } catch {
    /* clipboard blocked: the text is selectable */
  }
}
</script>

<template>
  <Modal :open="open" :title="title" @close="emit('close')">
    <p class="mb-3 rounded border border-warning/40 bg-warning/10 p-3 text-sm text-warning">
      Cópialo ahora: <strong>no se vuelve a mostrar</strong>. Si se pierde hay que generar otro.
    </p>
    <code class="block break-all rounded bg-black/40 p-3 font-mono text-sm text-white select-all">{{
      secret
    }}</code>
    <p v-if="note" class="a-hint">{{ note }}</p>
    <div class="mt-5 flex justify-end gap-3">
      <button type="button" class="a-btn" @click="copy(secret)">
        <Check v-if="copied" class="size-4 text-success" /><Copy v-else class="size-4" />{{
          copied ? 'Copiado' : 'Copiar'
        }}
      </button>
      <button type="button" class="a-btn a-btn-primary" @click="emit('close')">Listo</button>
    </div>
  </Modal>
</template>
