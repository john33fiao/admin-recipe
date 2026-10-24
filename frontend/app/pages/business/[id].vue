<script setup lang="ts">
import type { BusinessDetail } from '~/types/business'
import { ApiError, apiRequest } from '~/utils/api'
import { businessInputText, businessLabel, businessStatuses } from '~/utils/business'
import { formatCreatedAt } from '~/utils/material'

const route = useRoute()
const connection = useConnection()
const detail = ref<BusinessDetail | null>(null)
const loading = ref(false)
const failure = ref('')
let generation = 0
async function load(): Promise<BusinessDetail | null> {
  const current = ++generation
  if (connection.value !== 'connected') return null
  loading.value = true
  failure.value = ''
  try {
    const result = await apiRequest<BusinessDetail>(`/api/business/${encodeURIComponent(String(route.params.id))}`)
    if (current !== generation) return null
    detail.value = result
    return result
  } catch (error) { if (current === generation) failure.value = error instanceof ApiError && error.status === 404 ? '이 업무 기록을 찾을 수 없습니다.' : '업무 기록을 불러오지 못했습니다. 다시 시도해 주세요.'; return null }
  finally { if (current === generation) loading.value = false }
}
watch(() => route.params.id, () => { detail.value = null })
watch([connection, () => route.params.id], () => { void load() }, { immediate: true })
const effective = computed(() => detail.value?.confirmations.find(value => value.id === detail.value?.item.effective_confirmation_id))
useHead({ title: computed(() => `${detail.value?.item.material_name ?? '업무 상세'} | 순창한상`) })
</script>

