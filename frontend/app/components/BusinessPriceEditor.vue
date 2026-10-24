<script setup lang="ts">
import type { BusinessDetail, DraftPriceV1 } from '~/types/business'
import type { BusinessEditorInitial } from '~/composables/useBusinessDraft'
import type { Material } from '~/types/material'
import { apiRequest } from '~/utils/api'
import { displayDecimal } from '~/utils/catalog'
import { validateBusinessDraft } from '~/utils/business'

const props = defineProps<{ identity: string; initial: BusinessEditorInitial; refresh: () => Promise<BusinessDetail | null> }>()
const { payload, status, issues, preview, busy, failure, notice, storageFailure, created, pending, dirty, locked, canRetry, canReview, send, showPreview, review, resetNew, synchronizeTerminal } = useBusinessDraft(props.identity, props.initial, props.refresh)
watch(() => props.initial, synchronizeTerminal, { immediate: true })
const connection = useConnection()
const available = computed(() => connection.value === 'connected')
const materials = ref<Material[]>([])
const materialFailure = ref('')
const choosing = ref(false)
const form = ref<HTMLFormElement | null>(null)
const previewPanel = ref<HTMLElement | null>(null)
const errors = ref<Partial<Record<keyof DraftPriceV1, string>>>({})
const archiveReview = ref(false)
const selected = computed(() => materials.value.find(item => item.id === payload.value.data.material_id))
const choices = computed(() => materials.value.filter(item => !item.archived || item.id === payload.value.data.material_id))
let activeComponent = true
onScopeDispose(() => { activeComponent = false })
async function loadMaterials() { try { materials.value = await apiRequest<Material[]>('/api/materials?archive=all'); materialFailure.value = '' } catch { materialFailure.value = '재료 목록을 불러오지 못했습니다.' } }
watch(available, value => { if (value) void loadMaterials() }, { immediate: true })
function selectMaterial(event: Event) {
  const item = materials.value.find(value => value.id === (event.target as HTMLSelectElement).value)
  payload.value.data.material_id = item?.id ?? null
  payload.value.data.material_version_id = item?.version_id ?? null
  if (item) payload.value.data.unit = item.unit
  errors.value = {}
}
async function currentMaterial() {
  if (!payload.value.data.material_id || locked.value || choosing.value) return
  const requestedId = payload.value.data.material_id
  choosing.value = true
  try {
    const item = await apiRequest<Material>(`/api/materials/${encodeURIComponent(requestedId)}`)
    if (!activeComponent || payload.value.data.material_id !== requestedId || locked.value) return
    payload.value.data.material_version_id = item.version_id
    materials.value = [item, ...materials.value.filter(value => value.id !== item.id)]
    materialFailure.value = ''
  } catch { materialFailure.value = '현재 재료 정보를 확인하지 못했습니다.' }
  finally { choosing.value = false }
}
function fieldIssue(field: keyof DraftPriceV1) { return errors.value[field] ?? issues.value.find(issue => issue.field === field || issue.field === `data.${field}`)?.message }
async function focusIssue(client = false) {
  await nextTick()
  const field = client ? Object.keys(errors.value)[0] : issues.value[0]?.field
  const targetId = field === 'material_version_id' ? 'business-current-material' : field === 'material_id' ? 'business-material' : field ? `business-${field}` : null
  const target = targetId ? form.value?.querySelector<HTMLElement>(`#${targetId}`) : form.value?.querySelector<HTMLElement>('[aria-invalid="true"]')
  if (target) target.focus()
  else previewPanel.value?.focus()
}
async function saveDraft() {
  errors.value = validateBusinessDraft(payload.value)
  if (Object.keys(errors.value).length) { await focusIssue(true); return }
  await send(props.initial.recordId ? 'revise' : 'create')
}
async function previewDraft() {
  if (await showPreview()) { await nextTick(); previewPanel.value?.focus() }
  else await focusIssue()
}
async function reviewInput() { await review(); await nextTick(); form.value?.querySelector<HTMLElement>('select, input')?.focus() }
</script>

