<script setup lang="ts">
import type { HoldingDetail } from '~/types/receipt'
import { apiRequest } from '~/utils/api'
import { displayDecimal, priceSources, priceText } from '~/utils/catalog'
import { formatCreatedAt } from '~/utils/material'
const props = defineProps<{ materialId: string; revision: string }>()
const connection = useConnection()
const holding = ref<HoldingDetail | null>(null)
const failure = ref('')
const loading = ref(false)
let generation = 0
onScopeDispose(() => { generation += 1 })
async function load() {
  const current = ++generation
  if (connection.value !== 'connected') return
  loading.value = true; failure.value = ''
  try { const value = await apiRequest<HoldingDetail>(`/api/holdings/${encodeURIComponent(props.materialId)}`); if (current === generation) holding.value = value }
  catch { if (current === generation) failure.value = '보유 근거를 불러오지 못했습니다. 다시 확인해 주세요.' }
  finally { if (current === generation) loading.value = false }
}
watch(() => props.materialId, () => { holding.value = null })
watch([connection, () => props.materialId, () => props.revision], () => { void load() }, { immediate: true })
</script>
<template>
  <section class="holding-panel history-section" aria-labelledby="holding-title" :aria-busy="loading">
    <div class="list-heading"><h2 id="holding-title">보유 근거</h2><button class="text-button" type="button" :disabled="loading || connection !== 'connected'" @click="load">보유 근거 새로고침</button></div>
    <p v-if="failure" class="notice warning" role="alert">{{ failure }}</p>
    <p v-if="!holding && !failure" class="stale-note">보유 근거를 불러오고 있습니다.</p>
    <template v-if="holding">
      <p v-if="connection !== 'connected'" class="stale-note">마지막으로 확인한 보유 근거입니다.</p>
      <dl class="holding-fields"><div><dt>마지막 확인 수량</dt><dd v-if="holding.observation">{{ displayDecimal(holding.observation.quantity) }}{{ holding.unit }}<p>{{ formatCreatedAt(holding.observation.recorded_at) }} 확인</p></dd><dd v-else>아직 확인한 수량 없음</dd></div><div><dt>정산 기준 수량</dt><dd>{{ holding.settlement_basis ? `${displayDecimal(holding.settlement_basis.quantity)}${holding.unit}` : '아직 정산 기준 없음' }}</dd></div><div><dt>정산 기준 가치</dt><dd>{{ holding.settlement_basis ? holding.settlement_basis.value === null ? '가치 미확인' : `${displayDecimal(holding.settlement_basis.value)}원` : '아직 정산 기준 없음' }}</dd></div><div><dt>원가 출처</dt><dd>{{ priceSources[holding.price.source] }} · {{ priceText(holding.price, holding.unit) }}<p v-if="holding.price.reason" class="field-hint">{{ holding.price.reason }}</p></dd></div><div v-if="holding.last_valid_price"><dt>마지막 유효 단가</dt><dd>{{ displayDecimal(holding.last_valid_price.amount) }}원 / {{ displayDecimal(holding.last_valid_price.quantity) }}{{ holding.unit }}</dd></div><div v-if="holding.basis"><dt>시작 근거</dt><dd><NuxtLink v-if="holding.basis.kind === 'receipt' && holding.basis.id" :to="`/receipts/${holding.basis.id}`">첫 입고 기록 보기</NuxtLink><span v-else>확인한 수량</span><p class="material-id">기록 {{ holding.basis.seq }}</p></dd></div></dl>
      <p class="field-hint">정산 기준은 확인한 근거와 이후 확정 변동을 반영합니다. 새로 측정한 현재고와 구분합니다.</p>
      <div v-if="holding.confirmed_changes.length" class="holding-changes"><h3>시작 근거 이후 확정 입고</h3><ul class="history-list"><li v-for="change in holding.confirmed_changes" :key="change.event_id"><NuxtLink :to="`/receipts/${change.event_id}`">+{{ displayDecimal(change.quantity) }}{{ holding.unit }} · {{ displayDecimal(change.value) }}원</NuxtLink><p>{{ formatCreatedAt(change.recorded_at) }} 기록 · 순서 {{ change.seq }}</p></li></ul></div>
      <p class="button-row"><NuxtLink class="secondary-link" :to="{ path: '/business', query: { kind: 'receipt', material: materialId } }">직접 입고 초안</NuxtLink><NuxtLink class="secondary-link" :to="{ path: '/receipts', query: { material: materialId } }">이 재료의 입고 기록</NuxtLink></p>
      <details v-if="holding.settlement_basis?.source_id || holding.price.source_id" class="holding-references"><summary>근거 참조 확인</summary><p v-if="holding.settlement_basis?.source_id" class="material-id">정산 근거 {{ holding.settlement_basis.source_id }}</p><p v-if="holding.price.source_id" class="material-id">원가 근거 {{ holding.price.source_id }}</p></details>
    </template>
  </section>
</template>
