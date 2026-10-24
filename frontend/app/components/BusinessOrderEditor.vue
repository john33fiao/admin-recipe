<script setup lang="ts">
import type { BusinessDetail } from '~/types/business'
import type { BusinessEditorInitial } from '~/composables/useBusinessDraft'
import type { Material } from '~/types/material'
import type { OrderDetail, OrderPayload } from '~/types/purchase-order'
import { apiRequest } from '~/utils/api'
import { businessInputText, validateBusinessDraft } from '~/utils/business'
import { displayDecimal } from '~/utils/catalog'
import { orderActions, orderStatuses } from '~/utils/purchase-order'

const props = defineProps<{ identity: string; initial: BusinessEditorInitial<OrderPayload>; refresh: () => Promise<BusinessDetail | null> }>()
const editor = useBusinessDraft(props.identity, props.initial, props.refresh)
const { payload, status, issues, preview, busy, failure, notice, storageFailure, created, pending, dirty, locked, canRetry, canReview, send, showPreview, review, resetNew, synchronizeTerminal } = editor
watch(() => props.initial, synchronizeTerminal, { immediate: true })
const connection = useConnection()
const available = computed(() => connection.value === 'connected')
const materials = ref<Material[]>([])
const order = ref<OrderDetail | null>(null)
const referenceFailure = ref('')
const checking = ref(false)
const errors = ref<Partial<Record<string, string>>>({})
const form = ref<HTMLFormElement | null>(null)
const previewPanel = ref<HTMLElement | null>(null)
const archiveReview = ref(false)
const selectedId = computed(() => payload.value.data.action === 'place' ? payload.value.data.material_id : order.value?.item.material_id)
const choices = computed(() => materials.value.filter(value => !value.archived || value.id === selectedId.value))
const targetId = computed(() => payload.value.data.action === 'place' ? null : payload.value.data.order_id)
let active = true
let readGeneration = 0
onScopeDispose(() => { active = false; readGeneration += 1 })
async function loadReferences() {
  const generation = ++readGeneration
  if (!available.value) return
  try {
    if (targetId.value) {
      const id = targetId.value
      const value = await apiRequest<OrderDetail>(`/api/orders/${encodeURIComponent(id)}`)
      if (active && generation === readGeneration && targetId.value === id) order.value = value
    } else {
      const values = await apiRequest<Material[]>('/api/materials?archive=all')
      if (active && generation === readGeneration) materials.value = values
    }
    if (active && generation === readGeneration) referenceFailure.value = ''
  } catch { if (active && generation === readGeneration) referenceFailure.value = '참조 자료를 불러오지 못했습니다. 다시 확인해 주세요.' }
}
watch([available, targetId], () => { void loadReferences() }, { immediate: true })
function selectMaterial(event: Event) {
  if (payload.value.data.action !== 'place') return
  const item = materials.value.find(value => value.id === (event.target as HTMLSelectElement).value)
  payload.value.data.material_id = item?.id ?? null
  payload.value.data.material_version_id = item?.version_id ?? null
  if (item) payload.value.data.unit = item.unit
}
async function applyCurrent() {
  if (locked.value || checking.value || !available.value) return
  const captured = { ...payload.value.data }
  checking.value = true
  try {
    if (captured.action === 'place') {
      if (!captured.material_id) return
      const item = await apiRequest<Material>(`/api/materials/${encodeURIComponent(captured.material_id)}`)
      if (!active || locked.value || payload.value.data.action !== 'place' || payload.value.data.material_id !== captured.material_id || payload.value.data.material_version_id !== captured.material_version_id) return
      payload.value.data.material_version_id = item.version_id
      materials.value = [item, ...materials.value.filter(value => value.id !== item.id)]
    } else {
      const latest = await apiRequest<OrderDetail>(`/api/orders/${encodeURIComponent(captured.order_id)}`)
      const material = captured.action === 'revise' && latest.item.status === 'open' ? await apiRequest<Material>(`/api/materials/${encodeURIComponent(latest.item.material_id)}`) : null
      const current = payload.value.data
      if (!active || locked.value || current.action !== captured.action || current.order_id !== captured.order_id || current.expected_order_version_id !== captured.expected_order_version_id) return
      order.value = latest
      if (latest.item.status !== 'open') { referenceFailure.value = '이미 닫힌 주문은 수정하거나 취소할 수 없습니다. 작성 중인 초안은 보관할 수 있습니다.'; return }
      current.expected_order_version_id = latest.head.id
      if (current.action === 'revise' && material) current.material_version_id = material.version_id
    }
    referenceFailure.value = ''
  } catch { referenceFailure.value = '최신 참조를 확인하지 못했습니다. 다시 시도해 주세요.' }
  finally { checking.value = false }
}
function issue(field: string) { return errors.value[field] ?? issues.value.find(value => value.field === field)?.message }
async function focusIssue(client = false) {
  await nextTick()
  const field = client ? Object.keys(errors.value)[0] : issues.value[0]?.field
  const refFields = ['order_id', 'expected_order_version_id', 'material_version_id']
  const id = field && refFields.includes(field) ? 'order-current-reference' : field === 'material_id' ? 'order-material' : field ? `order-${field}` : ''
  const target = id ? form.value?.querySelector<HTMLElement>(`#${id}`) : null
  if (target) target.focus(); else previewPanel.value?.focus()
}
async function saveDraft() { errors.value = validateBusinessDraft(payload.value); if (Object.keys(errors.value).length) { await focusIssue(true); return }; await send(props.initial.recordId ? 'revise' : 'create') }
async function previewDraft() { if (await showPreview()) { await nextTick(); previewPanel.value?.focus() } else await focusIssue() }
</script>