<template>
  <section class="business-editor" aria-label="사전 가격 초안 입력" :aria-busy="busy">
    <p v-if="storageFailure" class="notice warning" role="alert">{{ storageFailure }}</p>
    <div v-if="failure" class="notice warning" role="alert"><p>{{ failure }}</p><button v-if="canReview" type="button" class="text-button" :disabled="busy || !available" @click="reviewInput">최신 기록 확인 후 입력 검토</button></div>
    <p v-if="pending && !failure" class="notice warning" role="status">{{ busy ? '처리 결과를 확인하고 있습니다.' : '완료를 확인하지 못한 전송 요청이 있습니다. 같은 요청으로 재시도해 주세요.' }}</p>
    <button v-if="canRetry && !busy" type="button" class="secondary-button" :disabled="!available" @click="send(pending!.action)">같은 업무 요청 재시도</button>
    <p v-if="notice" class="notice success" role="status">{{ notice }}</p>
    <NuxtLink v-if="created" class="business-saved-link" :to="`/business/${created.id}`">저장한 초안 열기</NuxtLink>
    <button v-if="created && !initial.corrects" type="button" class="text-button" :disabled="busy" @click="resetNew">새 초안 작성</button>
    <section v-if="status !== 'draft' && dirty" class="notice warning" aria-labelledby="business-unsaved-title"><h2 id="business-unsaved-title">이 화면에 남은 미저장 입력</h2><p>다른 화면에서 업무가 {{ status === 'confirmed' ? '확정' : '보관' }}되어 이 입력은 반영되지 않았습니다. 위의 저장된 업무 내용과 구분해 보관합니다.</p><p>{{ payload.data.quantity || '수량 미입력' }} {{ payload.data.unit || '단위 미입력' }} / {{ payload.data.amount === '' ? '금액 미입력' : `${payload.data.amount}원` }}</p><p class="preserve-lines">{{ payload.data.note }}</p></section>
    <form v-if="status === 'draft' && !created" ref="form" novalidate @submit.prevent="saveDraft">
      <div class="section-heading"><h2>{{ initial.recordId ? '저장된 초안 수정' : initial.corrects ? '가격 정정 초안' : '사전 가격 초안 작성' }}</h2><p>빈칸이 있어도 임시 저장할 수 있습니다. 가격은 미리보기를 확인하고 확정한 뒤 반영됩니다.</p></div>
      <div class="field"><label for="business-material">재료</label><select id="business-material" :value="payload.data.material_id ?? ''" :disabled="!available || locked || choosing || !!initial.corrects" :aria-invalid="!!fieldIssue('material_id') || !!fieldIssue('material_version_id')" aria-describedby="business-material-issue" @change="selectMaterial"><option value="">재료 선택</option><option v-if="payload.data.material_id && !selected" :value="payload.data.material_id">참조한 재료 확인 필요</option><option v-for="item in choices" :key="item.id" :value="item.id">{{ item.name }}{{ item.archived ? ' (보관됨)' : '' }} · {{ item.id.slice(0, 8) }}</option></select><p id="business-material-issue" class="field-error">{{ fieldIssue('material_id') || fieldIssue('material_version_id') }}</p><button v-if="payload.data.material_id" id="business-current-material" type="button" class="text-button" :disabled="!available || locked || choosing" @click="currentMaterial">현재 재료 정보 적용</button><p v-if="initial.corrects" class="field-hint">정정 대상 재료는 바꿀 수 없습니다.</p></div>
      <p v-if="materialFailure" class="notice warning" role="alert">{{ materialFailure }} <button type="button" class="text-button" :disabled="!available" @click="loadMaterials">재료 다시 불러오기</button></p>
      <div class="business-price-fields">
        <div class="field"><label for="business-quantity">가격 기준 수량</label><input id="business-quantity" v-model="payload.data.quantity" inputmode="decimal" :readonly="locked" :disabled="!available || choosing" :aria-invalid="!!fieldIssue('quantity')" aria-describedby="business-quantity-issue" @input="errors.quantity = undefined"><p id="business-quantity-issue" class="field-error">{{ fieldIssue('quantity') }}</p></div>
        <div class="field"><label for="business-unit">가격 기준 단위</label><input id="business-unit" v-model="payload.data.unit" :readonly="locked" :disabled="!available || choosing" :aria-invalid="!!fieldIssue('unit')" aria-describedby="business-unit-issue" @input="errors.unit = undefined"><p id="business-unit-issue" class="field-error">{{ fieldIssue('unit') }}</p></div>
        <div class="field"><label for="business-amount">기준 금액 (원)</label><input id="business-amount" v-model="payload.data.amount" inputmode="decimal" placeholder="0원도 입력할 수 있습니다" :readonly="locked" :disabled="!available || choosing" :aria-invalid="!!fieldIssue('amount')" aria-describedby="business-amount-issue" @input="errors.amount = undefined"><p id="business-amount-issue" class="field-error">{{ fieldIssue('amount') }}</p></div>
      </div>
      <div class="field"><label for="business-note">{{ initial.corrects ? '정정 이유' : '메모' }}</label><textarea id="business-note" v-model="payload.data.note" rows="3" :readonly="locked" :disabled="!available || choosing" :aria-invalid="!!fieldIssue('note')" aria-describedby="business-note-issue" @input="errors.note = undefined"/><p id="business-note-issue" class="field-error">{{ fieldIssue('note') }}</p></div>
      <p v-if="dirty" class="draft-note" role="status">화면에서 바꾼 내용이 아직 임시 저장되지 않았습니다.</p>
      <div class="button-row"><button type="submit" class="primary-button" :disabled="!available || locked || choosing">{{ initial.recordId ? '수정 내용 임시 저장' : '초안 임시 저장' }}</button><button v-if="initial.recordId" type="button" class="secondary-button" :disabled="!available || locked || choosing || dirty" @click="previewDraft">확정 전 미리보기</button></div>
      <p class="field-hint">임시 저장한 초안은 이 PC의 자료에 남습니다. 전송 중 요청은 완료가 확인될 때까지 같은 내용으로 유지합니다.</p>
    </form>
    <section v-if="preview" ref="previewPanel" class="business-preview" tabindex="-1" aria-labelledby="business-preview-title">
      <h2 id="business-preview-title">확정 전 미리보기</h2>
      <ul v-if="preview.issues.length" class="business-issues"><li v-for="issue in preview.issues" :key="`${issue.field}-${issue.code}`">{{ issue.message }}</li></ul>
      <template v-if="preview.effect">
        <dl class="business-price-comparison">
          <div v-if="preview.effect.corrects_confirmation_id"><dt>정정 대상 가격</dt><dd>{{ preview.effect.corrected_price ? `${displayDecimal(preview.effect.corrected_price.amount)}원 / ${displayDecimal(preview.effect.corrected_price.quantity)}${preview.effect.corrected_price.unit}` : '대상 가격 확인 필요' }}</dd></div>
          <div><dt>현재 사전 가격</dt><dd>{{ preview.effect.previous_price ? `${displayDecimal(preview.effect.previous_price.amount)}원 / ${displayDecimal(preview.effect.previous_price.quantity)}${preview.effect.previous_price.unit}` : '가격 없음' }}</dd></div>
          <div><dt>새 사전 가격</dt><dd class="preview-result">{{ displayDecimal(preview.effect.quantity) }} {{ preview.effect.unit }} 기준 {{ displayDecimal(preview.effect.amount) }}원</dd></div>
        </dl>
        <p>확정하면 새 가격 기록을 남깁니다. 보유 수량을 늘리거나 과거 가격과 원가를 다시 계산하지 않습니다.</p>
      </template>
      <button type="button" class="primary-button" :disabled="!available || busy || !!pending || dirty || !preview.can_confirm" @click="send('confirm')">이 내용으로 확정</button>
    </section>
    <section v-if="initial.recordId && status === 'draft'" class="archive-section" aria-label="초안 보관"><p>이어 쓸 필요가 없는 초안은 보관할 수 있습니다. 입력 이력은 남습니다.</p><button v-if="!archiveReview" type="button" class="text-button" :disabled="!available || locked" @click="archiveReview = true">초안 보관</button><div v-else class="notice warning"><p>초안을 보관할까요? 화면의 미저장 입력은 함께 저장되지 않습니다.</p><div class="button-row"><button type="button" class="secondary-button" :disabled="!available || locked" @click="send('archive')">초안 보관 확정</button><button type="button" class="text-button" :disabled="locked" @click="archiveReview = false">취소</button></div></div></section>
  </section>
</template>
