<script setup lang="ts">
import type { OverviewList, OverviewScope } from '~/types/overview'
import { overviewLabels, overviewDescriptions, overviewListUrl, overviewPageText } from '~/utils/overview'
defineProps<{ value: OverviewList; zone: string; expanded?: boolean }>()
const emit = defineEmits<{ openList: [scope: OverviewScope, offset: number] }>()
</script>
<template>
  <section class="operations-section" :data-overview-section="value.section">
    <h2>{{ overviewLabels[value.section] }}</h2><p class="field-hint">{{ overviewDescriptions[value.section] }}</p><p class="overview-page-count">{{ overviewPageText(value.page) }}</p>
    <details :open="expanded"><summary>{{ overviewLabels[value.section] }} 목록 보기</summary><OverviewRows :value="value" :zone="zone" @open-list="(scope, offset) => emit('openList', scope, offset)" /></details>
    <NuxtLink class="primary-link" :to="overviewListUrl({ section: value.section, material_id: value.section === 'material_changes' ? value.material.id : null })" @click="emit('openList', { section: value.section, material_id: value.section === 'material_changes' ? value.material.id : null }, 0)">{{ overviewLabels[value.section] }} 전체 보기</NuxtLink>
  </section>
</template>