<template>
  <NuxtLink class="back-link" to="/business">‹ 업무 기록</NuxtLink>
  <p v-if="failure" class="notice warning" role="alert">{{ failure }} <button type="button" class="text-button" :disabled="loading || connection !== 'connected'" @click="load">다시 불러오기</button></p>
  <div v-if="!detail && !failure" class="empty-state"><h1>업무 기록을 불러오고 있습니다</h1></div>
  <template v-if="detail">
    <div class="page-heading"><div><h1>{{ detail.item.material_name ?? '재료 확인 필요' }}</h1><p>{{ detail.item.corrects_confirmation_id ? '사전 가격 정정' : businessLabel(detail.head.payload) }} · {{ businessStatuses[detail.item.status] }}</p></div><span class="status-label">{{ businessStatuses[detail.item.status] }}</span></div>
    <section class="business-record-summary" aria-label="저장된 업무 내용">
      <dl><div><dt>저장된 입력</dt><dd>{{ businessInputText(detail.head.payload) }}</dd></div><template v-if="detail.head.payload.kind === 'purchase_order' && detail.head.payload.data.action !== 'cancel'"><div><dt>거래처명</dt><dd>{{ detail.head.payload.data.supplier || '미입력' }}</dd></div><div><dt>입고 예정일</dt><dd>{{ detail.head.payload.data.expected_date || '미입력' }}</dd></div></template><div><dt>{{ detail.item.corrects_confirmation_id ? '정정 이유' : '메모' }}</dt><dd class="preserve-lines">{{ detail.head.payload.data.note || '없음' }}</dd></div><div><dt>최근 저장</dt><dd>{{ formatCreatedAt(detail.head.recorded_at) }}</dd></div><div><dt>업무 ID</dt><dd class="material-id">{{ detail.item.id }}</dd></div></dl>
      <NuxtLink v-if="'material_id' in detail.head.payload.data && detail.head.payload.data.material_id" :to="`/materials/${detail.head.payload.data.material_id}`">재료와 가격 확인</NuxtLink>
      <template v-if="detail.head.payload.kind === 'receipt'"><p>{{ detail.head.payload.data.source.kind === 'direct' ? '직접 입고' : '주문 연결 입고' }} · {{ detail.head.payload.data.stock_effect.kind === 'unreflected' ? '아직 재고에 반영되지 않은 입고' : '이미 포함되었거나 포함 여부 확인 필요' }}</p><NuxtLink v-if="detail.head.payload.data.source.kind === 'direct' && detail.head.payload.data.source.material_id" :to="`/materials/${detail.head.payload.data.source.material_id}`">재료와 보유 근거 확인</NuxtLink><NuxtLink v-else-if="detail.head.payload.data.source.kind === 'order' && detail.head.payload.data.source.order_id" :to="`/orders/${detail.head.payload.data.source.order_id}`">연결한 주문 보기</NuxtLink></template>
      <ProductionDraftSummary v-if="detail.head.payload.kind === 'production'" :payload="detail.head.payload"/>
    </section>
    <BusinessPriceEditor v-if="detail.head.payload.kind === 'catalog_price'" :key="detail.item.id" :identity="detail.item.id" :initial="{ payload: detail.head.payload, state: detail.state, recordId: detail.item.id, headId: detail.head.id, status: detail.head.status, corrects: detail.item.corrects_confirmation_id, issues: detail.issues }" :refresh="load" />
    <BusinessOrderEditor v-else-if="detail.head.payload.kind === 'purchase_order'" :key="detail.item.id" :identity="detail.item.id" :initial="{ payload: detail.head.payload, state: detail.state, recordId: detail.item.id, headId: detail.head.id, status: detail.head.status, corrects: null, issues: detail.issues }" :refresh="load" />
    <BusinessReceiptEditor v-else-if="detail.head.payload.kind === 'receipt'" :key="detail.item.id" :identity="detail.item.id" :initial="{ payload: detail.head.payload, state: detail.state, recordId: detail.item.id, headId: detail.head.id, status: detail.head.status, corrects: null, issues: detail.issues }" :refresh="load" />
    <BusinessProductionEditor v-else :key="detail.item.id" :identity="detail.item.id" :initial="{ payload: detail.head.payload, state: detail.state, recordId: detail.item.id, headId: detail.head.id, status: detail.head.status, corrects: null, issues: detail.issues }" :refresh="load" />
    <section v-if="detail.head.payload.kind === 'catalog_price' && detail.confirmations.length" class="history-section" aria-labelledby="business-chain-title">
      <h2 id="business-chain-title">확정과 정정 연결</h2><p class="field-hint">원본 확정과 정정 기록은 모두 남습니다. 이 연결의 현재 유효 기록과 재료의 최신 가격은 다를 수 있습니다.</p>
      <ol class="history-list business-chain"><li v-for="confirmation in detail.confirmations" :key="confirmation.id"><div><NuxtLink :to="`/business/${confirmation.record_id}`">{{ confirmation.id === detail.confirmations[0]?.id ? '원본 확정' : '정정 확정' }} · {{ confirmation.record_id.slice(0, 8) }}</NuxtLink><span v-if="confirmation.id === detail.item.effective_confirmation_id" class="status-label">현재 유효</span></div><p>{{ formatCreatedAt(confirmation.recorded_at) }} · 기록 {{ confirmation.seq }}</p><p v-if="confirmation.result.kind === 'catalog_price'" class="material-id">가격 결과 {{ confirmation.result.price_id }}</p></li></ol>
      <NuxtLink v-if="detail.item.confirmation_id && detail.item.confirmation_id === detail.item.effective_confirmation_id" class="secondary-link" :to="{ path: '/business', query: { corrects: detail.item.confirmation_id, record: detail.item.id } }">이 가격 정정하기</NuxtLink>
      <NuxtLink v-else-if="effective" class="secondary-link" :to="`/business/${effective.record_id}`">현재 유효한 확정 열기</NuxtLink>
    </section>
    <section v-if="detail.head.payload.kind === 'purchase_order' && detail.confirmations.length" class="history-section" aria-labelledby="order-result-title"><h2 id="order-result-title">확정한 주문</h2><template v-for="confirmation in detail.confirmations" :key="confirmation.id"><NuxtLink v-if="confirmation.result.kind === 'purchase_order'" class="secondary-link" :to="`/orders/${confirmation.result.order_id}`">주문과 이력 보기</NuxtLink></template></section>
    <section v-if="detail.head.payload.kind === 'receipt' && detail.confirmations.length" class="history-section" aria-labelledby="receipt-result-title"><h2 id="receipt-result-title">확정한 입고</h2><template v-for="confirmation in detail.confirmations" :key="confirmation.id"><NuxtLink v-if="confirmation.result.kind === 'receipt'" class="secondary-link" :to="`/receipts/${confirmation.result.receipt_id}`">실제 입고 기록 보기</NuxtLink></template></section>
    <section v-if="detail.head.payload.kind === 'production' && detail.confirmations.length" class="history-section" aria-labelledby="production-result-title"><h2 id="production-result-title">확정한 생산</h2><template v-for="confirmation in detail.confirmations" :key="confirmation.id"><NuxtLink v-if="confirmation.result.kind === 'production'" class="secondary-link" :to="`/productions/${confirmation.result.production_id}`">실제 생산 기록 보기</NuxtLink></template></section>
    <section class="history-section" aria-labelledby="business-revisions-title"><h2 id="business-revisions-title">입력과 상태 이력</h2><ol class="history-list"><li v-for="revision in [...detail.revisions].reverse()" :key="revision.id"><div><strong>{{ businessStatuses[revision.status] }}</strong><time>{{ formatCreatedAt(revision.recorded_at) }}</time></div><p>{{ businessInputText(revision.payload) }}</p><details v-if="revision.payload.kind === 'production'"><summary>당시 투입과 결과 입력</summary><ProductionDraftSummary :payload="revision.payload"/></details><p v-if="revision.payload.data.note" class="preserve-lines">{{ revision.payload.data.note }}</p><p class="material-id">기록 {{ revision.seq }} · {{ revision.id }}</p></li></ol></section>
  </template>
</template>
