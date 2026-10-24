<script setup lang="ts">
import type { ProductionList } from '~/types/production'
import type { Material } from '~/types/material'
import { apiRequest } from '~/utils/api'
import { displayDecimal } from '~/utils/catalog'
import { formatCreatedAt } from '~/utils/material'
const route = useRoute()
const connection = useConnection()
const filter = ref(typeof route.query.material === 'string' ? route.query.material : '')
const materials = ref<Material[]>([])
const list = ref<ProductionList | null>(null)
const loading = ref(false)
const failure = ref('')
let generation = 0
onScopeDispose(() => { generation += 1 })
async function load() {
  const current = ++generation
  if (connection.value !== 'connected') { loading.value = false; return }
  loading.value = true; failure.value = ''
  try {
    const [value, choices] = await Promise.all([apiRequest<ProductionList>(`/api/productions${filter.value ? `?material_id=${encodeURIComponent(filter.value)}` : ''}`), apiRequest<Material[]>('/api/materials?archive=all')])
    if (current === generation) { list.value = value; materials.value = choices }
  } catch { if (current === generation) failure.value = '생산 기록을 불러오지 못했습니다. 다시 시도해 주세요.' }
  finally { if (current === generation) loading.value = false }
}
watch(() => route.query.material, value => { filter.value = typeof value === 'string' ? value : '' })
watch([connection, filter], () => { list.value = null; void load() }, { immediate: true })
useHead({ title: '생산 기록 | 순창한상' })
</script>
<template>
  <div class="page-heading"><div><h1>생산 기록</h1><p>확정한 실제 투입·결과와 원가 배분을 확인합니다.</p></div><NuxtLink class="primary-link" to="/business?kind=production">생산 초안 작성</NuxtLink></div>
  <section class="order-list-panel" aria-labelledby="production-list-title" :aria-busy="loading"><div class="list-heading"><h2 id="production-list-title">확정한 생산</h2><button class="text-button" type="button" :disabled="loading || connection !== 'connected'" @click="load">다시 불러오기</button></div>
    <div class="field"><label for="production-material-filter">투입·결과 품목 조회</label><select id="production-material-filter" v-model="filter" :disabled="connection !== 'connected'"><option value="">모든 품목</option><option v-for="item in materials" :key="item.id" :value="item.id">{{ item.name }}{{ item.archived ? ' (보관됨)' : '' }} · {{ item.id.slice(0, 8) }}</option></select><p class="field-hint">투입 또는 결과에 포함된 생산을 한 번씩 표시합니다.</p></div>
    <p v-if="failure" class="notice warning" role="alert">{{ failure }}</p><div v-else-if="loading && !list" class="empty-state"><h3>생산 기록을 불러오고 있습니다</h3></div><div v-else-if="list && !list.items.length" class="empty-state"><h3>확정한 생산이 없습니다</h3><p>실제로 사용한 양과 만들어진 양을 초안으로 작성하고 확인한 뒤 확정하세요.</p><NuxtLink to="/business">작성 중인 업무 확인</NuxtLink></div>
    <ul v-else-if="list" class="business-list production-list"><li v-for="item in list.items" :key="item.id"><NuxtLink :to="`/productions/${item.id}`"><div><strong>{{ item.first_output_name }}</strong><span class="status-label">생산 확정</span></div><p>투입 {{ item.input_count }}행 · 결과 {{ item.output_count }}행</p><p class="order-quantity">총 투입 원가 {{ displayDecimal(item.total_input_value) }}원</p><time>{{ formatCreatedAt(item.recorded_at) }} 기록 · {{ item.id.slice(0, 8) }}</time></NuxtLink></li></ul>
  </section>
</template>
