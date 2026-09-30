<script setup lang="ts">
import { t } from '@/i18n'
import GamingBackground from '@/components/common/GamingBackground.vue'
import SectionHeader from '@/components/common/SectionHeader.vue'
import FaqList from '@/components/rules/FaqList.vue'
import RulesList from '@/components/rules/RulesList.vue'
import { useSeo } from '@/composables/useSeo'

useSeo(
  () => t('meta.pages.rules'),
  () => t('meta.pages.rulesDesc'),
)
</script>

<template>
  <div class="relative pb-20">
    <GamingBackground />
    <div class="container-x pt-10 sm:pt-14">
      <SectionHeader
        :eyebrow="t('rulesPage.eyebrow')"
        :title="[t('rulesPage.title1'), t('rulesPage.title2')]"
        :subtitle="t('rulesPage.subtitle')"
      />
      <RulesList />
      <div class="mt-20">
        <SectionHeader :eyebrow="t('rulesPage.faqEyebrow')" :title="t('rulesPage.faqTitle')" />
        <FaqList />
      </div>
    </div>
  </div>
</template>
