import type { BusinessPayload, BusinessPending, BusinessPreview, CatalogPricePayload } from '../types/business.ts'
import type { CatalogState } from '../types/material.ts'
import { validProductionShape, validateProductionDraft } from './production.ts'
import { validSaleShape, validateSaleDraft } from './sale.ts'
import { validReceiptShape, validateReceiptDraft } from './receipt.ts'

export const businessStatuses = { draft: '임시 저장', confirmed: '확정', archived: '보관' }
export const blankPriceDraft = (): CatalogPricePayload => ({ kind: 'catalog_price', version: 1, data: { material_id: null, material_version_id: null, quantity: '', unit: 'g', amount: '', note: '' } })
const uuid = (value: unknown): value is string => typeof value === 'string' && /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(value)
const object = (value: unknown): value is Record<string, unknown> => !!value && typeof value === 'object' && !Array.isArray(value)
const exact = (value: Record<string, unknown>, keys: string[]) => Object.keys(value).length === keys.length && keys.every(key => Object.hasOwn(value, key))

export function validateBusinessDraft(payload: BusinessPayload): Record<string, string> {
  if (payload.kind === 'receipt') return validateReceiptDraft(payload)
  if (payload.kind === 'production') return validateProductionDraft(payload)
  if (payload.kind === 'sale') return validateSaleDraft(payload)
  const errors: Record<string, string> = {}
  const data = payload.data as unknown as Record<string, unknown>
  const ids = payload.kind === 'catalog_price' || payload.data.action === 'place' ? ['material_id', 'material_version_id'] : payload.data.action === 'revise' ? ['order_id', 'expected_order_version_id', 'material_version_id'] : ['order_id', 'expected_order_version_id']
  for (const field of ids) if (!((field === 'material_id' || field === 'material_version_id') && data[field] === null) && !uuid(data[field])) errors[field] = '대상과 현재 버전을 다시 확인해 주세요.'
  const fields: Array<[string, number]> = payload.kind === 'catalog_price' ? [['quantity', 80], ['amount', 80], ['unit', 20], ['note', 2000]] : payload.data.action === 'cancel' ? [['note', 2000]] : [['quantity', 80], ['estimated_amount', 80], ['unit', 20], ['supplier', 200], ['expected_date', 32], ['note', 2000]]
  for (const [field, limit] of fields) {
    const value = data[field]
    if (typeof value !== 'string' || [...value].length > limit || (field === 'note' ? /[\u0000-\u0009\u000b-\u001f\u007f-\u009f]/u : /[\u0000-\u001f\u007f-\u009f]/u).test(value)) errors[field] = `${limit}자 이내의 올바른 문자를 입력해 주세요.`
  }
  return errors
}

export function copyPayload<T extends BusinessPayload>(payload: T): T {
  const data = payload.kind === 'receipt' ? { ...payload.data, source: { ...payload.data.source }, stock_effect: { ...payload.data.stock_effect } } : payload.kind === 'production' ? { ...payload.data, inputs: payload.data.inputs.map(row => ({ ...row })), outputs: payload.data.outputs.map(row => ({ ...row })) } : { ...payload.data }
  return { kind: payload.kind, version: payload.version, data } as T
}
function sameValue(a: unknown, b: unknown): boolean {
  if (a === b) return true
  if (Array.isArray(a) || Array.isArray(b)) return Array.isArray(a) && Array.isArray(b) && a.length === b.length && a.every((value, index) => sameValue(value, b[index]))
  return object(a) && object(b) && Object.keys(a).length === Object.keys(b).length && Object.keys(a).every(key => Object.hasOwn(b, key) && sameValue(a[key], b[key]))
}
export function samePayload(a: BusinessPayload, b: BusinessPayload) {
  return a.kind === b.kind && a.version === b.version && sameValue(a.data, b.data)
}
export function sameBusinessTarget(a: BusinessPayload, b: BusinessPayload) {
  if (a.kind !== b.kind) return false
  if (a.kind !== 'purchase_order' || b.kind !== 'purchase_order') return true
  return a.data.action === b.data.action && (a.data.action === 'place' || (b.data.action !== 'place' && a.data.order_id === b.data.order_id))
}
export function businessLabel(payload: BusinessPayload) { return payload.kind === 'catalog_price' ? '사전 가격' : payload.kind === 'receipt' ? '실제 입고' : payload.kind === 'production' ? '실제 생산' : payload.kind === 'sale' ? '판매' : { place: '주문 작성', revise: '주문 수정', cancel: '주문 취소' }[payload.data.action] }
export function businessInputText(payload: BusinessPayload) {
  if (payload.kind === 'catalog_price') return `${payload.data.quantity || '수량 미입력'} ${payload.data.unit || '단위 미입력'} / ${payload.data.amount === '' ? '금액 미입력' : `${payload.data.amount}원`}`
  if (payload.kind === 'receipt') return `${payload.data.quantity || '수량 미입력'} ${payload.data.unit || '단위 미입력'} / 실제 금액 ${payload.data.amount.trim() === '' ? '미입력' : `${payload.data.amount}원`} / ${payload.data.receipt_date || '입고일 미입력'}`
  if (payload.kind === 'sale') return payload.data.count.trim() === '' ? '판매 개수 미입력' : `판매 ${payload.data.count}개`
  if (payload.kind === 'production') return `실제 투입 ${payload.data.inputs.length}행 · 실제 결과 ${payload.data.outputs.length}행`
  if (payload.data.action === 'cancel') return `주문 ${payload.data.order_id.slice(0, 8)} 취소`
  return `${payload.data.quantity || '수량 미입력'} ${payload.data.unit || '단위 미입력'} / 예상 금액 ${payload.data.estimated_amount.trim() === '' ? '미입력' : `${payload.data.estimated_amount}원`}`
}

