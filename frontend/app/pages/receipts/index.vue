<script setup lang="ts">
import type { ReceiptList } from '~/types/receipt'
import type { Material } from '~/types/material'
import { apiRequest } from '~/utils/api'
import { displayDecimal } from '~/utils/catalog'
import { formatCreatedAt } from '~/utils/material'
const route = useRoute()
const connection = useConnection()
const filter = ref(typeof route.query.material === 'string' ? route.query.material : '')
const materials = ref<Material[]>([])
const list = ref<ReceiptList | null>(null)
const loading = ref(false)
const failure = ref('')
let generation = 0
onScopeDispose(() => { generation += 1 })
async function load() {
  const current = ++generation
  if (connection.value !== 'connected') return
  loading.value = true; failure.value = ''
  try {
    const [value, choices] = await Promise.all([apiRequest<ReceiptList>(`/api/receipts${filter.value ? `?material_id=${encodeURIComponent(filter.value)}` : ''}`), apiRequest<Material[]>('/api/materials?archive=all')])
    if (current === generation) { list.value = value; materials.value = choices }
  } catch { if (current === generation) failure.value = '입고 기록을 불러오지 못했습니다. 다시 시도해 주세요.' }
  finally { if (current === generation) loading.value = false }
}
watch(() => route.query.material, value => { filter.value = typeof value === 'string' ? value : '' })
watch([connection, filter], () => { list.value = null; void load() }, { immediate: true })
useHead({ title: '입고 기록 | 순창한상' })
</script>
<template>
  <div class="page-heading"><div><h1>입고 기록</h1><p>확정한 실제 수량·구입금액·입고일을 확인합니다.</p></div><NuxtLink class="primary-link" to="/business?kind=receipt">직접 입고 초안</NuxtLink></div>
  <section class="order-list-panel" aria-labelledby="receipt-list-title" :aria-busy="loading"><div class="list-heading"><h2 id="receipt-list-title">확정한 입고</h2><button class="text-button" type="button" :disabled="loading || connection !== 'connected'" @click="load">다시 불러오기</button></div>
    <div class="field"><label for="receipt-material-filter">입고 재료 조회</label><select id="receipt-material-filter" v-model="filter" :disabled="connection !== 'connected'"><option value="">모든 재료</option><option v-for="item in materials" :key="item.id" :value="item.id">{{ item.name }}{{ item.archived ? ' (보관됨)' : '' }} · {{ item.id.slice(0, 8) }}</option></select></div>
    <p v-if="failure" class="notice warning" role="alert">{{ failure }}</p><div v-else-if="loading && !list" class="empty-state"><h3>입고 기록을 불러오고 있습니다</h3></div><div v-else-if="list && !list.items.length" class="empty-state"><h3>확정한 입고가 없습니다</h3><p>실제 받은 수량과 구입금액을 초안으로 작성하고 확인한 뒤 확정하세요.</p><NuxtLink to="/business">작성 중인 업무 확인</NuxtLink></div>
    <ul v-else-if="list" class="business-list receipt-list"><li v-for="item in list.items" :key="item.id"><NuxtLink :to="`/receipts/${item.id}`"><div><strong>{{ item.material_name }}</strong><span class="status-label">{{ item.source.kind === 'direct' ? '직접 입고' : '주문 연결' }}</span></div><p class="order-quantity">{{ displayDecimal(item.input_quantity) }}{{ item.input_unit }} · {{ displayDecimal(item.amount) }}원</p><p>{{ item.receipt_date }} 입고</p><time>{{ formatCreatedAt(item.recorded_at) }} 기록 · {{ item.id.slice(0, 8) }}</time></NuxtLink></li></ul>
  </section>
</template>
