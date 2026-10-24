<script setup lang="ts">
import type { CorrectionPayload, CorrectionReviewRequest, CorrectionSeed } from '~/types/correction'
import type { ReceiptDetail } from '~/types/receipt'
import { apiRequest } from '~/utils/api'
import { cloneCorrection, correctionReferenceVersion } from '~/utils/correction'
import { displayDecimal, priceSources } from '~/utils/catalog'
const data = defineModel<CorrectionPayload['data']>({ required: true })
const props = defineProps<{ requests: CorrectionReviewRequest[]; disabled: boolean; mainReceiptId?: string }>()
const loading = ref<string | null>(null)
const failure = ref('')
let active = true
onScopeDispose(() => { active = false })
const valuationRequest = (id: string | null) => props.requests.find((request): request is Extract<CorrectionReviewRequest, { kind: 'observation_valuation' }> => request.kind === 'observation_valuation' && request.observation.id === id)
const receiptRequest = (id: string | null) => props.requests.find((request): request is Extract<CorrectionReviewRequest, { kind: 'receipt_inclusions' }> => request.kind === 'receipt_inclusions' && request.receipt_id === id)
function addObservation(request: Extract<CorrectionReviewRequest, { kind: 'observation_valuation' }>) {
  if (props.disabled || data.value.observation_reviews.some(row => row.observation_id === request.observation.id)) return
  data.value.observation_reviews.push({ observation_id: request.observation.id, expected_version_id: correctionReferenceVersion(request.observation), initial_value: null, excess: request.mode === 'excess' ? { reviewed: false, reason: '', value: null } : null })
}
async function addReceipt(request: Extract<CorrectionReviewRequest, { kind: 'receipt_inclusions' }>) {
  if (props.disabled || loading.value || request.receipt_id === props.mainReceiptId || data.value.linked_receipt_changes.some(row => row.receipt_id === request.receipt_id)) return
  const fingerprint = JSON.stringify(data.value)
  loading.value = request.receipt_id
  failure.value = ''
  try {
    const receipt = await apiRequest<ReceiptDetail>(`/api/receipts/${request.receipt_id}`)
    const seed = await apiRequest<CorrectionSeed>(`/api/business/confirmations/${receipt.item.confirmation_id}/correction-seed`)
    if (!active || props.disabled || JSON.stringify(data.value) !== fingerprint) return
    if (seed.current.reference.version_id !== request.expected_version_id || seed.current.input?.kind !== 'receipt') throw new Error('changed')
    data.value.linked_receipt_changes.push({ receipt_id: request.receipt_id, expected_version_id: request.expected_version_id, action: { kind: 'replace', data: cloneCorrection(seed.current.input.data) } })
  } catch { if (active) failure.value = '관련 입고가 바뀌었거나 조회하지 못했습니다. 입력을 유지했습니다. 다시 미리보기하여 영향을 확인해 주세요.' }
  finally { if (active) loading.value = null }
}
</script>
<template>
  <section class="correction-reviews" aria-labelledby="correction-reviews-title"><h3 id="correction-reviews-title">함께 검토할 기록</h3><p class="field-hint">아래 안내는 마지막 미리보기에서 확인한 영향입니다. 입력을 바꾸면 임시 저장 후 다시 미리보기해야 확정할 수 있습니다.</p><p v-if="failure" class="notice warning" role="alert">{{ failure }}</p>
    <ul v-if="requests.length" class="history-list"><li v-for="(request, index) in requests" :key="`${request.kind}-${index}`"><p>{{ request.message }}</p><template v-if="request.kind === 'observation_valuation'"><p><strong>{{ request.material_name }}</strong> · 실제 확인 수량 {{ displayDecimal(request.measured_quantity) }}{{ request.unit }}<span v-if="request.excess_quantity !== null"> · 정정 후 초과 {{ displayDecimal(request.excess_quantity) }}{{ request.unit }}</span></p><p>해당 시점의 단가 근거: {{ priceSources[request.proposed_price.source] }} <template v-if="request.proposed_price.amount !== null && request.proposed_price.quantity !== null">· {{ displayDecimal(request.proposed_price.amount) }}원 / {{ displayDecimal(request.proposed_price.quantity) }}{{ request.unit }}</template></p><button class="secondary-button" type="button" :disabled="disabled || data.observation_reviews.some(row => row.observation_id === request.observation.id)" @click="addObservation(request)">이 실사의 가치 확인 내용 작성</button></template><template v-else><NuxtLink :to="`/receipts/${request.receipt_id}`">영향받는 입고 보기</NuxtLink><p v-if="request.receipt_id === mainReceiptId" class="field-hint">위 입고 정정 입력에서 포함 근거를 검토하세요.</p><button v-else class="secondary-button" type="button" :disabled="disabled || !!loading || data.linked_receipt_changes.some(row => row.receipt_id === request.receipt_id)" @click="addReceipt(request)">이 입고의 연결도 함께 정정</button></template></li></ul>
    <fieldset v-for="(row, index) in data.observation_reviews" :key="row.observation_id ?? index" class="correction-review-row"><legend>실사 가치 검토 {{ index + 1 }} · {{ valuationRequest(row.observation_id)?.material_name ?? row.observation_id?.slice(0, 8) }}</legend><p v-if="valuationRequest(row.observation_id) && row.expected_version_id !== correctionReferenceVersion(valuationRequest(row.observation_id)!.observation)" class="notice warning">검토한 실사 버전이 바뀌었습니다. 이 검토 입력을 제거하고 현재 안내에서 다시 선택해 주세요.</p><CorrectionValuationFields v-model="data.observation_reviews[index]!" :prefix="`correction-review-${index}`" :field="`observation_reviews[${index}]`" :disabled="disabled" :mode="valuationRequest(row.observation_id)?.mode" :quantity="valuationRequest(row.observation_id)?.mode === 'initial' ? valuationRequest(row.observation_id)?.measured_quantity : undefined" :existing-allowed="valuationRequest(row.observation_id) ? valuationRequest(row.observation_id)!.proposed_price.source !== 'missing' : undefined" :manual-allowed="valuationRequest(row.observation_id) ? valuationRequest(row.observation_id)!.proposed_price.source === 'missing' || valuationRequest(row.observation_id)!.mode === 'initial' : undefined"/><button class="text-button" type="button" :disabled="disabled" @click="data.observation_reviews.splice(index, 1)">이 실사 검토 입력 제거</button></fieldset>
    <fieldset v-for="(row, index) in data.linked_receipt_changes" :key="row.receipt_id ?? index" class="correction-linked-row"><legend>관련 입고 정정 {{ index + 1 }} · {{ row.receipt_id?.slice(0, 8) }}</legend><NuxtLink v-if="row.receipt_id" :to="`/receipts/${row.receipt_id}`">입고 원기록 보기</NuxtLink><p v-if="receiptRequest(row.receipt_id) && row.expected_version_id !== receiptRequest(row.receipt_id)!.expected_version_id" class="notice warning">연결 입고 버전이 바뀌었습니다. 입력을 제거하고 최신 영향에서 다시 선택해 주세요.</p><template v-if="row.action.kind === 'replace'"><CorrectionReceiptFields v-model="row.action.data" :prefix="`correction-linked-${index}`" :field="`linked_receipt_changes[${index}].action.data`" :disabled="disabled" :proposed="receiptRequest(row.receipt_id)?.candidates"/><button class="secondary-button" type="button" :disabled="disabled" @click="row.action = { kind: 'void' }">이 연결 입고의 유효 효과 취소 선택</button></template><p v-else class="notice warning">이 입고의 유효 효과를 함께 취소합니다. 원기록은 남습니다. 입력을 다시 작성하려면 아래에서 이 변경을 제거한 뒤 영향 목록에서 다시 선택하세요.</p><button class="text-button" type="button" :disabled="disabled" @click="data.linked_receipt_changes.splice(index, 1)">이 연결 입고 변경 제거</button></fieldset>
    <p v-if="!requests.length && !data.observation_reviews.length && !data.linked_receipt_changes.length" class="field-hint">정정 내용을 임시 저장하고 미리보기하면 추가로 검토할 실사와 입고를 확인할 수 있습니다.</p>
  </section>
</template>
