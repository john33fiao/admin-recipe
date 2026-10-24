<script setup lang="ts">
import type { CorrectionSeed } from '~/types/correction'
import { saleConsumptionLabels } from '~/utils/sale'
import type { SaleDetail } from '~/types/sale'
import { ApiError, apiRequest } from '~/utils/api'
import { displayDecimal } from '~/utils/catalog'
import { formatCreatedAt } from '~/utils/material'
const route = useRoute()
const connection = useConnection()
const detail = ref<SaleDetail | null>(null)
const effectiveRecord = ref<CorrectionSeed | null>(null)
const voided = computed(() => detail.value?.effective?.kind === 'void' || effectiveRecord.value?.current.value === null)
const useCommonCorrection = computed(() => !!detail.value?.effective || detail.value?.consumption_status === 'settled' || effectiveRecord.value?.history.some(entry => entry.confirmation.result.kind === 'correction'))
const failure = ref('')
const loading = ref(false)
let generation = 0
onScopeDispose(() => { generation += 1 })
async function load() {
  const current = ++generation
  if (connection.value !== 'connected') { loading.value = false; return }
  loading.value = true; failure.value = ''
  try { const value = await apiRequest<SaleDetail>(`/api/sales/${encodeURIComponent(String(route.params.id))}`); if (current === generation) detail.value = value }
  catch (error) { if (current === generation) failure.value = error instanceof ApiError && error.status === 404 ? '이 판매 기록을 찾을 수 없습니다.' : '판매 기록을 불러오지 못했습니다. 다시 시도해 주세요.' }
  finally { if (current === generation) loading.value = false }
}
watch(() => route.params.id, () => { detail.value = null; effectiveRecord.value = null })
watch([connection, () => route.params.id], () => { void load() }, { immediate: true })
useHead({ title: computed(() => `${detail.value?.item.material_name ?? '판매 상세'} | 순창한상`) })
</script>
<template>
  <NuxtLink class="back-link" to="/sales">‹ 판매 기록</NuxtLink><p v-if="failure" class="notice warning" role="alert">{{ failure }} <button class="text-button" type="button" :disabled="loading || connection !== 'connected'" @click="load">다시 불러오기</button></p><div v-if="!detail && !failure" class="empty-state"><h1>판매 기록을 불러오고 있습니다</h1></div>
  <template v-if="detail"><div class="page-heading"><div><h1>{{ detail.item.material_name }}</h1><p>판매 당시의 메뉴와 레시피 · {{ voided ? '유효 효과 취소됨' : '판매 개수 이력' }}</p></div><span class="status-label">{{ saleConsumptionLabels[detail.consumption_status] }}</span></div>
    <section class="business-record-summary sale-detail" aria-labelledby="sale-detail-title"><h2 id="sale-detail-title">{{ voided ? '보존된 판매 기록' : '확정한 판매' }}</h2><p v-if="voided" class="notice warning">현재 유효한 판매는 취소되었습니다. 아래 개수와 금액은 보존된 기록이며 현재 판매로 합산하지 않습니다.</p><dl><div><dt>{{ voided ? '보존된 판매 개수' : '현재 유효한 판매 개수' }}</dt><dd class="preview-result">{{ displayDecimal(detail.effective?.kind === 'active' ? detail.effective.count : detail.head.count) }}개</dd></div><div><dt>판매금액</dt><dd>{{ (detail.effective?.kind === 'active' ? detail.effective.amount : detail.head.amount) === null ? '미계산' : `${displayDecimal((detail.effective?.kind === 'active' ? detail.effective.amount : detail.head.amount)!)}원` }}</dd></div><div><dt>메모·정정 이유</dt><dd class="preserve-lines">{{ detail.head.note || '없음' }}</dd></div><div><dt>최초 기록 시각</dt><dd>{{ formatCreatedAt(detail.item.recorded_at) }} · 기록 {{ detail.item.seq }}</dd></div><div v-if="detail.head.previous_version_id"><dt>최근 정정 시각</dt><dd>{{ formatCreatedAt(detail.head.recorded_at) }} · 기록 {{ detail.head.seq }}</dd></div><div><dt>판매 ID</dt><dd class="material-id">{{ detail.item.id }}</dd></div></dl><SaleCostSnapshot :cost="detail.item.recipe_cost"/><NuxtLink :to="`/recipes/${detail.item.material_id}/versions/${detail.item.recipe_version_id}`">판매 당시 레시피 버전 보기</NuxtLink></section>
    <SaleConsumption :status="detail.consumption_status" :rows="detail.consumption"/>
    <div class="button-row"><NuxtLink class="primary-link" :to="{ path: '/business', query: useCommonCorrection ? { kind: 'correction', corrects: detail.item.original_confirmation_id } : { kind: 'sale', record: detail.head.business_record_id, corrects: detail.head.confirmation_id } }">판매 개수 정정하기</NuxtLink><NuxtLink class="secondary-link" :to="`/business/${detail.head.business_record_id}`">현재 판매 업무 기록 보기</NuxtLink></div><p class="field-hint">개수 정정은 당시 레시피·판매가·원가를 유지합니다. 고객 환불이나 실물 반환을 기록하지 않습니다.</p>
    <section class="history-section" aria-labelledby="sale-history-title"><h2 id="sale-history-title">원본과 개수 정정 이력</h2><ol class="history-list"><li v-for="version in detail.versions" :key="version.id"><div><NuxtLink :to="`/business/${version.business_record_id}`">{{ version.previous_version_id ? '개수 정정' : '원본 판매' }}</NuxtLink><span v-if="version.id === detail.head.id" class="status-label">현재 유효</span></div><p>{{ displayDecimal(version.count) }}개 · {{ version.amount === null ? '판매금액 미계산' : `${displayDecimal(version.amount)}원` }}</p><p class="preserve-lines">{{ version.note || '메모 없음' }}</p><time>{{ formatCreatedAt(version.recorded_at) }} · 기록 {{ version.seq }}</time></li></ol></section>
    <CorrectionHistoryPanel :confirmation-id="detail.item.original_confirmation_id" @loaded="effectiveRecord = $event"/>
  </template>
</template>
