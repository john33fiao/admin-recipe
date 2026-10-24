<script setup lang="ts">
import type { ReceiptDetail } from '~/types/receipt'
import { ApiError, apiRequest } from '~/utils/api'
import { displayDecimal } from '~/utils/catalog'
import { formatCreatedAt } from '~/utils/material'
const route = useRoute()
const connection = useConnection()
const detail = ref<ReceiptDetail | null>(null)
const failure = ref('')
const loading = ref(false)
let generation = 0
onScopeDispose(() => { generation += 1 })
async function load() {
  const current = ++generation
  if (connection.value !== 'connected') return
  loading.value = true; failure.value = ''
  try { const value = await apiRequest<ReceiptDetail>(`/api/receipts/${encodeURIComponent(String(route.params.id))}`); if (current === generation) detail.value = value }
  catch (error) { if (current === generation) failure.value = error instanceof ApiError && error.status === 404 ? '이 입고 기록을 찾을 수 없습니다.' : '입고 기록을 불러오지 못했습니다. 다시 시도해 주세요.' }
  finally { if (current === generation) loading.value = false }
}
watch(() => route.params.id, () => { detail.value = null })
watch([connection, () => route.params.id], () => { void load() }, { immediate: true })
useHead({ title: computed(() => `${detail.value?.item.material_name ?? '입고 상세'} | 순창한상`) })
</script>
<template>
  <NuxtLink class="back-link" to="/receipts">‹ 입고 기록</NuxtLink>
  <p v-if="failure" class="notice warning" role="alert">{{ failure }} <button class="text-button" type="button" :disabled="loading || connection !== 'connected'" @click="load">다시 불러오기</button></p>
  <div v-if="!detail && !failure" class="empty-state"><h1>입고 기록을 불러오고 있습니다</h1></div>
  <template v-if="detail"><div class="page-heading"><div><h1>{{ detail.item.material_name }}</h1><p>{{ detail.item.source.kind === 'direct' ? '직접 입고' : '주문 연결 입고' }} · 확정</p></div></div>
    <section class="business-record-summary receipt-detail" aria-labelledby="receipt-detail-title"><h2 id="receipt-detail-title">확정한 실제 입고</h2><dl><div><dt>실제 입고량</dt><dd class="preview-result">{{ displayDecimal(detail.item.input_quantity) }}{{ detail.item.input_unit }}</dd></div><div><dt>기준 단위 수량</dt><dd>{{ displayDecimal(detail.item.quantity) }}{{ detail.item.unit }}</dd></div><div><dt>실제 구입금액</dt><dd>{{ displayDecimal(detail.item.amount) }}원</dd></div><div><dt>실제 입고일</dt><dd>{{ detail.item.receipt_date }}</dd></div><div><dt>기록 시각</dt><dd>{{ formatCreatedAt(detail.item.recorded_at) }}</dd></div><div><dt>메모</dt><dd class="preserve-lines">{{ detail.item.note || '없음' }}</dd></div><div><dt>입고 ID</dt><dd class="material-id">{{ detail.item.id }}</dd></div><div><dt>당시 품목 버전</dt><dd class="material-id">{{ detail.item.material_version_id }}</dd></div><div><dt>정산 근거</dt><dd class="material-id">{{ detail.holding_version_id }}</dd></div></dl></section>
    <section v-if="detail.order_closure" class="business-preview receipt-order-closure" aria-labelledby="receipt-closure-title"><h2 id="receipt-closure-title">이 입고로 완료한 주문</h2><dl class="business-price-comparison"><div><dt>최초 주문량</dt><dd>{{ displayDecimal(detail.order_closure.initial_ordered_quantity) }}{{ detail.order_closure.unit }}</dd></div><div><dt>입고 전 확정 주문량</dt><dd>{{ displayDecimal(detail.order_closure.ordered_quantity) }}{{ detail.order_closure.unit }}</dd></div><div><dt>실제 입고량</dt><dd>{{ displayDecimal(detail.order_closure.received_quantity) }}{{ detail.order_closure.unit }}</dd></div><div><dt>주문 대비 차이</dt><dd>{{ displayDecimal(detail.order_closure.difference) }}{{ detail.order_closure.unit }}</dd></div></dl><NuxtLink :to="`/orders/${detail.order_closure.order_id}`">완료한 주문과 이력 보기</NuxtLink></section>
    <div class="button-row"><NuxtLink class="secondary-link" :to="`/materials/${detail.item.material_id}`">재료와 보유 근거 확인</NuxtLink><NuxtLink class="secondary-link" :to="`/business/${detail.item.business_record_id}`">입고 업무 기록 보기</NuxtLink></div>
  </template>
</template>
