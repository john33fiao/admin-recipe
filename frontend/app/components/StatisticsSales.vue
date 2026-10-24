<script setup lang="ts">
import type { StatisticsReport } from '~/types/statistics'
import { coverageMessage, saleChartRows, type SalesMetric } from '~/utils/statistics'
import { displayDecimal } from '~/utils/catalog'
import { formatCreatedAt } from '~/utils/material'
const props = defineProps<{ value: StatisticsReport['sales']; zone: string }>()
const metric = ref<SalesMetric>('amount'), selectedId = ref('')
const points = computed(() => saleChartRows(props.value.rows, metric.value))
const selected = computed(() => props.value.rows.find(row => row.origin.id === selectedId.value) ?? props.value.rows[0])
const point = computed(() => points.value.find(row => row.id === selected.value?.origin.id))
const highest = computed(() => points.value.filter(row => row.y !== null).reduce<(typeof points.value)[number] | null>((best, row) => !best || row.y! < best.y! ? row : best, null))
const timeLimits = computed(() => [...points.value].sort((a, b) => Date.parse(a.time) - Date.parse(b.time)))
const reasons = { no_effective_allocation: '연결된 유효 배분 없음 · 소모 미확정', initial_basis_missing: '시작 기준 이전 소모 미확정', zero_weight: '대상 판매 총개수 0 · 미배분' }
</script>
<template>
  <section class="statistics-section" aria-labelledby="statistics-sales-title">
    <h2 id="statistics-sales-title">판매와 당시 레시피 원가</h2>
    <p>{{ coverageMessage(value.coverage, '판매') }}</p>
    <dl v-if="value.rows.length" class="statistics-totals"><div><dt>현재 유효 판매 개수</dt><dd>{{ displayDecimal(value.summary.count) }}개</dd></div><div><dt>{{ value.summary.missing_revenue_records ? '알려진 판매금액 소계' : '판매금액 합계' }}</dt><dd>{{ displayDecimal(value.summary.known_revenue) }}원</dd></div><div><dt>판매금액 미계산</dt><dd>{{ value.summary.missing_revenue_records }}건</dd></div><div><dt>취소된 판매</dt><dd>{{ value.summary.void_records }}건</dd></div></dl>
    <template v-if="value.rows.length">
      <p class="field-hint">현재 유효한 개수·금액입니다. 미계산 금액과 취소는 합계에 넣지 않습니다. 최초 개수·금액은 원기록의 최초 버전에서 확인하세요.</p>
      <figure class="statistics-chart" aria-labelledby="sales-chart-caption">
        <figcaption id="sales-chart-caption">원확정 시각별 판매 · 표와 같은 기록</figcaption>
        <div class="field"><label for="statistics-metric">시계열 값</label><select id="statistics-metric" v-model="metric"><option value="amount">판매금액 (원)</option><option value="count">판매 개수 (개)</option></select></div>
        <p class="chart-scale">세로축: 0{{ metric === 'amount' ? '원' : '개' }} — {{ highest?.label ?? '계산할 값 없음' }}</p>
        <svg viewBox="0 0 1000 210" role="img" :aria-label="`${metric === 'amount' ? '판매금액' : '판매 개수'} 시계열. 아래 표에서 기록을 선택해 정확한 값을 확인하세요.`"><line x1="50" y1="180" x2="950" y2="180" class="chart-axis"/><text x="30" y="185">0</text><template v-for="entry in points" :key="entry.id"><circle v-if="entry.y !== null" :cx="entry.x" :cy="entry.y" :r="selected?.origin.id === entry.id ? 8 : 5" :class="['chart-point', { selected: selected?.origin.id === entry.id }]" /></template></svg>
        <p v-if="timeLimits.length" class="chart-scale">가로축: {{ formatCreatedAt(timeLimits[0]!.time, zone) }} — {{ formatCreatedAt(timeLimits.at(-1)!.time, zone) }} ({{ zone }})</p>
        <p v-if="point" role="status" class="chart-selection">선택한 기록: {{ point.name }} · {{ formatCreatedAt(point.time, zone) }} · <strong>{{ point.label }}</strong></p>
        <p class="field-hint">날짜별로 묶거나 빈 날에 0을 만들지 않습니다. 미계산·취소는 점을 그리지 않습니다.</p>
      </figure>
      <div class="statistics-table-scroll" tabindex="0" role="region" aria-label="판매 기록 표"><table class="statistics-table"><caption>원시각과 현재 유효 판매</caption><thead><tr><th scope="col">당시 메뉴 · 시각</th><th scope="col">개수</th><th scope="col">판매금액</th><th scope="col">상태 · 근거</th></tr></thead><tbody><tr v-for="row in value.rows" :key="row.origin.id" :data-sale-id="row.origin.id" :class="{ selected: selected?.origin.id === row.origin.id }"><th scope="row"><button type="button" class="text-button" :aria-pressed="selected?.origin.id === row.origin.id" aria-controls="statistics-sale-detail" @click="selectedId = row.origin.id">{{ row.original.material_name }}</button><time :datetime="row.origin.recorded_at">{{ formatCreatedAt(row.origin.recorded_at, zone) }}</time></th><td>{{ row.status === 'void' ? '취소됨' : row.value ? displayDecimal(row.value.count) + '개' : '미확인' }}</td><td>{{ row.status === 'void' ? '합산 제외' : row.value?.amount === null || !row.value ? '미계산' : displayDecimal(row.value.amount) + '원' }}</td><td><span>{{ row.changes.length ? '정정 반영' : '원확정 유지' }}</span><NuxtLink :to="`/sales/${row.origin.id}`">판매 상세</NuxtLink></td></tr></tbody></table></div>
      <section v-if="selected" id="statistics-sale-detail" class="statistics-selected" aria-labelledby="statistics-selected-sale-title"><h3 id="statistics-selected-sale-title">{{ selected.original.material_name }} · 선택한 판매의 근거</h3><StatisticsReferences :origin="selected.origin" :changes="selected.changes" :zone="zone" /><p><NuxtLink :to="`/recipes/${selected.original.material_id}/versions/${selected.original.recipe_version_id}`">판매 당시 레시피</NuxtLink></p>
        <details><summary>당시 레시피 기준 원가 · 1개 기준</summary><SaleCostSnapshot :cost="selected.original.recipe_cost" /></details>
        <h4>실사 소모에서 배분한 수량·원가</h4><p class="field-hint">판매 개수 비율로 배분한 값입니다. 당시 레시피 기준 원가와 다르며 두 원가를 더하지 않습니다.</p>
        <p v-if="selected.status === 'void'">취소된 판매는 현재 배분 대상에서 제외합니다. 원기록을 보존합니다.</p><p v-else-if="!selected.consumption.length">당시 직접 구성 품목이 없습니다. 소모 0을 뜻하지 않습니다.</p>
        <ul v-else class="history-list"><li v-for="row in selected.consumption" :key="row.material_id"><NuxtLink :to="`/materials/${row.material_id}`">{{ row.material_name }}</NuxtLink><p>{{ row.quantity === null ? '수량 미확정' : displayDecimal(row.quantity) + row.unit }} · {{ row.value === null ? '원가 미확정' : displayDecimal(row.value) + '원' }}</p><p v-if="row.reason">{{ reasons[row.reason] }}</p><p v-if="row.stocktake_id"><NuxtLink :to="`/stocktakes/${row.stocktake_id}`">배분 근거 실사</NuxtLink> <strong v-if="row.observation_in_period === false">· 조회 기간 밖 실사</strong><time v-if="row.observation_recorded_at"> · {{ formatCreatedAt(row.observation_recorded_at, zone) }}</time></p></li></ul>
      </section>
    </template>
  </section>
</template>
