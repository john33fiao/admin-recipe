<script setup lang="ts">
import type { SaleRecipeCost } from '~/types/sale'
import { displayDecimal, priceSources, priceText } from '~/utils/catalog'
import { ratioPercent, recipeFraction } from '~/utils/recipe'
defineProps<{ cost: SaleRecipeCost }>()
const heading = useId()
const reasons = { missing_selling_price: '판매가가 없어 원가율을 계산할 수 없습니다.', zero_selling_price: '판매가가 0원이므로 원가율을 계산할 수 없습니다.', missing_cost: '당시 가격이 없는 재료가 있어 원가율을 계산할 수 없습니다.' }
</script>
<template>
  <section class="sale-cost-snapshot" :aria-labelledby="heading"><h3 :id="heading">레시피 기준 원가 (판매 당시)</h3><p class="field-hint">판매 당시의 메뉴 1개 기준 식재료비입니다. 실사로 배분한 소모 원가와 구분합니다.</p>
    <dl class="business-price-comparison"><div><dt>1개 기준 식재료비</dt><dd>{{ cost.total_cost === null ? '미계산' : `${displayDecimal(cost.total_cost)}원` }}</dd></div><div v-if="cost.total_cost === null"><dt>알려진 원가 소계</dt><dd>{{ displayDecimal(cost.known_subtotal) }}원 · 미계산 {{ cost.rows.filter(row => row.cost === null).length }}행</dd></div><div><dt>당시 판매가</dt><dd>{{ cost.selling_price === null ? '미입력' : `${displayDecimal(cost.selling_price)}원` }}</dd></div><div><dt>단순 원가율</dt><dd>{{ cost.cost_ratio ? ratioPercent(cost.cost_ratio) : reasons[cost.ratio_unavailable_reason ?? 'missing_cost'] }}</dd></div></dl>
    <p v-if="!cost.rows.length" class="field-hint">직접 구성 재료가 없는 레시피입니다. 실제 소모가 0이라는 뜻은 아닙니다.</p>
    <ul v-else class="recipe-cost-rows"><li v-for="row in cost.rows" :key="row.line_id"><div><NuxtLink :to="`/materials/${row.material_id}`">{{ row.material_name }}</NuxtLink><p>{{ recipeFraction(row.numerator_quantity, row.basis_count, row.unit) }}</p></div><div><p>{{ priceSources[row.price.source] }} · {{ priceText(row.price, row.unit) }}</p><p v-if="row.price.reason" class="field-hint">{{ row.price.reason }}</p><details v-if="row.price.source_id"><summary>당시 가격 근거</summary><p class="material-id">{{ row.price.source_id }}</p></details></div><strong>{{ row.cost === null ? '미계산' : `${displayDecimal(row.cost)}원` }}</strong></li></ul>
  </section>
</template>
