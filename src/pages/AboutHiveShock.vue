<script setup lang="ts">
import { t } from '@/i18n'
import ArtImage from '@/components/common/ArtImage.vue'
import { ART } from '@/config/artwork'
import GamingBackground from '@/components/common/GamingBackground.vue'
import SectionHeader from '@/components/common/SectionHeader.vue'
import ActivityFeed from '@/components/hiveshock/ActivityFeed.vue'
import CapabilityList from '@/components/hiveshock/CapabilityList.vue'
import EventTerminal from '@/components/hiveshock/EventTerminal.vue'
import HiveFlow from '@/components/hiveshock/HiveFlow.vue'
import HiveShockStats from '@/components/hiveshock/HiveShockStats.vue'
import PoweredBy from '@/components/hiveshock/PoweredBy.vue'
import { useSeo } from '@/composables/useSeo'

useSeo(
  () => t('meta.pages.hiveshock'),
  () => t('meta.pages.hiveshockDesc'),
)
</script>

<template>
  <div class="relative pb-20">
    <section class="relative overflow-hidden pb-16 pt-12 sm:pt-20">
      <GamingBackground variant="hero" shapes particles />
      <ArtImage
        :src="ART.hiveshockLogo"
        alt=""
        class="pointer-events-none absolute right-[4%] top-1/2 hidden h-[78%] -translate-y-1/2 motion-safe:animate-[float_9s_ease-in-out_infinite] lg:block"
      />
      <div class="container-x relative">
        <p class="hud-label mb-5">{{ t('hiveshock.eyebrow') }}</p>
        <h1 class="display text-[clamp(3.2rem,11vw,8.5rem)] text-white">
          <span class="block">{{ t('hiveshock.title1') }}</span
          ><span class="block">{{ t('hiveshock.title2') }}</span
          ><span class="text-gradient block">{{ t('hiveshock.title3') }}</span>
        </h1>
        <p class="mt-6 max-w-xl text-lg text-white/85">
          {{ t('hiveshock.tagline') }}
        </p>
        <div class="mt-8"><PoweredBy /></div>
      </div>
    </section>

    <div class="container-x space-y-20">
      <section aria-labelledby="flow">
        <SectionHeader
          id="flow"
          :eyebrow="t('hiveshock.pipeline')"
          :title="t('hiveshock.pipelineTitle')"
          :subtitle="t('hiveshock.pipelineSub')"
        /><HiveFlow />
      </section>
      <section aria-labelledby="cap">
        <SectionHeader
          id="cap"
          :eyebrow="t('hiveshock.capsEyebrow')"
          :title="t('hiveshock.capsTitle')"
          :subtitle="t('hiveshock.capsSub')"
        /><CapabilityList />
      </section>
      <section aria-labelledby="live">
        <SectionHeader
          id="live"
          :eyebrow="t('hiveshock.telemetry')"
          :title="t('hiveshock.live')"
          :subtitle="t('hiveshock.telemetrySub')"
        />
        <HiveShockStats />
        <div class="mt-8 grid gap-8 lg:grid-cols-2">
          <EventTerminal :limit="8" /><ActivityFeed :limit="8" />
        </div>
      </section>
      <section class="panel panel-gold panel-lg" :aria-label="t('hiveshock.licensing')">
        <div class="p-8 sm:p-12">
          <p class="hud-label !text-accent">{{ t('hiveshock.freeLabel') }}</p>
          <p class="display mt-3 text-4xl text-white sm:text-6xl">
            {{ t('hiveshock.free') }}
          </p>
          <p class="mt-4 max-w-2xl text-muted">
            {{ t('hiveshock.freeBody') }}
          </p>
        </div>
      </section>
    </div>
  </div>
</template>
