<script setup lang="ts">
import { t } from '@/i18n'
import { RouterLink } from 'vue-router'
import { ART } from '@/config/artwork'
import GamingBackground from '@/components/common/GamingBackground.vue'
import SectionDivider from '@/components/common/SectionDivider.vue'
import SectionHeader from '@/components/common/SectionHeader.vue'
import DailyClockSection from '@/components/countdown/DailyClockSection.vue'
import HeroSection from '@/components/hero/HeroSection.vue'
import ActivityFeed from '@/components/hiveshock/ActivityFeed.vue'
import HiveShockSection from '@/components/hiveshock/HiveShockSection.vue'
import HowItWorks from '@/components/race/HowItWorks.vue'
import RaceIntro from '@/components/race/RaceIntro.vue'
import DonorsPodium from '@/components/donors/DonorsPodium.vue'
import RacersGrid from '@/components/racers/RacersGrid.vue'
import FaqList from '@/components/rules/FaqList.vue'
import StandingsTable from '@/components/standings/StandingsTable.vue'
import StreamGrid from '@/components/streams/StreamGrid.vue'
import Button from '@/components/ui/Button.vue'
import { useSeo } from '@/composables/useSeo'
import { useDonorsStore } from '@/stores/donors'
import { useRacersStore } from '@/stores/racers'

/** Marketing / event introduction. The live dashboard itself lives on /race. */
useSeo()
const racers = useRacersStore()
const donors = useDonorsStore()
</script>

<template>
  <div>
    <HeroSection />
    <SectionDivider />
    <RaceIntro />
    <DailyClockSection />

    <section id="racers" class="section" aria-labelledby="racers-title">
      <GamingBackground shapes :grid="false" :art="ART.bgForest" />
      <div class="container-x">
        <SectionHeader
          id="racers-title"
          index="03"
          :eyebrow="t('racers.eyebrow')"
          :title="t('racers.title')"
          :subtitle="t('racers.subtitle', { n: racers.list.length || 8 })"
        />
        <RacersGrid />
      </div>
    </section>

    <section id="standings" class="section" aria-labelledby="standings-title">
      <div class="container-x">
        <div class="mb-10 flex flex-wrap items-end justify-between gap-6 md:mb-14">
          <SectionHeader
            id="standings-title"
            class="!mb-0"
            index="04"
            :eyebrow="t('standings.eyebrow')"
            :title="t('standings.title')"
            :subtitle="t('standings.subtitle')"
          />
          <Button to="/race" variant="ghost">{{ t('standings.open') }}</Button>
        </div>
        <StandingsTable :limit="5" />
      </div>
    </section>

    <section v-if="donors.visible" id="donors" class="section" aria-labelledby="donors-title">
      <GamingBackground shapes :grid="false" />
      <div class="container-x">
        <SectionHeader
          id="donors-title"
          index="05"
          :eyebrow="t('donors.eyebrow')"
          :title="t('donors.title')"
          :subtitle="t('donors.subtitle')"
        />
        <DonorsPodium />
      </div>
    </section>

    <section id="streams" class="section" aria-labelledby="streams-title">
      <GamingBackground :grid="false" :art="ART.bgField" />
      <div class="container-x">
        <div class="mb-10 flex flex-wrap items-end justify-between gap-6 md:mb-14">
          <SectionHeader
            id="streams-title"
            class="!mb-0"
            index="06"
            :eyebrow="t('streams.eyebrow')"
            :title="t('streams.title')"
            :subtitle="t('streams.subtitle')"
          />
          <Button to="/streams?live=1" variant="primary">{{ t('streams.watchAllLive') }}</Button>
        </div>
        <StreamGrid :limit="4" />
        <div class="mt-16 grid gap-8 lg:grid-cols-[0.8fr_1.2fr]">
          <div>
            <p class="hud-label mb-2">{{ t('activity.right') }}</p>
            <h3 class="display text-5xl text-white">{{ t('activity.title') }}</h3>
            <p class="mt-3 max-w-sm text-sm text-muted">
              {{ t('activity.text') }}
            </p>
          </div>
          <ActivityFeed :limit="7" />
        </div>
      </div>
    </section>

    <HiveShockSection />

    <section id="how" class="section" aria-labelledby="how-title">
      <div class="container-x">
        <SectionHeader
          id="how-title"
          index="08"
          :eyebrow="t('how.eyebrow')"
          :title="t('how.title')"
        />
        <HowItWorks />
      </div>
    </section>

    <section id="rules" class="section" aria-labelledby="faq-title">
      <div class="container-x grid gap-10 lg:grid-cols-[0.8fr_1.2fr]">
        <div>
          <SectionHeader
            id="faq-title"
            index="09"
            :eyebrow="t('rulesPage.home.eyebrow')"
            :title="[t('rulesPage.home.title1'), t('rulesPage.home.title2')]"
          />
          <RouterLink to="/rules" class="btn btn-ghost">{{ t('rulesPage.home.full') }}</RouterLink>
        </div>
        <FaqList />
      </div>
    </section>
  </div>
</template>
