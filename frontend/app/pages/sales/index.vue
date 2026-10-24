<script setup lang="ts">
import type { SaleList } from '~/types/sale'
import type { RecipeList } from '~/types/recipe'
import { apiRequest } from '~/utils/api'
import { displayDecimal } from '~/utils/catalog'
import { formatCreatedAt } from '~/utils/material'
const route = useRoute()
const connection = useConnection()
const filter = ref(typeof route.query.material === 'string' ? route.query.material : '')
const recipes = ref<RecipeList['items']>([])
const list = ref<SaleList | null>(null)
const loading = ref(false)
const failure = ref('')
let generation = 0
onScopeDispose(() => { generation += 1 })
async function load() {
  const current = ++generation
  if (connection.value !== 'connected') { loading.value = false; return }
  loading.value = true; failure.value = ''
  try {
    const [value, choices] = await Promise.all([apiRequest<SaleList>(`/api/sales${filter.value ? `?material_id=${encodeURIComponent(filter.value)}` : ''}`), apiRequest<RecipeList>('/api/recipes?archive=all')])
    if (current === generation) { list.value = value; recipes.value = choices.items }
  } catch { if (current === generation) failure.value = '판매 기록을 불러오지 못했습니다. 다시 시도해 주세요.' }
  finally { if (current === generation) loading.value = false }
}
watch(() => route.query.material, value => { filter.value = typeof value === 'string' ? value : '' })
watch([connection, filter], () => { list.value = null; void load() }, { immediate: true })
useHead({ title: '판매 기록 | 순창한상' })
</script>
<template>
  <div class="page-heading"><div><h1>판매 기록</h1><p>판매 개수와 당시 가격·레시피를 확인합니다. 실사 소모는 아직 확정되지 않았습니다.</p></div><NuxtLink class="primary-link" to="/business?kind=sale">판매 초안 작성</NuxtLink></div>
  <section class="order-list-panel" aria-labelledby="sale-list-title" :aria-busy="loading"><div class="list-heading"><h2 id="sale-list-title">확정한 판매</h2><button class="text-button" type="button" :disabled="loading || connection !== 'connected'" @click="load">다시 불러오기</button></div>
    <div class="field"><label for="sale-material-filter">판매한 메뉴 조회</label><select id="sale-material-filter" v-model="filter" :disabled="connection !== 'connected'"><option value="">모든 메뉴</option><option v-for="entry in recipes" :key="entry.item.id" :value="entry.item.id">{{ entry.item.name }}{{ entry.item.archived ? ' (보관됨)' : '' }} · {{ entry.item.id.slice(0, 8) }}</option></select><p class="field-hint">판매한 메뉴 기준입니다. 구성 재료를 기준으로 조회하지 않습니다.</p></div>
    <p v-if="failure" class="notice warning" role="alert">{{ failure }}</p><div v-else-if="loading && !list" class="empty-state"><h3>판매 기록을 불러오고 있습니다</h3></div>
    <dl v-if="list" class="business-price-comparison sale-totals"><div><dt>알려진 판매금액 합계</dt><dd>{{ displayDecimal(list.known_revenue) }}원</dd></div><div><dt>판매금액 미계산</dt><dd>{{ list.missing_revenue_count }}건</dd></div></dl><p v-if="list" class="field-hint">조회한 판매의 현재 유효한 개수 기준입니다. 가격이 없는 판매는 합계에 포함하지 않습니다.</p>
    <div v-if="list && !list.items.length" class="empty-state"><h3>확정한 판매가 없습니다</h3><p>판매한 메뉴와 개수를 초안으로 저장하고 확인한 뒤 확정하세요.</p><NuxtLink to="/business?kind=sale">판매 초안 작성</NuxtLink></div>
    <ul v-else-if="list" class="business-list sale-list"><li v-for="item in list.items" :key="item.id"><NuxtLink :to="`/sales/${item.id}`"><div><strong>{{ item.material_name }}</strong><span class="status-label">실사 소모 미확정</span></div><p class="order-quantity">{{ displayDecimal(item.count) }}개 · {{ item.amount === null ? '판매금액 미계산' : `${displayDecimal(item.amount)}원` }}</p><p>당시 판매가 {{ item.selling_price === null ? '미입력' : `${displayDecimal(item.selling_price)}원` }}</p><time>{{ formatCreatedAt(item.recorded_at) }} 최초 기록 · {{ item.id.slice(0, 8) }}</time><p v-if="item.updated_at !== item.recorded_at">{{ formatCreatedAt(item.updated_at) }} 개수 정정</p></NuxtLink></li></ul>
  </section>
</template>
