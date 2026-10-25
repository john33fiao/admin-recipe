<script setup lang="ts">
import type { OverviewList, OverviewScope } from '~/types/overview'
import { overviewListUrl, overviewPageText } from '~/utils/overview'
import { displayDecimal } from '~/utils/catalog'
import { formatCreatedAt } from '~/utils/material'
defineProps<{ value: OverviewList; zone: string }>()
const emit = defineEmits<{ openList: [scope: OverviewScope, offset: number] }>()
const kindNames = { catalog_price: '사전 가격', purchase_order: '발주', receipt: '입고', production: '생산', sale: '판매', stocktake: '실사', waste: '폐기·직원식', correction: '정정', purchase_return: '구입 반품' }
</script>
<template>
  <p v-if="!value.page.rows.length" class="empty-state">이 목록에 표시할 기록이 없습니다.</p>
  <ol v-if="value.section === 'materials'" class="overview-records overview-materials">
    <li v-for="row in value.page.rows" :key="row.material.id" :data-overview-row="row.material.id">
      <h3><NuxtLink :to="`/materials/${row.material.id}`">{{ row.material.name }}</NuxtLink> <span v-if="row.material.archived" class="status-tag">보관</span></h3>
      <OverviewObservation v-if="row.last_observation" :value="row.last_observation" :zone="zone" /><p v-else class="overview-measured">아직 확인한 수량 없음</p>
      <NuxtLink :to="`/business?kind=stocktake&material=${row.material.id}`">이 품목 수량 확인</NuxtLink>
      <details class="overview-change-details"><summary>{{ row.changes_boundary.kind === 'not_observed' ? '수량 확인 전 등록된 확정 변동' : '마지막 확인 이후 확정 변동' }} {{ row.changes_after_observation.total_count }}건</summary>
        <p class="field-hint">{{ overviewPageText(row.changes_after_observation) }} · 변동을 확인 수량에 더해 현재고로 표시하지 않습니다.</p>
        <OverviewChanges :rows="row.changes_after_observation.rows" :zone="zone" />
        <NuxtLink v-if="row.changes_after_observation.total_count" class="primary-link" :to="overviewListUrl({ section: 'material_changes', material_id: row.material.id })" @click="emit('openList', { section: 'material_changes', material_id: row.material.id }, 0)">이 품목 변동 전체 보기</NuxtLink>
      </details>
    </li>
  </ol>
  <template v-else-if="value.section === 'material_changes'">
    <h2>{{ value.material.name }}</h2><p v-if="value.boundary.kind === 'observation'">기준 확인 시각 {{ formatCreatedAt(value.boundary.observed_at, zone) }} · <NuxtLink :to="`/stocktakes/${value.boundary.stocktake_id}`">실사 근거</NuxtLink></p><p v-else>아직 확인한 수량 없음 · 수량 확인 전 등록된 확정 변동입니다.</p>
    <OverviewChanges :rows="value.page.rows" :zone="zone" />
  </template>
  <ol v-else-if="value.section === 'open_orders'" class="overview-records">
    <li v-for="row in value.page.rows" :key="row.id" :data-overview-row="row.id"><h3><NuxtLink :to="`/orders/${row.id}`">{{ row.material_name }}</NuxtLink></h3><p>주문 {{ displayDecimal(row.quantity) }}{{ row.unit }} · 예상 금액 {{ row.estimated_amount === null ? '미입력' : `${displayDecimal(row.estimated_amount)}원` }}</p><p>예정일 {{ row.expected_date ?? '미입력' }} · 구입처 {{ row.supplier ?? '미입력' }}</p><NuxtLink :to="`/business?kind=receipt&order=${row.id}`">이 주문 입고 기록</NuxtLink></li>
  </ol>
  <ol v-else-if="value.section === 'received_orders'" class="overview-records">
    <li v-for="row in value.page.rows" :key="row.order.id" :data-overview-row="row.order.id"><h3><NuxtLink :to="`/orders/${row.order.id}`">{{ row.order.material_name }}</NuxtLink></h3><p>주문 {{ displayDecimal(row.closure.ordered_quantity) }}{{ row.closure.unit }} · 실제 입고 {{ displayDecimal(row.closure.received_quantity) }}{{ row.closure.unit }}</p><p>입고 − 주문 차이 {{ displayDecimal(row.closure.difference) }}{{ row.closure.unit }}</p><NuxtLink :to="`/receipts/${row.closure.receipt_id}`">현재 유효한 입고 근거</NuxtLink></li>
  </ol>
  <ol v-else-if="value.section === 'receipts'" class="overview-records">
    <li v-for="row in value.page.rows" :key="row.origin.id" :data-overview-row="row.origin.id"><h3>{{ row.effective.material_name }}</h3><p>현재 유효 입고 {{ displayDecimal(row.effective.quantity) }}{{ row.effective.unit }} · {{ displayDecimal(row.effective.amount) }}원 · 입고일 {{ row.effective.receipt_date }}</p><details><summary>원입고 값</summary><p>{{ row.original.material_name }} · {{ displayDecimal(row.original.quantity) }}{{ row.original.unit }} · {{ displayDecimal(row.original.amount) }}원 · 입고일 {{ row.original.receipt_date }}</p></details>
      <template v-if="row.reconciliation"><p>이미 포함 {{ displayDecimal(row.reconciliation.included_quantity) }}{{ row.effective.unit }} · {{ displayDecimal(row.reconciliation.included_amount) }}원</p><p>새 유입 {{ displayDecimal(row.reconciliation.unreflected_quantity) }}{{ row.effective.unit }} · {{ displayDecimal(row.reconciliation.unreflected_amount) }}원</p><p>입고 전체 가치 차이 {{ displayDecimal(row.reconciliation.value_difference) }}원 · 별도 보정 {{ displayDecimal(row.reconciliation.separate_adjustment) }}원</p></template>
      <p class="field-hint">원확정 {{ formatCreatedAt(row.origin.recorded_at, zone) }}</p><StatisticsReferences :origin="row.origin" :changes="row.changes" :zone="zone" />
    </li>
  </ol>
  <ol v-else-if="value.section === 'sales'" class="overview-records">
    <li v-for="row in value.page.rows" :key="row.origin.id" :data-overview-row="row.origin.id"><h3>{{ row.material_name }}</h3><p>{{ displayDecimal(row.count) }}개 · {{ row.amount === null ? '판매금액 미계산' : `${displayDecimal(row.amount)}원` }}</p><p>직접 구성 품목: 소모 배분 {{ row.consumption_counts.allocated }}건 · 미배분 {{ row.consumption_counts.unallocated }}건 · 미확정 {{ row.consumption_counts.unconfirmed }}건</p><p class="field-hint">원확정 {{ formatCreatedAt(row.origin.recorded_at, zone) }}</p><StatisticsReferences :origin="row.origin" :changes="row.changes" :zone="zone" /></li>
  </ol>
  <ol v-else-if="value.section === 'observations'" class="overview-records">
    <li v-for="row in value.page.rows" :key="row.observation_id" :data-overview-row="row.observation_id"><h3>{{ row.material_name }}</h3><OverviewObservation :value="row" :zone="zone" detailed /></li>
  </ol>
  <ol v-else-if="value.section === 'unallocated'" class="overview-records">
    <li v-for="row in value.page.rows" :key="row.observation.observation_id" :data-overview-row="row.observation.observation_id"><h3>{{ row.observation.material_name }}</h3><OverviewObservation :value="row.observation" :zone="zone" detailed /></li>
  </ol>
  <ol v-else-if="value.section === 'unconfirmed'" class="overview-records">
    <li v-for="row in value.page.rows" :key="`${row.sale.origin.id}-${row.material.id}`" :data-overview-row="`${row.sale.origin.id}-${row.material.id}`"><h3>{{ row.sale.material_name }} 판매 · {{ row.material.name }}</h3><p>판매 {{ displayDecimal(row.sale.count) }}개 · {{ row.reason === 'initial_basis_missing' ? '처음 확인한 기준만 있어 소모를 계산하지 못했습니다.' : '유효한 실사 소모 근거가 아직 연결되지 않았습니다.' }}</p><p v-if="row.initial_observation"><NuxtLink :to="`/stocktakes/${row.initial_observation.stocktake_id}`">처음 확인한 수량 근거</NuxtLink> · {{ formatCreatedAt(row.initial_observation.observed_at, zone) }}</p><p><NuxtLink :to="`/sales/${row.sale.origin.id}`">판매 상세</NuxtLink> · <NuxtLink :to="`/business?kind=stocktake&material=${row.material.id}`">이 품목 수량 확인</NuxtLink></p></li>
  </ol>
  <ol v-else-if="value.section === 'drafts'" class="overview-records">
    <li v-for="row in value.page.rows" :key="row.id" :data-overview-row="row.id"><h3><NuxtLink :to="`/business/${row.id}`">{{ kindNames[row.kind] }} · {{ row.material_name ?? '대상 확인 필요' }}</NuxtLink></h3><p>DB 임시 저장 · 확정 가능 여부 미평가</p><p>마지막 저장 {{ formatCreatedAt(row.updated_at, zone) }}</p></li>
  </ol>
</template>
