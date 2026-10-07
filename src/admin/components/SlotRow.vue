<script setup lang="ts">
import { computed } from 'vue'
import { Hand, Trash2, Undo2 } from 'lucide-vue-next'
import { clock, SLOT_LABEL, type SlotState } from '../schedule'
import type { ScheduleSlot } from '../types'

/** One schedule slot: who streams when, who referees it, and whether it is on track. */
const props = defineProps<{
  entry: ScheduleSlot
  state: SlotState
  /** The signed-in referee, to offer "Tomar" or "Dejar". */
  userId: number | null
  /** Admins can remove the slot. */
  canRemove?: boolean
  busy?: boolean
}>()
defineEmits<{ take: []; leave: []; remove: [] }>()

const mine = computed(() => props.entry.assignees.some((a) => a.userId === props.userId))
const covered = computed(() => props.entry.assignees.length > 0)
const sameZone = computed(
  () => clock(props.entry.startUtc) === clock(props.entry.startUtc, props.entry.racerTimezone),
)
const tone: Record<SlotState, string> = {
  done: 'border-line text-muted',
  upcoming: 'border-secondary/40 text-secondary',
  starting: 'border-warning/50 text-warning',
  live: 'border-success/50 text-success',
  late: 'border-danger/60 text-[#ff8aa0]',
}
</script>

<template>
  <li
    class="flex flex-wrap items-center gap-x-4 gap-y-2 rounded border bg-white/[0.03] px-3 py-2 text-sm"
    :class="tone[state]"
    :data-slot="entry.id"
  >
    <div class="num w-28 shrink-0 text-white">
      {{ clock(entry.startUtc) }} – {{ clock(entry.endUtc) }}
      <span v-if="!sameZone" class="block text-xs text-muted"
        >{{ clock(entry.startUtc, entry.racerTimezone) }} –
        {{ clock(entry.endUtc, entry.racerTimezone) }} allá</span
      >
    </div>
    <div class="min-w-0 flex-1">
      <p class="truncate font-semibold text-white">
        {{ entry.racerName }}
        <span v-if="entry.note" class="font-normal text-muted">· {{ entry.note }}</span>
      </p>
      <p class="text-xs" :class="covered ? 'text-muted' : 'text-warning'">
        {{
          covered ? `Árbitro: ${entry.assignees.map((a) => a.username).join(', ')}` : 'Sin árbitro'
        }}
      </p>
    </div>
    <span class="hud-label !text-inherit">{{ SLOT_LABEL[state] }}</span>
    <div v-if="state !== 'done'" class="flex gap-2">
      <button
        v-if="!mine"
        type="button"
        class="a-btn a-btn-sm"
        :disabled="busy"
        @click="$emit('take')"
      >
        <Hand class="size-3.5" />Tomar
      </button>
      <button v-else type="button" class="a-btn a-btn-sm" :disabled="busy" @click="$emit('leave')">
        <Undo2 class="size-3.5" />Dejar
      </button>
      <button
        v-if="canRemove"
        type="button"
        class="a-btn a-btn-sm a-btn-danger"
        :disabled="busy"
        :aria-label="`Quitar el horario de ${entry.racerName}`"
        @click="$emit('remove')"
      >
        <Trash2 class="size-3.5" />
      </button>
    </div>
  </li>
</template>