<template>
  <section class="business-editor" aria-label="발주 업무 초안 입력" :aria-busy="busy">
    <p v-if="storageFailure" class="notice warning" role="alert">{{ storageFailure }}</p>
    <div v-if="failure" class="notice warning" role="alert"><p>{{ failure }}</p><button v-if="canReview" class="text-button" type="button" :disabled="busy || !available" @click="review">최신 기록 확인 후 입력 검토</button></div>
    <p v-if="pending && !failure" class="notice warning" role="status">{{ busy ? '처리 결과를 확인하고 있습니다.' : '완료를 확인하지 못한 전송 요청이 있습니다. 같은 요청으로 재시도해 주세요.' }}</p>
    <button v-if="canRetry && !busy" class="secondary-button" type="button" :disabled="!available" @click="send(pending!.action)">같은 업무 요청 재시도</button>
    <p v-if="notice" class="notice success" role="status">{{ notice }}</p>
    <NuxtLink v-if="created" class="business-saved-link" :to="`/business/${created.id}`">저장한 초안 열기</NuxtLink>
    <button v-if="created && payload.data.action === 'place'" class="text-button" type="button" :disabled="busy" @click="resetNew">새 주문 초안 작성</button>
    <section v-if="status !== 'draft' && dirty" class="notice warning" aria-labelledby="order-unsaved-title"><h2 id="order-unsaved-title">이 화면에 남은 미저장 입력</h2><p>다른 화면에서 확정하거나 보관해 이 입력은 반영되지 않았습니다.</p><p>{{ businessInputText(payload) }}</p><p class="preserve-lines">{{ payload.data.note }}</p></section>
    <form v-if="status === 'draft' && !created" ref="form" novalidate @submit.prevent="saveDraft">
      <div class="section-heading"><h2>{{ orderActions[payload.data.action] }} 초안</h2><p>{{ payload.data.action === 'cancel' ? '취소하면 주문 상태만 바뀝니다. 보유 수량과 가격을 차감하지 않습니다.' : '주문량은 현재 보유량과 별개입니다. 임시 저장한 뒤 내용을 확인하고 확정하세요.' }}</p></div>
      <div v-if="payload.data.action === 'place'" class="field"><label for="order-material">주문 재료</label><select id="order-material" :value="payload.data.material_id ?? ''" :disabled="!available || locked || checking" :aria-invalid="!!issue('material_id')" @change="selectMaterial"><option value="">재료 선택</option><option v-if="selectedId && !materials.some(item => item.id === selectedId)" :value="selectedId">참조한 재료 확인 필요</option><option v-for="item in choices" :key="item.id" :value="item.id">{{ item.name }}{{ item.archived ? ' (보관됨)' : '' }} · {{ item.id.slice(0, 8) }}</option></select><p class="field-error">{{ issue('material_id') }}</p></div>
      <div v-else class="order-reference"><p><strong>{{ order?.item.material_name ?? '대상 주문 확인 중' }}</strong></p><p v-if="order">현재 주문 {{ displayDecimal(order.head.input_quantity) }}{{ order.head.input_unit }} · {{ orderStatuses[order.item.status] }}</p><NuxtLink :to="`/orders/${payload.data.order_id}`">대상 주문과 이력 보기</NuxtLink><p class="field-hint">같은 재료의 주문만 수정할 수 있습니다.</p></div>
      <button v-if="selectedId || targetId" id="order-current-reference" class="text-button" type="button" :disabled="!available || locked || checking" @click="applyCurrent">{{ payload.data.action === 'place' ? '현재 재료 정보 적용' : '현재 주문 정보 적용' }}</button>
      <p class="field-error">{{ issue('material_version_id') || issue('expected_order_version_id') || issue('order_id') }}</p>
      <p v-if="referenceFailure" class="notice warning" role="alert">{{ referenceFailure }} <button class="text-button" type="button" :disabled="!available" @click="loadReferences">참조 다시 불러오기</button></p>
      <template v-if="payload.data.action !== 'cancel'">
        <div class="business-price-fields order-fields"><div class="field"><label for="order-quantity">주문 수량</label><input id="order-quantity" v-model="payload.data.quantity" inputmode="decimal" :readonly="locked" :disabled="!available || checking" :aria-invalid="!!issue('quantity')" aria-describedby="order-quantity-error" @input="errors.quantity = undefined"><p id="order-quantity-error" class="field-error">{{ issue('quantity') }}</p></div><div class="field"><label for="order-unit">주문 단위</label><input id="order-unit" v-model="payload.data.unit" :readonly="locked" :disabled="!available || checking" :aria-invalid="!!issue('unit')" aria-describedby="order-unit-error" @input="errors.unit = undefined"><p id="order-unit-error" class="field-error">{{ issue('unit') }}</p></div></div>
        <div class="field"><label for="order-supplier">거래처명 (선택)</label><input id="order-supplier" v-model="payload.data.supplier" :readonly="locked" :disabled="!available || checking" :aria-invalid="!!issue('supplier')" aria-describedby="order-supplier-error" @input="errors.supplier = undefined"><p id="order-supplier-error" class="field-error">{{ issue('supplier') }}</p></div>
        <div class="field"><label for="order-expected_date">입고 예정일 (선택)</label><input id="order-expected_date" v-model="payload.data.expected_date" placeholder="YYYY-MM-DD" :readonly="locked" :disabled="!available || checking" :aria-invalid="!!issue('expected_date')" aria-describedby="order-date-hint order-date-error" @input="errors.expected_date = undefined"><p id="order-date-hint" class="field-hint">예정일을 적는 칸입니다. 실제 입고를 기록하지 않습니다.</p><p id="order-date-error" class="field-error">{{ issue('expected_date') }}</p></div>
        <div class="field"><label for="order-estimated_amount">예상 금액 (원, 선택)</label><input id="order-estimated_amount" v-model="payload.data.estimated_amount" inputmode="decimal" placeholder="미입력과 0원은 구분합니다" :readonly="locked" :disabled="!available || checking" :aria-invalid="!!issue('estimated_amount')" aria-describedby="order-amount-error" @input="errors.estimated_amount = undefined"><p id="order-amount-error" class="field-error">{{ issue('estimated_amount') }}</p></div>
      </template>
      <div class="field"><label for="order-note">메모 (선택)</label><textarea id="order-note" v-model="payload.data.note" rows="3" :readonly="locked" :disabled="!available || checking" :aria-invalid="!!issue('note')" aria-describedby="order-note-error" @input="errors.note = undefined"/><p id="order-note-error" class="field-error">{{ issue('note') }}</p></div>
      <p v-if="dirty" class="draft-note" role="status">화면에서 바꾼 내용이 아직 임시 저장되지 않았습니다.</p>
      <div class="button-row"><button class="primary-button" type="submit" :disabled="!available || locked || checking">{{ initial.recordId ? '수정 내용 임시 저장' : '초안 임시 저장' }}</button><button v-if="initial.recordId" class="secondary-button" type="button" :disabled="!available || locked || checking || dirty" @click="previewDraft">확정 전 미리보기</button></div>
      <p class="field-hint">미완성 입력도 임시 저장할 수 있습니다. 변경한 내용을 저장하면 미리보기가 가능합니다.</p>
    </form>
    <section v-if="preview" ref="previewPanel" class="business-preview" tabindex="-1" aria-labelledby="order-preview-title"><h2 id="order-preview-title">{{ orderActions[payload.data.action] }} 미리보기</h2><ul v-if="preview.issues.length" class="business-issues"><li v-for="item in preview.issues" :key="`${item.field}-${item.code}`">{{ item.message }}</li></ul>
      <template v-if="preview.effect?.kind === 'purchase_order'">
        <dl class="business-price-comparison"><div><dt>주문 상태</dt><dd>{{ preview.effect.before ? `${orderStatuses[preview.effect.before.status]} → ` : '' }}{{ orderStatuses[preview.effect.after.status] }}</dd></div><div><dt>주문량</dt><dd class="preview-result">{{ preview.effect.before ? `${displayDecimal(preview.effect.before.input_quantity)}${preview.effect.before.input_unit} → ` : '' }}{{ displayDecimal(preview.effect.after.input_quantity) }}{{ preview.effect.after.input_unit }}</dd></div><div><dt>기준 단위 주문량</dt><dd>{{ displayDecimal(preview.effect.after.quantity) }}{{ preview.effect.after.unit }}</dd></div><div><dt>거래처명</dt><dd>{{ preview.effect.after.supplier ?? '미입력' }}</dd></div><div><dt>입고 예정일</dt><dd>{{ preview.effect.after.expected_date ?? '미입력' }}</dd></div><div><dt>예상 금액</dt><dd>{{ preview.effect.after.estimated_amount === null ? '미입력' : `${displayDecimal(preview.effect.after.estimated_amount)}원` }}</dd></div></dl>
        <p>주문 기록과 진행 상태를 반영합니다. 보유 수량과 사전 가격은 바뀌지 않습니다.</p>
      </template><button class="primary-button" type="button" :disabled="!available || busy || !!pending || dirty || !preview.can_confirm" @click="send('confirm')">이 내용으로 확정</button>
    </section>
    <section v-if="initial.recordId && status === 'draft'" class="archive-section" aria-label="초안 보관"><p>초안을 보관해도 확정된 주문은 바뀌지 않습니다.</p><button v-if="!archiveReview" class="text-button" type="button" :disabled="!available || locked" @click="archiveReview = true">초안 보관</button><div v-else class="notice warning"><p>화면의 미저장 입력은 함께 저장되지 않습니다.</p><div class="button-row"><button class="secondary-button" type="button" :disabled="!available || locked" @click="send('archive')">초안 보관 확정</button><button class="text-button" type="button" :disabled="locked" @click="archiveReview = false">취소</button></div></div></section>
  </section>
</template>
