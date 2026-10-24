<script setup lang="ts">
import type { OrderDetail } from '~/types/purchase-order'
import { ApiError, apiRequest } from '~/utils/api'
import { displayDecimal } from '~/utils/catalog'
import { formatCreatedAt } from '~/utils/material'
import { orderActions, orderStatuses } from '~/utils/purchase-order'
const route = useRoute()
const connection = useConnection()
const detail = ref<OrderDetail | null>(null)
const loading = ref(false)
const failure = ref('')
let generation = 0
onScopeDispose(() => { generation += 1 })
async function load() {
  const current = ++generation
  if (connection.value !== 'connected') return
  loading.value = true
  failure.value = ''
  try { const value = await apiRequest<OrderDetail>(`/api/orders/${encodeURIComponent(String(route.params.id))}`); if (current === generation) detail.value = value }
  catch (error) { if (current === generation) failure.value = error instanceof ApiError && error.status === 404 ? '이 주문을 찾을 수 없습니다.' : '주문을 불러오지 못했습니다. 다시 시도해 주세요.' }
  finally { if (current === generation) loading.value = false }
}
watch(() => route.params.id, () => { detail.value = null })
watch([connection, () => route.params.id], () => { void load() }, { immediate: true })
useHead({ title: computed(() => `${detail.value?.item.material_name ?? '주문 상세'} | 순창한상`) })
</script>
<template>
  <NuxtLink class="back-link" to="/orders">‹ 발주 내역</NuxtLink>
  <p v-if="failure" class="notice warning" role="alert">{{ failure }} <button class="text-button" type="button" :disabled="loading || connection !== 'connected'" @click="load">다시 불러오기</button></p>
  <div v-if="!detail && !failure" class="empty-state"><h1>주문을 불러오고 있습니다</h1></div>
  <template v-if="detail">
    <div class="page-heading"><div><h1>{{ detail.item.material_name }}</h1><p>발주 내역 · {{ orderStatuses[detail.item.status] }}</p></div><span class="status-label">{{ orderStatuses[detail.item.status] }}</span></div>
    <section class="business-record-summary order-current" aria-labelledby="order-current-title"><h2 id="order-current-title">현재 주문 내용</h2><dl><div><dt>주문량</dt><dd class="order-quantity">{{ displayDecimal(detail.head.input_quantity) }}{{ detail.head.input_unit }}</dd></div><div><dt>기준 단위 주문량</dt><dd>{{ displayDecimal(detail.head.quantity) }}{{ detail.head.unit }}</dd></div><div><dt>거래처명</dt><dd>{{ detail.head.supplier ?? '미입력' }}</dd></div><div><dt>입고 예정일</dt><dd>{{ detail.head.expected_date ?? '미입력' }}</dd></div><div><dt>예상 금액</dt><dd>{{ detail.head.estimated_amount === null ? '미입력' : `${displayDecimal(detail.head.estimated_amount)}원` }}</dd></div><div><dt>메모</dt><dd class="preserve-lines">{{ detail.head.note || '없음' }}</dd></div><div><dt>최근 변경</dt><dd>{{ formatCreatedAt(detail.head.recorded_at) }}</dd></div><div><dt>주문 ID</dt><dd class="material-id">{{ detail.item.id }}</dd></div></dl><NuxtLink :to="`/materials/${detail.item.material_id}`">재료와 가격 확인</NuxtLink></section>
    <div v-if="detail.item.status === 'open'" class="button-row order-actions"><NuxtLink class="secondary-link" :to="{ path: '/business', query: { kind: 'purchase_order', action: 'revise', order: detail.item.id } }">주문 수정 초안</NuxtLink><NuxtLink class="secondary-link" :to="{ path: '/business', query: { kind: 'purchase_order', action: 'cancel', order: detail.item.id } }">주문 취소 초안</NuxtLink></div>
    <p class="field-hint">주문 기록은 현재 보유 수량과 사전 가격을 변경하지 않습니다.</p>
    <section class="history-section" aria-labelledby="order-history-title"><h2 id="order-history-title">주문 변경 이력</h2><ol class="history-list order-history"><li v-for="version in [...detail.versions].reverse()" :key="version.id"><div><strong>{{ orderActions[version.action] }}</strong><span class="status-label">{{ orderStatuses[version.status] }}</span></div><p>{{ version.material_name }} · {{ displayDecimal(version.input_quantity) }}{{ version.input_unit }}</p><p>{{ version.supplier ?? '거래처 미입력' }} · {{ version.expected_date ?? '예정일 미입력' }} · 예상 금액 {{ version.estimated_amount === null ? '미입력' : `${displayDecimal(version.estimated_amount)}원` }}</p><p v-if="version.note" class="preserve-lines">{{ version.note }}</p><p>{{ formatCreatedAt(version.recorded_at) }} · 기록 {{ version.seq }}</p><NuxtLink :to="`/business/${version.business_record_id}`">이 변경의 업무 기록</NuxtLink><p class="material-id">{{ version.id }}</p></li></ol></section>
  </template>
</template>
