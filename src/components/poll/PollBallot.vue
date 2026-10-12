<script setup lang="ts">
import { ref } from 'vue'
import { t } from '@/i18n'
import Avatar from '@/components/common/Avatar.vue'
import type { PollOption } from '@/types/poll'

/** The racers to choose from and the vote button. Shared by the entry modal and the poll page. */
defineProps<{ options: PollOption[]; busy?: boolean; error?: string | null }>()
const emit = defineEmits<{ vote: [racerId: string] }>()
const chosen = ref<string | null>(null)
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
    <p v-if="error" class="mt-3 text-sm text-[#ff8aa0]" role="alert">{{ error }}</p>
    <div class="mt-5 flex flex-wrap items-center gap-3">
      <button
        type="button"
        class="btn btn-primary"
        :disabled="!chosen || busy"
        @click="chosen && emit('vote', chosen)"
      >
        {{ busy ? t('poll.sending') : t('poll.vote') }}
      </button>
      <slot name="extra" />
    </div>
    <p class="mt-3 text-xs text-white/60">{{ t('poll.oneVote') }}</p>
  </div>
</template>
