import type { OrderDetail, OrderPayload } from '../types/purchase-order.ts'
import { restoreBusinessPending, validBusinessPayload } from './business.ts'

export const orderStatuses = { open: '미입고', cancelled: '취소', received: '입고 완료' }
export const orderActions = { place: '주문 작성', revise: '주문 수정', cancel: '주문 취소', receive: '입고 확정' }
export const blankOrderDraft = (): OrderPayload => ({ kind: 'purchase_order', version: 1, data: { action: 'place', material_id: null, material_version_id: null, quantity: '', unit: 'kg', supplier: '', expected_date: '', estimated_amount: '', note: '' } })
export function orderActionDraft(order: OrderDetail, action: 'revise' | 'cancel'): OrderPayload {
  const target = { order_id: order.item.id, expected_order_version_id: order.head.id }
  if (action === 'cancel') return { kind: 'purchase_order', version: 1, data: { action, ...target, note: '' } }
  return { kind: 'purchase_order', version: 1, data: { action, ...target, material_version_id: order.head.material_version_id, quantity: order.head.input_quantity, unit: order.head.input_unit, supplier: order.head.supplier ?? '', expected_date: order.head.expected_date ?? '', estimated_amount: order.head.estimated_amount ?? '', note: order.head.note } }
}

// A new head starts a fresh editor. An unresolved request or local edit keeps its
// original identity until the user completes/reviews it, even if another tab acts.
export function orderDraftIdentity(payload: OrderPayload, entries: Array<[string, string]>): string {
  if (payload.data.action === 'place') return 'new.order.place'
  const data = payload.data
  const base = `new.order.${data.action}.${data.order_id}.`
  const prefix = `sunchang.business.${base}`
  for (const suffix of ['.pending.v1', '.draft.v1']) {
    for (const [key, raw] of entries) {
      if (!key.startsWith(prefix) || !key.endsWith(suffix)) continue
      try {
        const pending = suffix === '.pending.v1' ? restoreBusinessPending(raw) : null
        const stored = pending && pending.action === 'create' ? pending.body.payload : suffix === '.draft.v1' ? JSON.parse(raw)?.payload : null
        if (validBusinessPayload(stored) && stored.kind === 'purchase_order' && stored.data.action === data.action && stored.data.order_id === data.order_id) return key.slice('sunchang.business.'.length, -suffix.length)
      } catch { /* Ignore malformed local state. */ }
    }
  }
  return `${base}${data.expected_order_version_id}`
}
