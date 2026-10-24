<script setup lang="ts">
import type { BusinessDetail } from '~/types/business'
import type { BusinessEditorInitial } from '~/composables/useBusinessDraft'
import type { CorrectionPayload, CorrectionReviewRequest, CorrectionSeed } from '~/types/correction'
import { apiRequest } from '~/utils/api'
import { cloneCorrection, correctionKinds } from '~/utils/correction'
import { validateBusinessDraft } from '~/utils/business'
import { formatCreatedAt } from '~/utils/material'
const props = defineProps<{ identity: string; initial: BusinessEditorInitial<CorrectionPayload>; refresh: () => Promise<BusinessDetail | null>; seed?: CorrectionSeed | null }>()
const { payload, status, issues, preview, busy, failure, notice, storageFailure, created, pending, dirty, locked, canRetry, canReview, send, showPreview, review, synchronizeTerminal } = useBusinessDraft(props.identity, props.initial, props.refresh)
watch(() => props.initial, synchronizeTerminal, { immediate: true })
const connection = useConnection()
const available = computed(() => connection.value === 'connected')
const target = ref<CorrectionSeed | null>(props.seed ?? null)
const targetFailure = ref('')
const checking = ref(false)
const errors = ref<Record<string, string>>({})
const form = ref<HTMLFormElement | null>(null)
const previewPanel = ref<HTMLElement | null>(null)
const reviewContext = ref<CorrectionReviewRequest[]>([])
const archiveReview = ref(false)
const staleTarget = computed(() => target.value && payload.value.data.expected_target_version_id !== target.value.current.reference.version_id)
const mainReceiptId = computed(() => target.value?.original.reference.kind === 'receipt' ? target.value.original.reference.original_id : undefined)
const mainCandidates = computed(() => reviewContext.value.find((request): request is Extract<CorrectionReviewRequest, { kind: 'receipt_inclusions' }> => request.kind === 'receipt_inclusions' && request.receipt_id === mainReceiptId.value)?.candidates)
let active = true, generation = 0
onScopeDispose(() => { active = false; generation += 1 })
async function loadTarget() {
  const current = ++generation
  if (!props.initial.corrects || !available.value) return
  checking.value = true
  targetFailure.value = ''
  try { const seed = await apiRequest<CorrectionSeed>(`/api/business/confirmations/${encodeURIComponent(props.initial.corrects)}/correction-seed`); if (active && current === generation) target.value = seed }
  catch { if (active && current === generation) targetFailure.value = '정정 대상의 현재 기록을 확인하지 못했습니다. 작성 중인 입력과 재시도 요청은 유지했습니다.' }
  finally { if (active && current === generation) checking.value = false }
}
watch(available, () => { if (!target.value) void loadTarget() }, { immediate: true })
function applyTarget() { if (!locked.value && target.value) payload.value.data.expected_target_version_id = target.value.current.reference.version_id }
function action(event: Event) {
  if (locked.value) return
  if ((event.target as HTMLSelectElement).value === 'void') payload.value.data.action = { kind: 'void' }
  else { const input = target.value?.replacement_template; if (input) payload.value.data.action = { kind: 'replace', replacement: cloneCorrection(input) } }
}
function issue(field: string) { return errors.value[field] ?? issues.value.find(item => item.field.replace(/^data\./, '') === field)?.message }
async function focusIssue() {
  await nextTick()
  const field = (Object.keys(errors.value)[0] ?? issues.value[0]?.field)?.replace(/^data\./, '')
  const inputs = Array.from(form.value?.querySelectorAll<HTMLElement>('[data-field]') ?? [])
  const exact = inputs.find(input => input.dataset.field === field)
  const parent = field ? inputs.find(input => field.startsWith(`${input.dataset.field}.`) || field.startsWith(`${input.dataset.field}[`)) : null
  ;(exact ?? parent ?? previewPanel.value ?? form.value)?.focus()
}
async function saveDraft() { errors.value = validateBusinessDraft(payload.value); if (Object.keys(errors.value).length) { await focusIssue(); return }; await send(props.initial.recordId ? 'revise' : 'create') }
async function previewDraft() { errors.value = {}; await showPreview(); if (preview.value?.effect?.kind === 'correction') reviewContext.value = cloneCorrection(preview.value.effect.correction.review_requests); await nextTick(); if (preview.value) previewPanel.value?.focus(); else await focusIssue() }
async function reviewRequest() { if (await review()) await loadTarget() }
</script>
<template>
  <section class="business-editor correction-editor" aria-label="확정 기록 정정 입력" :aria-busy="busy">
    <p v-if="storageFailure" class="notice warning" role="alert">{{ storageFailure }}</p><div v-if="failure" class="notice warning" role="alert"><p>{{ failure }}</p><button v-if="canReview" class="text-button" type="button" :disabled="busy || !available" @click="reviewRequest">최신 기록 확인 후 입력 검토</button></div><p v-if="pending && !failure" class="notice warning" role="status">{{ busy ? '처리 결과를 확인하고 있습니다.' : '완료를 확인하지 못한 요청을 보존했습니다.' }}</p><button v-if="canRetry && !busy" class="secondary-button" type="button" :disabled="!available" @click="send(pending!.action)">같은 업무 요청 재시도</button><p v-if="notice" class="notice success" role="status">{{ notice }}</p><NuxtLink v-if="created" class="business-saved-link" :to="`/business/${created.id}`">저장한 초안 열기</NuxtLink>
    <section v-if="target" class="correction-target"><h2>{{ correctionKinds[target.original.reference.kind] }} 정정 대상</h2><p>원래 기록 {{ target.original.reference.logical_seq }} · {{ formatCreatedAt(target.original.reference.original_recorded_at) }}</p><NuxtLink :to="`/business/${target.original.reference.business_record_id}`">원본 업무 기록 보기</NuxtLink><details><summary>현재 유효한 입력 확인</summary><CorrectionEventValue :value="target.current.value"/></details><details><summary>최초 확정 입력 확인</summary><CorrectionEventValue :value="target.original.value"/></details><p class="material-id">확인한 대상 버전 {{ payload.data.expected_target_version_id ?? '아직 미확인' }}</p><button class="text-button" type="button" :disabled="checking || !available || busy" @click="loadTarget">정정 대상 새로고침</button></section><p v-if="targetFailure" class="notice warning" role="alert">{{ targetFailure }}</p>
    <div v-if="staleTarget" class="notice warning" role="alert"><p>확인한 대상 버전과 현재 유효한 버전이 다릅니다. 현재 내용을 비교하고 아래 버튼으로 검토한 버전을 선택하세요. 입력은 자동으로 바꾸지 않습니다.</p><button class="secondary-button" type="button" :disabled="locked || !available" @click="applyTarget">현재 대상 버전을 확인하고 입력 유지</button></div>
    <section v-if="status !== 'draft' && dirty" class="notice warning"><h3>이 화면에 남은 미저장 정정 입력</h3><p>다른 화면에서 확정하거나 보관하여 이 입력은 반영되지 않았습니다.</p><p class="preserve-lines">{{ payload.data.reason }}</p></section>
    <form v-if="status === 'draft' && !created" ref="form" novalidate tabindex="-1" @submit.prevent="saveDraft"><h2>정정 초안</h2><div class="field"><label for="correction-reason">정정 이유</label><textarea id="correction-reason" v-model="payload.data.reason" data-field="reason" rows="3" :disabled="locked || !available" :aria-invalid="!!issue('reason')" aria-describedby="correction-reason-error"/><p id="correction-reason-error" class="field-error">{{ issue('reason') }}</p><p class="field-hint">미완성 초안은 저장할 수 있습니다. 확정할 때는 이유가 필요합니다.</p></div><div class="field"><label for="correction-action">정정 방법</label><select id="correction-action" data-field="action" :value="payload.data.action.kind" :disabled="locked || !available" @change="action"><option value="replace" :disabled="!target?.replacement_template">입력 내용 정정</option><option value="void">이 기록의 유효 효과 취소</option></select></div>
      <CorrectionReplacementFields v-if="payload.data.action.kind === 'replace'" v-model="payload.data.action.replacement" :disabled="locked || !available" :original="target?.replacement_template" :proposed="mainCandidates"/>
      <p v-else class="notice warning">원기록은 남기고 그 기록의 유효 효과를 취소합니다. 수량이나 금액을 0으로 바꾼 새 기록과 다릅니다. 연관된 실사·입고·판매 배분에 미치는 영향을 미리보기에서 확인하세요.</p>
      <CorrectionReviewFields v-model="payload.data" :requests="reviewContext" :disabled="locked || !available" :main-receipt-id="mainReceiptId"/>
      <p v-if="dirty" class="draft-note" role="status">화면에서 바꾼 내용이 아직 임시 저장되지 않았습니다.</p><div class="button-row"><button class="primary-button" type="submit" :disabled="locked || !available">{{ initial.recordId ? '수정 내용 임시 저장' : '초안 임시 저장' }}</button><button v-if="initial.recordId" class="secondary-button" type="button" :disabled="locked || !available || dirty" @click="previewDraft">확정 전 미리보기</button></div>
    </form>
    <section v-if="preview" ref="previewPanel" class="business-preview correction-preview" tabindex="-1" aria-labelledby="correction-preview-title"><h2 id="correction-preview-title">정정 영향 미리보기</h2><ul v-if="preview.issues.length" class="business-issues"><li v-for="item in preview.issues" :key="`${item.field}-${item.code}`">{{ item.message }}</li></ul><p v-if="!preview.can_confirm" class="notice warning">아직 확정할 수 없습니다. 필요한 입력과 함께 검토할 기록을 확인해 주세요.</p><CorrectionPlanPanel v-if="preview.effect?.kind === 'correction'" :plan="preview.effect.correction"/><button class="primary-button" type="button" :disabled="!available || busy || !!pending || dirty || !preview.can_confirm" @click="send('confirm')">이 내용으로 확정</button></section>
    <section v-if="initial.recordId && status === 'draft'" class="archive-section"><button v-if="!archiveReview" class="text-button" type="button" :disabled="locked || !available" @click="archiveReview = true">초안 보관</button><div v-else class="notice warning"><p>정정 초안을 보관합니다. 원래 확정 기록은 바뀌지 않습니다.</p><button class="secondary-button" type="button" :disabled="locked || !available" @click="send('archive')">초안 보관 확정</button><button class="text-button" type="button" :disabled="locked" @click="archiveReview = false">취소</button></div></section>
  </section>
</template>
