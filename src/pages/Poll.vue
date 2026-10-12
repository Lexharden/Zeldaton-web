<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted } from 'vue'
import { t } from '@/i18n'
import GamingBackground from '@/components/common/GamingBackground.vue'
import PollBallot from '@/components/poll/PollBallot.vue'
import PollResults from '@/components/poll/PollResults.vue'
import SkeletonBlock from '@/components/common/SkeletonBlock.vue'
import { useSeo } from '@/composables/useSeo'
import { usePollStore } from '@/stores/poll'

useSeo(
  () => t('meta.pages.poll'),
  () => t('meta.pages.pollDesc'),
)
const store = usePollStore()
const poll = computed(() => store.poll)
const showResults = computed(() => !!poll.value?.results)

let timer: ReturnType<typeof setInterval> | null = null
onMounted(() => {
  void store.load()
  // Votes keep coming in: refresh the bars while the tab is in view.
  timer = setInterval(() => {
    if (!document.hidden && showResults.value) void store.load()
  }, 30000)
})
onBeforeUnmount(() => timer && clearInterval(timer))
</script>

<template>
  <div class="relative pb-20">
    <GamingBackground shapes />
    <div class="container-x max-w-3xl pt-10 sm:pt-14">
      <header class="mb-8">
        <p class="hud-label mb-3">{{ t('poll.eyebrow') }}</p>
        <h1 class="display text-[clamp(2.6rem,8vw,5rem)] text-white">{{ t('poll.title') }}</h1>
        <p class="mt-3 text-white/80">{{ t('poll.subtitle') }}</p>
      </header>

      <SkeletonBlock v-if="!poll && store.loading" class="h-64" />
      <p v-else-if="!poll" class="text-white/80" role="alert">
        {{ store.error ?? t('states.unknownError') }}
      </p>
      <template v-else>
        <p
          v-if="!poll.open"
          class="mb-5 border border-line bg-white/[0.04] p-3 text-sm text-white/85"
        >
          {{ t('poll.closed') }}
        </p>
        <PollBallot
          v-if="store.canVote"
          :options="poll.options"
          :busy="store.busy"
          :error="store.error"
          @vote="store.vote"
        />
        <template v-else-if="showResults">
          <p v-if="poll.voted" class="mb-5 text-white/85">{{ t('poll.thanks') }}</p>
          <PollResults :poll="poll" />
        </template>
        <p v-else class="text-white/80">{{ t('poll.empty') }}</p>
      </template>
    </div>
  </div>
</template>
