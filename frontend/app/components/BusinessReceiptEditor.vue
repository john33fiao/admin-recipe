<script setup lang="ts">
import type { BusinessDetail } from '~/types/business'
import type { BusinessEditorInitial } from '~/composables/useBusinessDraft'
import type { Material } from '~/types/material'
import type { OrderDetail, OrderList, OrderSummary } from '~/types/purchase-order'
import type { ReceiptPayload } from '~/types/receipt'
import { apiRequest } from '~/utils/api'
import { businessInputText, validateBusinessDraft } from '~/utils/business'
import { displayDecimal } from '~/utils/catalog'
import { orderStatuses } from '~/utils/purchase-order'
import { blankReceiptSelection } from '~/utils/receipt'
const props = defineProps<{ identity: string; initial: BusinessEditorInitial<ReceiptPayload>; refresh: () => Promise<BusinessDetail | null> }>()
const { payload, status, issues, preview, busy, failure, notice, storageFailure, created, pending, dirty, locked, canRetry, canReview, send, showPreview, review, resetNew, synchronizeTerminal } = useBusinessDraft(props.identity, props.initial, props.refresh)
watch(() => props.initial, synchronizeTerminal, { immediate: true })
const connection = useConnection()
const available = computed(() => connection.value === 'connected')
const materials = ref<Material[]>([])
const orders = ref<OrderSummary[]>([])
const selectedOrder = ref<OrderDetail | null>(null)
const referenceFailure = ref('')
const checking = ref(false)
const errors = ref<Record<string, string>>({})
const form = ref<HTMLFormElement | null>(null)
const previewPanel = ref<HTMLElement | null>(null)
const archiveReview = ref(false)
const materialId = computed(() => payload.value.data.source.kind === 'direct' ? payload.value.data.source.material_id : selectedOrder.value?.item.id === payload.value.data.source.order_id ? selectedOrder.value.item.material_id : null)
const orderId = computed(() => payload.value.data.source.kind === 'order' ? payload.value.data.source.order_id : null)
const materialChoices = computed(() => materials.value.filter(item => !item.archived || item.id === materialId.value))
let active = true
let generation = 0
onScopeDispose(() => { active = false; generation += 1 })
async function loadReferences() {
  const current = ++generation
  if (!available.value) return
  const target = orderId.value
  try {
    const [allMaterials, openOrders, order] = await Promise.all([apiRequest<Material[]>('/api/materials?archive=all'), apiRequest<OrderList>('/api/orders'), target ? apiRequest<OrderDetail>(`/api/orders/${encodeURIComponent(target)}`) : null])
    if (!active || current !== generation) return
    materials.value = allMaterials
    orders.value = openOrders.items
    selectedOrder.value = order
    referenceFailure.value = ''
  } catch { if (active && current === generation) referenceFailure.value = '입고 대상 자료를 불러오지 못했습니다. 다시 확인해 주세요.' }
}
watch([available, orderId], () => { void loadReferences() }, { immediate: true })
function changeSource(event: Event) {
  if (locked.value) return
  payload.value.data.source = (event.target as HTMLSelectElement).value === 'order' ? { kind: 'order', order_id: null, expected_order_version_id: null, material_version_id: null } : { kind: 'direct', material_id: null, material_version_id: null }
}
function selectMaterial(event: Event) {
  if (locked.value || payload.value.data.source.kind !== 'direct') return
  const item = materials.value.find(value => value.id === (event.target as HTMLSelectElement).value)
  payload.value.data.source = { kind: 'direct', material_id: item?.id ?? null, material_version_id: item?.version_id ?? null }
  if (item) payload.value.data.unit = item.unit
}
function selectOrder(event: Event) {
  if (locked.value || payload.value.data.source.kind !== 'order') return
  const item = orders.value.find(value => value.id === (event.target as HTMLSelectElement).value)
  payload.value.data.source = { kind: 'order', order_id: item?.id ?? null, expected_order_version_id: item?.head_id ?? null, material_version_id: item?.material_version_id ?? null }
  if (item) payload.value.data.unit = item.input_unit
}
function changeStockEffect(event: Event) {
  if (locked.value) return
  const kind = (event.target as HTMLSelectElement).value
  if (kind === payload.value.data.stock_effect.kind) return
  if (kind === 'reconciled') {
    const previous = payload.value.data.stock_effect
    payload.value.data.stock_effect = { kind: 'reconciled', selections: [{ ...blankReceiptSelection(), included_quantity: previous.kind === 'needs_reconciliation' ? previous.included_quantity : '' }] }
  } else payload.value.data.stock_effect = kind === 'needs_reconciliation' ? { kind: 'needs_reconciliation', basis_id: null, included_quantity: '' } : { kind: 'unreflected' }
}
async function applyCurrent() {
  if (locked.value || checking.value || !available.value) return
  const source = { ...payload.value.data.source }
  const fingerprint = JSON.stringify(source)
  checking.value = true
  try {
    let order: OrderDetail | null = null
    let material: Material
    if (source.kind === 'direct') {
      if (!source.material_id) return
      material = await apiRequest<Material>(`/api/materials/${encodeURIComponent(source.material_id)}`)
    } else {
      if (!source.order_id) return
      order = await apiRequest<OrderDetail>(`/api/orders/${encodeURIComponent(source.order_id)}`)
      material = await apiRequest<Material>(`/api/materials/${encodeURIComponent(order.item.material_id)}`)
    }
    if (!active || locked.value || JSON.stringify(payload.value.data.source) !== fingerprint) return
    materials.value = [material, ...materials.value.filter(item => item.id !== material.id)]
    if (order) {
      selectedOrder.value = order
      if (order.item.status !== 'open') { referenceFailure.value = '이미 닫힌 주문에는 새 입고를 확정할 수 없습니다. 작성 중인 입력은 초안으로 보관할 수 있습니다.'; return }
      if (payload.value.data.source.kind === 'order') payload.value.data.source.expected_order_version_id = order.head.id
    }
    payload.value.data.source.material_version_id = material.version_id
    referenceFailure.value = ''
  } catch { if (active) referenceFailure.value = '최신 입고 대상을 확인하지 못했습니다. 다시 시도해 주세요.' }
  finally { checking.value = false }
}
function issue(field: string) { return errors.value[field] ?? issues.value.find(value => value.field === field || value.field === field.split('.').pop())?.message }
async function focusIssue(client = false) {
  await nextTick()
  const field = client ? Object.keys(errors.value)[0] : issues.value[0]?.field
  const simple = field?.split('.').pop()
  const selection = field?.match(/selections\.(\d+)(?:\.(.+))?/)
  const id = selection ? `receipt-inclusion-${selection[1]}-${selection[2] === 'included_quantity' ? 'included_quantity' : selection[2] === 'observation_version_id' ? 'observation_version_id' : 'source'}` : simple === 'material_id' ? 'receipt-material' : simple === 'order_id' ? 'receipt-order' : simple?.includes('version') ? 'receipt-current-reference' : field?.startsWith('stock_effect') || field === 'needs_reconciliation' ? 'receipt-stock-effect' : simple ? `receipt-${simple}` : ''
  const target = id ? form.value?.querySelector<HTMLElement>(`#${id}`) : null
  if (target) target.focus(); else previewPanel.value?.focus()
}
async function saveDraft() { errors.value = validateBusinessDraft(payload.value); if (Object.keys(errors.value).length) { await focusIssue(true); return }; await send(props.initial.recordId ? 'revise' : 'create') }
async function previewDraft() { if (await showPreview()) { await nextTick(); previewPanel.value?.focus() } else await focusIssue() }
</script>
<template>
  <section class="business-editor" aria-label="실제 입고 초안 입력" :aria-busy="busy">
    <p v-if="storageFailure" class="notice warning" role="alert">{{ storageFailure }}</p>
    <div v-if="failure" class="notice warning" role="alert"><p>{{ failure }}</p><button v-if="canReview" class="text-button" type="button" :disabled="busy || !available" @click="review">최신 기록 확인 후 입력 검토</button></div>
    <p v-if="pending && !failure" class="notice warning" role="status">{{ busy ? '처리 결과를 확인하고 있습니다.' : '완료를 확인하지 못한 전송 요청이 있습니다. 같은 요청으로 재시도해 주세요.' }}</p>
    <button v-if="canRetry && !busy" class="secondary-button" type="button" :disabled="!available" @click="send(pending!.action)">같은 업무 요청 재시도</button>
    <p v-if="notice" class="notice success" role="status">{{ notice }}</p>
    <NuxtLink v-if="created" class="business-saved-link" :to="`/business/${created.id}`">저장한 초안 열기</NuxtLink>
    <button v-if="created" class="text-button" type="button" :disabled="busy" @click="resetNew">새 입고 초안 작성</button>
    <section v-if="status !== 'draft' && dirty" class="notice warning" aria-labelledby="receipt-unsaved-title"><h2 id="receipt-unsaved-title">이 화면에 남은 미저장 입력</h2><p>다른 화면에서 확정하거나 보관해 이 입력은 반영되지 않았습니다.</p><p>{{ businessInputText(payload) }}</p><p>{{ payload.data.source.kind === 'direct' ? '직접 입고' : '주문 연결 입고' }} · {{ payload.data.stock_effect.kind === 'unreflected' ? '아직 미반영' : '포함 여부 확인 필요' }}</p><p class="preserve-lines">{{ payload.data.note }}</p></section>
    <form v-if="status === 'draft' && !created" ref="form" novalidate @submit.prevent="saveDraft">
      <div class="section-heading"><h2>실제 입고 초안</h2><p>받은 수량과 실제 구입금액을 확인해 입력하세요. 주문의 예상 수량·금액과 구분합니다.</p></div>
      <div class="field"><label for="receipt-source">입고 방식</label><select id="receipt-source" :value="payload.data.source.kind" :disabled="!available || locked || checking" @change="changeSource"><option value="direct">직접 입고</option><option value="order">주문 연결 입고</option></select></div>
      <div v-if="payload.data.source.kind === 'direct'" class="field"><label for="receipt-material">입고 재료</label><select id="receipt-material" :value="payload.data.source.material_id ?? ''" :disabled="!available || locked || checking" :aria-invalid="!!issue('source.material_id')" aria-describedby="receipt-material-error" @change="selectMaterial"><option value="">재료 선택</option><option v-if="materialId && !materials.some(item => item.id === materialId)" :value="materialId">참조한 재료 확인 필요</option><option v-for="item in materialChoices" :key="item.id" :value="item.id">{{ item.name }}{{ item.archived ? ' (보관됨)' : '' }} · {{ item.id.slice(0, 8) }}</option></select><p id="receipt-material-error" class="field-error">{{ issue('source.material_id') }}</p></div>
      <div v-else class="field"><label for="receipt-order">연결할 주문</label><select id="receipt-order" :value="payload.data.source.order_id ?? ''" :disabled="!available || locked || checking" :aria-invalid="!!issue('source.order_id')" aria-describedby="receipt-order-error" @change="selectOrder"><option value="">미입고 주문 선택</option><option v-if="orderId && !orders.some(item => item.id === orderId)" :value="orderId">{{ selectedOrder ? `${selectedOrder.item.material_name} · ${orderStatuses[selectedOrder.item.status]}` : '참조한 주문 확인 필요' }}</option><option v-for="item in orders" :key="item.id" :value="item.id">{{ item.material_name }} · {{ displayDecimal(item.input_quantity) }}{{ item.input_unit }} · {{ item.id.slice(0, 8) }}</option></select><p id="receipt-order-error" class="field-error">{{ issue('source.order_id') }}</p><p v-if="selectedOrder" class="field-hint">현재 주문량 {{ displayDecimal(selectedOrder.head.input_quantity) }}{{ selectedOrder.head.input_unit }} · {{ orderStatuses[selectedOrder.item.status] }}. 받은 수량은 아래에 직접 입력하세요.</p><NuxtLink v-if="orderId" :to="`/orders/${orderId}`">연결한 주문 보기</NuxtLink></div>
      <button v-if="materialId || orderId" id="receipt-current-reference" class="text-button" type="button" :disabled="!available || locked || checking" @click="applyCurrent">현재 입고 대상 정보 적용</button>
      <p class="field-error">{{ issue('source.material_version_id') || issue('source.expected_order_version_id') }}</p>
      <p v-if="referenceFailure" class="notice warning" role="alert">{{ referenceFailure }} <button class="text-button" type="button" :disabled="!available" @click="loadReferences">대상 다시 불러오기</button></p>
      <div class="business-price-fields order-fields"><div class="field"><label for="receipt-quantity">실제 입고 수량</label><input id="receipt-quantity" v-model="payload.data.quantity" inputmode="decimal" :readonly="locked" :disabled="!available || checking" :aria-invalid="!!issue('quantity')" aria-describedby="receipt-quantity-error" @input="delete errors.quantity"><p id="receipt-quantity-error" class="field-error">{{ issue('quantity') }}</p></div><div class="field"><label for="receipt-unit">입고 단위</label><input id="receipt-unit" v-model="payload.data.unit" :readonly="locked" :disabled="!available || checking" :aria-invalid="!!issue('unit')" aria-describedby="receipt-unit-error" @input="delete errors.unit"><p id="receipt-unit-error" class="field-error">{{ issue('unit') }}</p></div></div>
      <div class="field"><label for="receipt-amount">실제 구입금액 (원)</label><input id="receipt-amount" v-model="payload.data.amount" inputmode="decimal" placeholder="실제 0원이면 0을 입력하세요" :readonly="locked" :disabled="!available || checking" :aria-invalid="!!issue('amount')" aria-describedby="receipt-amount-hint receipt-amount-error" @input="delete errors.amount"><p id="receipt-amount-hint" class="field-hint">미입력 금액으로는 확정할 수 없습니다. 예상 금액이나 사전 가격을 자동 적용하지 않습니다.</p><p id="receipt-amount-error" class="field-error">{{ issue('amount') }}</p></div>
      <div class="field"><label for="receipt-receipt_date">실제 입고일</label><input id="receipt-receipt_date" v-model="payload.data.receipt_date" placeholder="YYYY-MM-DD" :readonly="locked" :disabled="!available || checking" :aria-invalid="!!issue('receipt_date')" aria-describedby="receipt-date-hint receipt-date-error" @input="delete errors.receipt_date"><p id="receipt-date-hint" class="field-hint">받은 날짜를 입력하세요. 미래 입고 예정은 <NuxtLink to="/orders">발주 내역</NuxtLink>에서 관리합니다.</p><p id="receipt-date-error" class="field-error">{{ issue('receipt_date') }}</p></div>
      <div class="field"><label for="receipt-stock-effect">재고 반영 구분</label><select id="receipt-stock-effect" :value="payload.data.stock_effect.kind" :disabled="!available || locked || checking" aria-describedby="receipt-stock-hint" @change="changeStockEffect"><option value="unreflected">아직 재고에 반영되지 않은 입고</option><option value="needs_reconciliation">이미 포함되었거나 포함 여부 확인 필요</option><option value="reconciled">실사에 포함된 수량을 확인하고 보정</option></select><p v-if="payload.data.stock_effect.kind === 'unreflected'" id="receipt-stock-hint" class="field-hint">이번에 받은 수량과 금액을 정산 기준에 한 번 더합니다. 이미 실사 등에 포함한 수량은 포함 확인을 선택하세요.</p><div v-else-if="payload.data.stock_effect.kind === 'needs_reconciliation'" id="receipt-stock-hint" class="notice warning"><p>수량을 중복으로 더하지 않도록 이 입력은 초안으로 보관합니다. 확정하려면 위에서 실사에 포함된 수량을 확인하는 구분으로 전환하고 근거를 선택하세요.</p><p v-if="payload.data.stock_effect.included_quantity">보관된 포함량 메모: {{ payload.data.stock_effect.included_quantity }} (검증 전)</p><p v-if="payload.data.stock_effect.basis_id" class="material-id">보관된 근거 {{ payload.data.stock_effect.basis_id }}</p></div><p v-else id="receipt-stock-hint" class="field-hint">선택한 실사 근거와 현재에 반영할 차이를 미리보기한 뒤 확정합니다.</p><p class="field-error">{{ issue('stock_effect') }}</p></div>
      <ReceiptInclusionEditor v-if="payload.data.stock_effect.kind === 'reconciled'" v-model="payload.data.stock_effect.selections" :material-id="materialId" :locked="locked || checking" :available="available" :issue="issue" />
      <div class="field"><label for="receipt-note">메모 (선택)</label><textarea id="receipt-note" v-model="payload.data.note" rows="3" :readonly="locked" :disabled="!available || checking" :aria-invalid="!!issue('note')" aria-describedby="receipt-note-error" @input="delete errors.note"/><p id="receipt-note-error" class="field-error">{{ issue('note') }}</p></div>
      <p v-if="dirty" class="draft-note" role="status">화면에서 바꾼 내용이 아직 임시 저장되지 않았습니다.</p>
      <div class="button-row"><button class="primary-button" type="submit" :disabled="!available || locked || checking">{{ initial.recordId ? '수정 내용 임시 저장' : '초안 임시 저장' }}</button><button v-if="initial.recordId" class="secondary-button" type="button" :disabled="!available || locked || checking || dirty" @click="previewDraft">확정 전 미리보기</button></div>
      <p class="field-hint">미완성 입력도 임시 저장할 수 있습니다. 변경한 내용을 저장하면 미리보기가 가능합니다.</p>
    </form>
    <section v-if="preview" ref="previewPanel" class="business-preview receipt-preview" tabindex="-1" aria-labelledby="receipt-preview-title"><h2 id="receipt-preview-title">실제 입고 미리보기</h2><ul v-if="preview.issues.length" class="business-issues"><li v-for="item in preview.issues" :key="`${item.field}-${item.code}`">{{ item.message }}</li></ul>
      <template v-if="preview.effect?.kind === 'receipt'"><dl class="business-price-comparison"><div><dt>실제 입고</dt><dd class="preview-result">{{ displayDecimal(preview.effect.input_quantity) }}{{ preview.effect.input_unit }} · {{ displayDecimal(preview.effect.amount) }}원</dd></div><div><dt>실제 입고일</dt><dd>{{ preview.effect.receipt_date }}</dd></div><div><dt>마지막 확인 수량</dt><dd>{{ preview.effect.observation ? `${displayDecimal(preview.effect.observation.quantity)}${preview.effect.unit}` : '아직 확인한 수량 없음' }}</dd></div><div><dt>이전 정산 기준</dt><dd>{{ preview.effect.before ? `${displayDecimal(preview.effect.before.quantity)}${preview.effect.unit} · ${preview.effect.before.value === null ? '가치 미확인' : `${displayDecimal(preview.effect.before.value)}원`}` : '아직 정산 기준 없음' }}</dd></div><div><dt>입고 후 정산 기준</dt><dd>{{ displayDecimal(preview.effect.after.quantity) }}{{ preview.effect.unit }} · {{ preview.effect.after.value === null ? '가치 미확인' : `${displayDecimal(preview.effect.after.value)}원` }}</dd></div></dl>
        <section v-if="preview.effect.order_after" class="receipt-order-comparison" aria-labelledby="receipt-order-preview-title"><h3 id="receipt-order-preview-title">주문 입고 완료</h3><dl class="business-price-comparison"><div><dt>최초 주문량</dt><dd>{{ displayDecimal(preview.effect.order_after.initial_ordered_quantity) }}{{ preview.effect.order_after.unit }}</dd></div><div><dt>확정 주문량</dt><dd>{{ displayDecimal(preview.effect.order_after.ordered_quantity) }}{{ preview.effect.order_after.unit }}</dd></div><div><dt>실제 입고량</dt><dd>{{ displayDecimal(preview.effect.order_after.received_quantity) }}{{ preview.effect.order_after.unit }}</dd></div><div><dt>주문 대비 차이</dt><dd>{{ displayDecimal(preview.effect.order_after.difference) }}{{ preview.effect.order_after.unit }}</dd></div></dl><p>이 입고로 주문을 종료합니다. 이전 주문량과 실측량은 각각 이력에 남습니다.</p></section>
        <p>확인한 입고를 정산 기준에 반영합니다. 입고를 기록해도 새로 계량한 현재 수량으로 바꾸지 않습니다.</p></template>
      <ReceiptReconciliationResult v-if="preview.effect?.kind === 'reconciled_receipt'" :effect="preview.effect.receipt" />
      <button class="primary-button" type="button" :disabled="!available || busy || !!pending || dirty || !preview.can_confirm" @click="send('confirm')">이 내용으로 확정</button>
    </section>
    <section v-if="initial.recordId && status === 'draft'" class="archive-section" aria-label="초안 보관"><p>초안을 보관해도 이미 확정된 입고와 정산 기준은 바뀌지 않습니다.</p><button v-if="!archiveReview" class="text-button" type="button" :disabled="!available || locked" @click="archiveReview = true">초안 보관</button><div v-else class="notice warning"><p>화면의 미저장 입력은 함께 저장되지 않습니다.</p><div class="button-row"><button class="secondary-button" type="button" :disabled="!available || locked" @click="send('archive')">초안 보관 확정</button><button class="text-button" type="button" :disabled="locked" @click="archiveReview = false">취소</button></div></div></section>
  </section>
</template>