export function businessPending(action: 'create', recordId: null, headId: null, state: CatalogState, payload: BusinessPayload, corrects: string | null, makeId: () => string): Extract<BusinessPending, { action: 'create' }>
export function businessPending(action: 'revise' | 'archive', recordId: string, headId: string, state: CatalogState, payload: BusinessPayload, corrects: string | null, makeId: () => string): BusinessPending
export function businessPending(action: 'create' | 'revise' | 'archive', recordId: string | null, headId: string | null, state: CatalogState, payload: BusinessPayload, corrects: string | null, makeId: () => string): BusinessPending {
  const meta = { request_id: makeId(), epoch: state.epoch, expected_revision: state.revision }
  if (action === 'create') return { action, record_id: null, body: { meta, payload: copyPayload(payload), corrects_confirmation_id: corrects } }
  if (!recordId || !headId) throw new Error('missing-business-head')
  if (action === 'revise') return { action, record_id: recordId, body: { meta, expected_head_id: headId, payload: copyPayload(payload) } }
  return { action, record_id: recordId, body: { meta, expected_head_id: headId } }
}

export function confirmPending(preview: BusinessPreview, makeId: () => string): BusinessPending {
  if (!preview.can_confirm || !preview.effect || preview.issues.length) throw new Error('invalid-preview')
  return { action: 'confirm', record_id: preview.record_id, body: { meta: { request_id: makeId(), epoch: preview.state.epoch, expected_revision: preview.state.revision }, expected_head_id: preview.head_id } }
}

export function validBusinessPayload(value: unknown): value is BusinessPayload {
  if (!object(value) || !exact(value, ['kind', 'version', 'data']) || value.version !== 1 || !object(value.data)) return false
  if (value.kind === 'catalog_price') { if (!exact(value.data, ['material_id', 'material_version_id', 'quantity', 'unit', 'amount', 'note'])) return false }
  else if (value.kind === 'purchase_order') {
    const common = ['material_version_id', 'quantity', 'unit', 'supplier', 'expected_date', 'estimated_amount', 'note']
    if (value.data.action === 'place') { if (!exact(value.data, ['action', 'material_id', ...common])) return false }
    else if (value.data.action === 'revise') { if (!exact(value.data, ['action', 'order_id', 'expected_order_version_id', ...common])) return false }
    else if (value.data.action === 'cancel') { if (!exact(value.data, ['action', 'order_id', 'expected_order_version_id', 'note'])) return false }
    else return false
  } else if (value.kind === 'receipt') { if (!validReceiptShape(value.data)) return false }
  else if (value.kind === 'production') { if (!validProductionShape(value.data)) return false }
  else if (value.kind === 'sale') { if (!validSaleShape(value.data)) return false }
  else return false
  return Object.keys(validateBusinessDraft(value as unknown as BusinessPayload)).length === 0
}

export function restoreBusinessPending(raw: string | null): BusinessPending | null {
  if (!raw) return null
  try {
    const value: unknown = JSON.parse(raw)
    if (!object(value) || !exact(value, ['action', 'record_id', 'body']) || !object(value.body)) return null
    const body = value.body
    if (!object(body.meta) || !exact(body.meta, ['request_id', 'epoch', 'expected_revision']) || !uuid(body.meta.request_id) || !uuid(body.meta.epoch) || typeof body.meta.expected_revision !== 'string' || !/^\d+$/.test(body.meta.expected_revision)) return null
    if (value.action === 'create') {
      if (value.record_id !== null || !exact(body, ['meta', 'payload', 'corrects_confirmation_id']) || !validBusinessPayload(body.payload) || !(body.corrects_confirmation_id === null || uuid(body.corrects_confirmation_id))) return null
      if (body.payload.kind !== 'catalog_price' && body.payload.kind !== 'sale' && body.corrects_confirmation_id !== null) return null
    } else {
      if (!uuid(value.record_id) || !uuid(body.expected_head_id)) return null
      if (value.action === 'revise') { if (!exact(body, ['meta', 'expected_head_id', 'payload']) || !validBusinessPayload(body.payload)) return null }
      else if (value.action === 'confirm' || value.action === 'archive') { if (!exact(body, ['meta', 'expected_head_id'])) return null }
      else return null
    }
    return value as unknown as BusinessPending
  } catch { return null }
}
