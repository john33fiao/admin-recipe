<script setup lang="ts">
import type { ReceiptPayload } from '~/types/receipt'
import type { CorrectionInclusionCandidate } from '~/types/correction'
import type { OrderList } from '~/types/purchase-order'
import { apiRequest } from '~/utils/api'
const data = defineModel<ReceiptPayload['data']>({ required: true })
const props = defineProps<{ prefix: string; field: string; disabled: boolean; proposed?: CorrectionInclusionCandidate[] }>()
const orders = ref<OrderList['items']>([])
let active = true
onScopeDispose(() => { active = false })
onMounted(async () => { try { const result = await apiRequest<OrderList>('/api/orders?status=all'); if (active) orders.value = result.items } catch { /* The stored order reference remains visible. */ } })
function source(event: Event) { if (props.disabled) return; data.value.source = (event.target as HTMLSelectElement).value === 'order' ? { kind: 'order', order_id: null, expected_order_version_id: null, material_version_id: null } : { kind: 'direct', material_id: null, material_version_id: null } }
function order(event: Event) { if (props.disabled || data.value.source.kind !== 'order') return; const item = orders.value.find(item => item.id === (event.target as HTMLSelectElement).value); data.value.source.order_id = item?.id ?? null; data.value.source.expected_order_version_id = item?.head_id ?? null; data.value.source.material_version_id = item?.material_version_id ?? null }
function stock(event: Event) { if (props.disabled) return; const kind = (event.target as HTMLSelectElement).value; data.value.stock_effect = kind === 'reconciled' ? { kind, selections: [] } : kind === 'needs_reconciliation' ? { kind, basis_id: null, included_quantity: '' } : { kind: 'unreflected' } }
</script>
<template>
  <div class="correction-receipt-fields"><div class="field"><label :for="`${prefix}-source`">입고 연결</label><select :id="`${prefix}-source`" :value="data.source.kind" :disabled="disabled" @change="source"><option value="direct">직접 입고</option><option value="order">주문 연결</option></select></div>
    <CorrectionMaterialInput v-if="data.source.kind === 'direct'" :id="`${prefix}-material`" v-model="data.source" label="입고 품목" :field="`${field}.source.material_id`" :disabled="disabled"/>
    <div v-else class="field"><label :for="`${prefix}-order`">연결 주문</label><select :id="`${prefix}-order`" :value="data.source.order_id ?? ''" :data-field="`${field}.source.order_id`" :disabled="disabled" @change="order"><option value="">주문 선택</option><option v-if="data.source.order_id && !orders.some(item => data.source.kind === 'order' && item.id === data.source.order_id)" :value="data.source.order_id">저장한 주문 · {{ data.source.order_id.slice(0, 8) }}</option><option v-for="item in orders" :key="item.id" :value="item.id">{{ item.material_name }} · {{ item.id.slice(0, 8) }}</option></select><p class="field-hint">입고 완료 여부와 주문 연결은 정정 미리보기에서 함께 확인합니다.</p></div>
    <div class="quantity-fields"><div class="field"><label :for="`${prefix}-quantity`">실제 입고 수량</label><input :id="`${prefix}-quantity`" v-model="data.quantity" :data-field="`${field}.quantity`" inputmode="decimal" :disabled="disabled"></div><div class="field"><label :for="`${prefix}-unit`">입고 단위</label><input :id="`${prefix}-unit`" v-model="data.unit" :data-field="`${field}.unit`" :disabled="disabled"></div><div class="field"><label :for="`${prefix}-amount`">실제 입고 금액 (원)</label><input :id="`${prefix}-amount`" v-model="data.amount" :data-field="`${field}.amount`" inputmode="decimal" :disabled="disabled"></div></div>
    <div class="field"><label :for="`${prefix}-date`">입고일</label><input :id="`${prefix}-date`" v-model="data.receipt_date" :data-field="`${field}.receipt_date`" type="date" :disabled="disabled"></div>
    <div class="field"><label :for="`${prefix}-stock`">실사 포함 여부</label><select :id="`${prefix}-stock`" :data-field="`${field}.stock_effect`" :value="data.stock_effect.kind" :disabled="disabled" @change="stock"><option value="unreflected">실사에 포함되지 않은 입고</option><option value="reconciled">실사에 포함된 수량을 명시</option><option v-if="data.stock_effect.kind === 'needs_reconciliation'" value="needs_reconciliation">이전 입력 · 포함 여부 확인 필요</option></select></div>
    <p v-if="data.stock_effect.kind === 'needs_reconciliation'" class="notice warning">이전 확인 필요 입력을 보존했습니다. 확정하려면 포함 여부를 명시적으로 선택해 주세요.</p>
    <CorrectionInclusions v-if="data.stock_effect.kind === 'reconciled'" v-model="data.stock_effect.selections" :prefix="`${prefix}-included`" :field="`${field}.stock_effect.selections`" :disabled="disabled" :material-id="data.source.kind === 'direct' ? data.source.material_id : orders.find(item => data.source.kind === 'order' && data.source.kind === 'order' && item.id === data.source.order_id)?.material_id" :proposed="proposed"/>
    <div class="field"><label :for="`${prefix}-note`">입고 메모</label><textarea :id="`${prefix}-note`" v-model="data.note" :data-field="`${field}.note`" rows="2" :disabled="disabled"/></div>
  </div>
</template>
