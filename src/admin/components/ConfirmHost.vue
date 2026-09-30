<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { answer, pending } from '../composables/useConfirm'
import Modal from './Modal.vue'

/** Renders whatever `confirm()` asked for. Mounted once in the layout. */
const typed = ref('')
const ok = computed(() => !pending.value?.requireText || typed.value === pending.value.requireText)
watch(pending, () => (typed.value = ''))
</script>

<template>
  <Modal :open="!!pending" :title="pending?.title ?? ''" @close="answer(false)">
    <p class="text-sm text-muted">{{ pending?.message }}</p>
    <div v-if="pending?.requireText" class="mt-4">
      <label class="a-label" for="confirm-text"
        >Escribe <strong class="text-white">{{ pending.requireText }}</strong> para continuar</label
      >
      <input id="confirm-text" v-model="typed" class="a-input" autocomplete="off" />
    </div>
    <div class="mt-6 flex justify-end gap-3">
      <button type="button" class="a-btn" @click="answer(false)">Cancelar</button>
      <button
        type="button"
        class="a-btn"
        :class="pending?.danger ? 'a-btn-danger' : 'a-btn-primary'"
        :disabled="!ok"
        @click="answer(true)"
      >
        {{ pending?.confirmLabel ?? 'Confirmar' }}
      </button>
    </div>
  </Modal>
</template>
