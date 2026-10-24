<script setup lang="ts">
import type { StocktakeRowEffect } from '~/types/stocktake'
import { displayDecimal, priceSources } from '~/utils/catalog'
import { formatCreatedAt } from '~/utils/material'
defineProps<{ rows: StocktakeRowEffect[]; unchecked?: number[] }>()
</script>
<template>
  <div class="stocktake-results">
    <p v-if="unchecked?.length" class="field-hint">수량을 확인하지 않은 {{ unchecked.length }}행은 이전 수량과 확인 시각을 유지합니다.</p>
    <section v-for="row in rows" :key="row.position" class="stocktake-result">
      <h3><NuxtLink :to="`/materials/${row.material_id}`">{{ row.material_name }}</NuxtLink> <span class="status-label">{{ row.calculation.mode === 'initial' ? '시작 기준 확인' : '실사 정산' }}</span></h3>
      <dl class="stocktake-fields"><div><dt>확인한 실제 수량</dt><dd>{{ displayDecimal(row.measured_quantity) }}{{ row.unit }} <span class="field-hint">입력 {{ row.input_quantity }}{{ row.input_unit }}</span></dd></div><div><dt>확인 전 정산 기준</dt><dd>{{ row.before ? `${displayDecimal(row.before.quantity)}${row.unit} · ${row.before.value === null ? '가치 미확인' : displayDecimal(row.before.value) + '원'}` : '시작 기준 없음' }}</dd></div><div><dt>확인 후 정산 기준</dt><dd>{{ displayDecimal(row.after.quantity) }}{{ row.unit }} · {{ row.after.value === null ? '가치 미확인' : `${displayDecimal(row.after.value)}원` }}</dd></div><div><dt>이전 확인 수량</dt><dd>{{ row.observation_before ? `${displayDecimal(row.observation_before.quantity)}${row.unit} · ${formatCreatedAt(row.observation_before.recorded_at)}` : '아직 확인한 수량 없음' }}</dd></div></dl>
      <p v-if="row.calculation.mode === 'initial'" class="notice">실제 시작 수량과 가치를 확인합니다. 시작 이전 소모는 미확정이며 이전 판매에 0을 배분하지 않습니다.</p>
      <template v-else><dl class="stocktake-fields"><div><dt>확정 소모</dt><dd>{{ displayDecimal(row.calculation.consumption_quantity) }}{{ row.unit }} · {{ displayDecimal(row.calculation.consumption_value) }}원</dd></div><div><dt>초과 조정</dt><dd>{{ displayDecimal(row.calculation.excess_quantity) }}{{ row.unit }} · {{ displayDecimal(row.calculation.excess_value) }}원</dd></div></dl>
        <h4>판매 개수에 따른 소모 배분</h4><p class="field-hint">당시 레시피 사용량이나 판매가가 아닌 판매 개수 비율입니다. 당시 레시피 원가와 별도로 보관합니다.</p>
        <ul v-if="row.calculation.allocation.candidates.length" class="history-list"><li v-for="candidate in row.calculation.allocation.candidates" :key="candidate.sale_id"><NuxtLink :to="`/sales/${candidate.sale_id}`">판매 {{ candidate.sale_id.slice(0, 8) }}</NuxtLink><p>당시 유효 개수 {{ displayDecimal(candidate.count) }}개 · {{ candidate.quantity === null ? '미배분' : displayDecimal(candidate.quantity) + row.unit }} · {{ candidate.value === null ? '원가 미배분' : displayDecimal(candidate.value) + '원' }}</p></li></ul>
        <p v-else class="field-hint">이 기간에 직접 연결할 판매가 없습니다.</p><p class="notice">미배분 소모 {{ displayDecimal(row.calculation.allocation.unallocated_quantity) }}{{ row.unit }} · {{ displayDecimal(row.calculation.allocation.unallocated_value) }}원 <span v-if="row.calculation.allocation.reason === 'zero_weight'">— 대상 판매의 총개수가 0이므로 판매별 배분 수량·원가는 미확정입니다.</span><span v-else-if="row.calculation.allocation.reason === 'no_sales'">— 대상 판매가 없어 별도로 보존합니다.</span></p>
      </template>
      <p v-if="row.calculation.valuation" class="field-hint">가치 출처: <template v-if="row.calculation.valuation.kind === 'manual_amount'">직접 확인한 총금액 {{ displayDecimal(row.calculation.valuation.amount) }}원</template><template v-else>{{ priceSources[row.calculation.valuation.source] }} · {{ displayDecimal(row.calculation.valuation.amount) }}원 / {{ displayDecimal(row.calculation.valuation.quantity) }}{{ row.unit }}</template></p>
    </section>
  </div>
</template>
