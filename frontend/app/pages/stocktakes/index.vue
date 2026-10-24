<script setup lang="ts">
import type { StocktakeList } from '~/types/stocktake'
import type { Material } from '~/types/material'
import { apiRequest } from '~/utils/api'
import { formatCreatedAt } from '~/utils/material'
const route = useRoute()
const connection = useConnection()
const filter = ref(typeof route.query.material === 'string' ? route.query.material : '')
const materials = ref<Material[]>([])
const list = ref<StocktakeList | null>(null)
const loading = ref(false)
const failure = ref('')
let generation = 0
onScopeDispose(() => { generation += 1 })
async function load() {
  const current = ++generation
  if (connection.value !== 'connected') { loading.value = false; return }
  loading.value = true; failure.value = ''
  try {
    const [value, choices] = await Promise.all([apiRequest<StocktakeList>(`/api/stocktakes${filter.value ? `?material_id=${encodeURIComponent(filter.value)}` : ''}`), apiRequest<Material[]>('/api/materials?archive=all')])
    if (current === generation) { list.value = value; materials.value = choices }
  } catch { if (current === generation) failure.value = '실사 기록을 불러오지 못했습니다. 다시 시도해 주세요.' }
  finally { if (current === generation) loading.value = false }
}
watch(() => route.query.material, value => { filter.value = typeof value === 'string' ? value : '' })
watch([connection, filter], () => { list.value = null; void load() }, { immediate: true })
useHead({ title: '실사 기록 | 순창한상' })
</script>
<template>
  <div class="page-heading"><div><h1>실사 기록</h1><p>직접 확인한 수량과 확정 소모·판매 배분을 확인합니다.</p></div><NuxtLink class="primary-link" to="/business?kind=stocktake">실사 초안 작성</NuxtLink></div>
  <section class="order-list-panel" aria-labelledby="stocktake-list-title" :aria-busy="loading"><div class="list-heading"><h2 id="stocktake-list-title">확정한 실사</h2><button class="text-button" type="button" :disabled="loading || connection !== 'connected'" @click="load">다시 불러오기</button></div>
    <div class="field"><label for="stocktake-material-filter">품목 조회</label><select id="stocktake-material-filter" v-model="filter" :disabled="connection !== 'connected'"><option value="">모든 품목</option><option v-for="item in materials" :key="item.id" :value="item.id">{{ item.name }}{{ item.archived ? ' (보관됨)' : '' }} · {{ item.id.slice(0, 8) }}</option></select><p class="field-hint">선택한 품목의 확정 기록을 조회합니다.</p></div>
    <p v-if="failure" class="notice warning" role="alert">{{ failure }}</p><div v-else-if="loading && !list" class="empty-state"><h3>실사 기록을 불러오고 있습니다</h3></div><div v-else-if="list && !list.items.length" class="empty-state"><h3>확정한 실사가 없습니다</h3><p>실제로 확인한 내용을 초안으로 작성하고 미리보기한 뒤 확정하세요.</p><NuxtLink to="/business">작성 중인 업무 확인</NuxtLink></div>
    <ul v-else-if="list" class="business-list stocktake-list"><li v-for="item in list.items" :key="item.id"><NuxtLink :to="`/stocktakes/${item.id}`"><div><strong>{{ item.first_material_name }}</strong><span class="status-label">실사 확정</span></div><p>확인한 품목 {{ item.confirmed_count }}개</p><time>{{ formatCreatedAt(item.recorded_at) }} 기록 · {{ item.id.slice(0, 8) }}</time></NuxtLink></li></ul>
  </section>
</template>
