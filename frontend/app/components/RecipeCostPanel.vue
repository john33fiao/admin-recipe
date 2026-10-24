<script setup lang="ts">
import type { RecipeCost, RecipeVersion } from '~/types/recipe'
import { apiRequest } from '~/utils/api'
import { displayDecimal, priceSources, priceText } from '~/utils/catalog'
import { ratioPercent, recipeFraction } from '~/utils/recipe'
const props = defineProps<{ version: RecipeVersion; past?: boolean }>()
const connection = useConnection()
const value = ref<RecipeCost | null>(null)
const failure = ref('')
const loading = ref(false)
let generation = 0
onScopeDispose(() => { generation += 1 })
async function load() {
  const token = ++generation, id = props.version.id, owner = props.version.material_id
  value.value = null; failure.value = ''; loading.value = false
  if (connection.value !== 'connected') return
  loading.value = true
  try {
    const result = await apiRequest<RecipeCost>(`/api/recipes/${encodeURIComponent(owner)}/versions/${encodeURIComponent(id)}/cost`)
    if (token !== generation || props.version.id !== id || result.recipe_version_id !== id) return
    value.value = result
  } catch { if (token === generation) failure.value = '원가를 계산하지 못했습니다. 가격과 계산 범위를 확인한 뒤 다시 시도해 주세요. 배합은 계속 조회하고 수정할 수 있습니다.' }
  finally { if (token === generation) loading.value = false }
}
watch([connection, () => props.version.id], () => { void load() }, { immediate: true })
const reasons = { missing_selling_price: '판매가가 없어 원가율을 계산할 수 없습니다.', zero_selling_price: '판매가가 0원이므로 원가율을 계산할 수 없습니다.', missing_cost: '미계산 재료의 가격을 확인해 주세요.' }
</script>
<template>
  <section class="history-section recipe-cost" aria-labelledby="recipe-cost-title" :aria-busy="loading">
    <div class="section-heading"><h2 id="recipe-cost-title">{{ version.reference_output ? '기준 배치 식재료비' : '1개 기준 식재료비' }}</h2><p>{{ past ? '이 과거 배합을 현재 가격으로 계산합니다. 당시 판매 원가와 다릅니다.' : '저장된 배합을 현재 가격으로 계산합니다. 작성 중인 입력은 포함하지 않습니다.' }}</p></div>
    <p v-if="failure" class="notice warning" role="alert">{{ failure }}</p>
    <p v-else-if="!value" class="stale-note">{{ loading ? '적용 가격을 확인하고 있습니다.' : '연결되면 원가를 확인할 수 있습니다.' }}</p>
    <template v-if="value">
      <dl class="detail-fields"><div><dt>전체 식재료비</dt><dd>{{ value.total_cost === null ? '미계산' : `${displayDecimal(value.total_cost)}원` }}</dd></div><div v-if="value.total_cost === null"><dt>알려진 원가 소계</dt><dd>{{ displayDecimal(value.known_subtotal) }}원 · 미계산 {{ value.rows.filter(row => row.cost === null).length }}행</dd></div><div><dt>판매가</dt><dd>{{ value.selling_price === null ? '미입력' : `${displayDecimal(value.selling_price)}원` }}</dd></div><div><dt>단순 원가율</dt><dd>{{ value.cost_ratio ? ratioPercent(value.cost_ratio) : reasons[value.ratio_unavailable_reason ?? 'missing_cost'] }}</dd></div></dl>
      <ul class="recipe-cost-rows"><li v-for="row in value.rows" :key="row.line_id"><div><NuxtLink :to="`/materials/${row.material_id}`">{{ version.lines.find(line => line.id === row.line_id)?.material_name ?? '구성 재료' }}</NuxtLink><p>{{ recipeFraction(row.numerator_quantity, row.basis_count, row.unit) }}</p></div><div><p>{{ priceSources[row.price.source] }} · {{ priceText(row.price, row.unit) }}</p><p v-if="row.price.reason" class="field-hint">{{ row.price.reason }}</p><details v-if="row.price.source_id"><summary>가격 근거</summary><p class="material-id">{{ row.price.source_id }}</p></details></div><strong>{{ row.cost === null ? '미계산' : `${displayDecimal(row.cost)}원` }}</strong></li></ul>
      <p class="field-hint">직접 참조한 재료의 가격만 합산합니다. 비축품의 하위 재료 원가는 다시 더하지 않습니다.</p>
    </template>
    <button class="text-button" type="button" :disabled="loading || connection !== 'connected'" @click="load">현재 가격으로 원가 다시 확인</button>
  </section>
</template>
