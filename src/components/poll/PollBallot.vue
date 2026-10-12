<script setup lang="ts">
import { computed, ref } from 'vue'
import { t } from '@/i18n'
import Avatar from '@/components/common/Avatar.vue'
import { NAME_MAX, NAME_MIN, type PollOption } from '@/types/poll'

/** The racers to choose from and the vote button. Shared by the entry modal and the poll page. */
defineProps<{ options: PollOption[]; busy?: boolean; error?: string | null }>()
const emit = defineEmits<{ vote: [racerId: string, name: string] }>()
const chosen = ref<string | null>(null)
const name = ref('')
const nameOk = computed(() => {
  const n = name.value.trim().replace(/\s+/g, ' ')
  return n.length >= NAME_MIN && n.length <= NAME_MAX
})
</script>

<template>
  <div>
    <div
      role="radiogroup"
      :aria-label="t('poll.choose')"
      class="grid grid-cols-2 gap-3 sm:grid-cols-3"
    >
      <button
        v-for="o in options"
        :key="o.racerId"
        type="button"
        role="radio"
        :aria-checked="chosen === o.racerId"
        class="flex flex-col items-center gap-2 border p-3 text-center transition-colors"
        :class="
          chosen === o.racerId
            ? 'border-accent bg-accent/15'
            : 'border-line bg-white/[0.03] hover:border-white/40'
        "
        @click="chosen = o.racerId"
      >
        <Avatar :name="o.name" :src="o.avatarUrl" :size="64" />
        <span class="font-display text-lg font-semibold uppercase tracking-wide text-white">{{
          o.name
        }}</span>
      </button>
    </div>
    <label class="mt-5 block">
      <span class="hud-label">{{ t('poll.nameLabel') }}</span>
      <input
        v-model="name"
        type="text"
        :maxlength="NAME_MAX"
        autocomplete="name"
        :placeholder="t('poll.namePlaceholder')"
        class="mt-2 w-full border border-line bg-white/[0.05] px-3 py-2 text-white placeholder:text-white/40 focus:border-accent focus:outline-none"
        @keydown.enter.prevent="chosen && nameOk && !busy && emit('vote', chosen, name)"
      />
    </label>
    <p v-if="error" class="mt-3 text-sm text-[#ff8aa0]" role="alert">{{ error }}</p>
    <div class="mt-5 flex flex-wrap items-center gap-3">
      <button
        type="button"
        class="btn btn-primary"
        :disabled="!chosen || !nameOk || busy"
        @click="chosen && emit('vote', chosen, name)"
      >
        {{ busy ? t('poll.sending') : t('poll.vote') }}
      </button>
      <slot name="extra" />
    </div>
    <p class="mt-3 text-xs text-white/60">{{ t('poll.oneVote') }}</p>
  </div>
</template>
