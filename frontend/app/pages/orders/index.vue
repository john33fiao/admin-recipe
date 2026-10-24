<script setup lang="ts">
import type { OrderList } from '~/types/purchase-order'
import { apiRequest } from '~/utils/api'
import { displayDecimal } from '~/utils/catalog'
import { formatCreatedAt } from '~/utils/material'
import { orderStatuses } from '~/utils/purchase-order'
const connection = useConnection()
const status = ref('open')
const list = ref<OrderList | null>(null)
const loading = ref(false)
const failure = ref('')
let generation = 0
onScopeDispose(() => { generation += 1 })
async function load() {
  const current = ++generation
  if (connection.value !== 'connected') return
  loading.value = true
  failure.value = ''
  try { const value = await apiRequest<OrderList>(`/api/orders?status=${status.value}`); if (current === generation) list.value = value }
  catch { if (current === generation) failure.value = '발주 내역을 불러오지 못했습니다. 다시 시도해 주세요.' }
  finally { if (current === generation) loading.value = false }
}
watch([connection, status], () => { list.value = null; void load() }, { immediate: true })
useHead({ title: '발주 내역 | 순창한상' })
</script>
<template>
  <div class="page-heading"><div><h1>발주 내역</h1><p>확정한 주문의 진행 상태와 변경 이력을 확인합니다.</p></div><NuxtLink class="primary-link" to="/business?kind=purchase_order&amp;action=place">주문 초안 작성</NuxtLink></div>
  <section class="order-list-panel" aria-labelledby="order-list-title" :aria-busy="loading">
    <div class="list-heading"><h2 id="order-list-title">주문 목록</h2><button class="text-button" type="button" :disabled="loading || connection !== 'connected'" @click="load">다시 불러오기</button></div>
    <div class="field order-filter"><label for="order-status">주문 상태</label><select id="order-status" v-model="status" :disabled="connection !== 'connected'"><option value="open">미입고</option><option value="received">입고 완료</option><option value="cancelled">취소</option><option value="all">전체</option></select></div>
    <p v-if="failure" class="notice warning" role="alert">{{ failure }}</p>
    <div v-else-if="loading && !list" class="empty-state"><h3>발주 내역을 불러오고 있습니다</h3></div>
    <div v-else-if="list && !list.items.length" class="empty-state"><h3>{{ status === 'open' ? '미입고 주문이 없습니다' : '이 상태의 주문이 없습니다' }}</h3><p>주문 초안을 저장하고 확정하면 발주 내역에 표시됩니다.</p><NuxtLink to="/business">작성 중인 업무 확인</NuxtLink></div>
    <ul v-else-if="list" class="business-list order-list"><li v-for="item in list.items" :key="item.id"><NuxtLink :to="`/orders/${item.id}`"><div><strong>{{ item.material_name }}</strong><span class="status-label">{{ orderStatuses[item.status] }}</span></div><p class="order-quantity">{{ displayDecimal(item.input_quantity) }}{{ item.input_unit }} 주문</p><p>{{ item.supplier ?? '거래처 미입력' }} · {{ item.expected_date ?? '예정일 미입력' }}</p><p>예상 금액 {{ item.estimated_amount === null ? '미입력' : `${displayDecimal(item.estimated_amount)}원` }}</p><time>{{ formatCreatedAt(item.updated_at) }} 수정 · {{ item.id.slice(0, 8) }}</time></NuxtLink></li></ul>
  </section>
</template>
