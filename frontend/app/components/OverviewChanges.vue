<script setup lang="ts">
import type { OverviewMaterialChange } from '~/types/overview'
import { displayDecimal } from '~/utils/catalog'
import { formatCreatedAt } from '~/utils/material'
const labels = { receipt: '입고', reconciled_receipt: '이미 포함한 수량을 확인한 입고', production: '생산', purchase_return: '구입 반품', waste: '폐기·직원식' }
defineProps<{ rows: OverviewMaterialChange[]; zone: string }>()
</script>
<template>
  <ol class="overview-records overview-changes">
    <li v-for="row in rows" :key="`${row.origin.kind}-${row.origin.id}`" :data-overview-row="row.origin.id">
      <h3>{{ labels[row.effect.kind] }} · {{ row.material.name }}</h3><p class="field-hint">원확정 <time :datetime="row.origin.recorded_at">{{ formatCreatedAt(row.origin.recorded_at, zone) }}</time></p>
      <template v-if="row.effect.kind === 'receipt'"><p>입고 {{ displayDecimal(row.effect.quantity) }}{{ row.material.unit }} · {{ displayDecimal(row.effect.amount) }}원</p><p>입고일 {{ row.effect.receipt_date }}</p></template>
      <template v-else-if="row.effect.kind === 'reconciled_receipt'">
        <p>실제 구입 {{ row.effect.receipt.material_name }} · {{ displayDecimal(row.effect.receipt.quantity) }}{{ row.effect.receipt.unit }} · {{ displayDecimal(row.effect.receipt.amount) }}원 · 입고일 {{ row.effect.receipt.receipt_date }}</p>
        <p>이미 포함한 구입량 {{ displayDecimal(row.effect.included_quantity) }}{{ row.effect.receipt.unit }} · {{ displayDecimal(row.effect.included_amount) }}원</p>
        <p>이 품목 새 유입 {{ displayDecimal(row.effect.inflow_quantity) }}{{ row.material.unit }} · {{ displayDecimal(row.effect.inflow_value) }}원</p>
        <p>이 품목 가치 조정 {{ displayDecimal(row.effect.value_delta) }}원</p><p class="field-hint">입고 전체 별도 보정 {{ displayDecimal(row.effect.separate_adjustment) }}원 · 품목별로 더하지 않습니다.</p>
      </template>
      <template v-else-if="row.effect.kind === 'production'"><p>투입 {{ displayDecimal(row.effect.input_quantity) }}{{ row.material.unit }} · {{ displayDecimal(row.effect.input_value) }}원</p><p>결과 {{ displayDecimal(row.effect.output_quantity) }}{{ row.material.unit }} · {{ displayDecimal(row.effect.output_value) }}원</p></template>
      <template v-else-if="row.effect.kind === 'purchase_return'"><p>반품 {{ displayDecimal(row.effect.quantity) }}{{ row.material.unit }} · 차감 가치 {{ displayDecimal(row.effect.deducted_value) }}원</p><NuxtLink :to="`/receipts/${row.effect.source_receipt_id}`">반품의 입고 근거</NuxtLink></template>
      <p v-else>폐기·직원식 {{ displayDecimal(row.effect.quantity) }}{{ row.material.unit }} · 손실 {{ displayDecimal(row.effect.loss_value) }}원</p>
      <StatisticsReferences :origin="row.origin" :changes="row.changes" :zone="zone" />
    </li>
  </ol>
</template>
