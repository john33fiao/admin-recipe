<script setup lang="ts">
import type { OverviewObservation } from '~/types/overview'
import { displayDecimal } from '~/utils/catalog'
import { formatCreatedAt } from '~/utils/material'
import { noAllocationText } from '~/utils/overview'
defineProps<{ value: OverviewObservation; zone: string; detailed?: boolean }>()
</script>
<template>
  <div class="overview-observation">
    <p class="overview-measured"><strong>{{ displayDecimal(value.quantity) }}{{ value.unit }}</strong> <span>{{ value.mode === 'initial' ? '처음 확인한 기준' : '실사 정산' }}</span></p>
    <p>확인 시각 <time :datetime="value.observed_at">{{ formatCreatedAt(value.observed_at, zone) }}</time></p>
    <p class="field-hint">확인 당시 품목: <NuxtLink :to="`/materials/${value.material_id}`">{{ value.material_name }}</NuxtLink> · {{ value.unit }}</p>
    <StatisticsReferences :origin="value.origin" :changes="value.changes" :zone="zone" />
    <template v-if="detailed && value.consumption">
      <dl class="overview-values"><div><dt>실사 소모</dt><dd>{{ displayDecimal(value.consumption.quantity) }}{{ value.unit }} · {{ displayDecimal(value.consumption.value) }}원</dd></div><div><dt>초과 조정</dt><dd>{{ displayDecimal(value.consumption.excess_quantity) }}{{ value.unit }} · {{ displayDecimal(value.consumption.excess_value) }}원</dd></div><div><dt>미배분 소모</dt><dd>{{ displayDecimal(value.consumption.unallocated_quantity) }}{{ value.unit }} · {{ displayDecimal(value.consumption.unallocated_value) }}원</dd></div></dl>
      <p>{{ noAllocationText(value.consumption.reason) }} · 판매 후보 {{ value.consumption.candidate_count }}건</p>
    </template>
    <p v-else-if="detailed">처음 확인한 기준입니다. 소모가 계산된 정산 기록과 구별합니다.</p>
  </div>
</template>
