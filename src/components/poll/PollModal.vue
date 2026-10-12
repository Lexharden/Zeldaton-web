<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { t } from '@/i18n'
import { analyticsEnabled, bannerOpen } from '@/analytics/analytics'
import PollBallot from '@/components/poll/PollBallot.vue'
import { usePollStore } from '@/stores/poll'

/**
 * Invites each new visitor to vote on who will win, then takes them to the results. Appears once the
 * page has settled, never over the cookie notice, and not again for a day after "Not now".
 */
const poll = usePollStore()
const route = useRoute()
const router = useRouter()
const ready = ref(false)
const dismissed = ref(false)
const panel = ref<HTMLElement | null>(null)
let timer: ReturnType<typeof setTimeout> | null = null

const show = computed(
  () =>
    ready.value &&
    !dismissed.value &&
    poll.shouldPrompt &&
    route.name !== 'poll' &&
    !(analyticsEnabled && bannerOpen.value),
)

onMounted(() => {
  void poll.load()
  timer = setTimeout(() => (ready.value = true), 1500)
})
onBeforeUnmount(() => {
  if (timer) clearTimeout(timer)
  window.removeEventListener('keydown', onKey)
})

function notNow() {
  dismissed.value = true
  poll.snooze()
}

function onKey(e: KeyboardEvent) {
  if (e.key === 'Escape') notNow()
}

watch(show, async (open) => {
  if (open) {
    window.addEventListener('keydown', onKey)
    await nextTick()
    panel.value?.focus()
  } else {
    window.removeEventListener('keydown', onKey)
  }
})

async function cast(racerId: string) {
  if (await poll.vote(racerId)) await router.push({ name: 'poll' })
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="show"
      class="fixed inset-0 z-[85] flex items-start justify-center overflow-y-auto bg-black/75 p-4 backdrop-blur-sm sm:items-center"
      @mousedown.self="notNow"
    >
      <div
        ref="panel"
        role="dialog"
        aria-modal="true"
        aria-labelledby="poll-title"
        tabindex="-1"
        class="my-8 w-full max-w-2xl border border-accent/50 bg-surface p-5 shadow-2xl outline-none sm:p-7"
      >
        <p class="hud-label !text-accent">{{ t('poll.eyebrow') }}</p>
        <h2 id="poll-title" class="display mt-2 text-4xl text-white sm:text-5xl">
          {{ t('poll.title') }}
        </h2>
        <p class="mb-5 mt-2 text-sm text-white/80">{{ t('poll.modalText') }}</p>
        <PollBallot
          v-if="poll.poll"
          :options="poll.poll.options"
          :busy="poll.busy"
          :error="poll.error"
          @vote="cast"
        >
          <template #extra>
            <button
              type="button"
              class="px-3 py-2 font-display text-lg uppercase tracking-wider text-white/70 hover:text-white"
              @click="notNow"
            >
              {{ t('poll.notNow') }}
            </button>
          </template>
        </PollBallot>
      </div>
    </div>
  </Teleport>
</template>
