import { validPurchaseReturnShape } from './purchase-return.ts'
import type { CorrectionPayload, CorrectionReplacement, CorrectionPlanRef } from '../types/correction.ts'
import { validReceiptShape, validateReceiptDraft } from './receipt.ts'
import { validProductionShape, validateProductionDraft } from './production.ts'
import { validValuation, validWasteShape, validateStocktakeDraft } from './stocktake.ts'

const object = (value: unknown): value is Record<string, unknown> => !!value && typeof value === 'object' && !Array.isArray(value)
const exact = (value: Record<string, unknown>, keys: string[]) => keys.length === Object.keys(value).length && keys.every(key => Object.hasOwn(value, key))
const uuid = (value: unknown) => value === null || typeof value === 'string' && /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(value)
const text = (value: unknown, limit: number, multiline = false) => typeof value === 'string' && [...value].length <= limit && !(multiline ? /[\u0000-\u0009\u000b-\u001f\u007f-\u009f]/u : /[\u0000-\u001f\u007f-\u009f]/u).test(value)
const validExcess = (value: unknown): boolean => value === null || object(value) && exact(value, ['reviewed', 'reason', 'value']) && typeof value.reviewed === 'boolean' && text(value.reason, 2000, true) && validValuation(value.value)
const receipt = (data: unknown) => object(data) && validReceiptShape(data) && Object.keys(validateReceiptDraft({ kind: 'receipt', version: 1, data } as Parameters<typeof validateReceiptDraft>[0])).length === 0
const observationAction = (value: unknown): boolean => object(value) && (value.kind === 'void' ? exact(value, ['kind']) : value.kind === 'replace' && exact(value, ['kind', 'quantity', 'unit', 'initial_value', 'excess']) && text(value.quantity, 80) && text(value.unit, 20) && validValuation(value.initial_value) && validExcess(value.excess))
export function cloneCorrection<T>(value: T): T {
  if (Array.isArray(value)) return value.map(item => cloneCorrection(item)) as T
  if (value && typeof value === 'object') return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, cloneCorrection(item)])) as T
  return value
}
export function validCorrectionReplacement(value: unknown): value is CorrectionReplacement {
  if (!object(value) || !exact(value, ['kind', 'data']) || !object(value.data)) return false
  if (value.kind === 'purchase_return') return validPurchaseReturnShape(value.data)
  if (value.kind === 'receipt') return receipt(value.data)
  if (value.kind === 'production') return validProductionShape(value.data) && Object.keys(validateProductionDraft({ kind: 'production', version: 1, data: value.data } as Parameters<typeof validateProductionDraft>[0])).length === 0
  if (value.kind === 'waste') return validWasteShape(value.data) && Object.keys(validateStocktakeDraft({ kind: 'waste', version: 1, data: value.data } as Parameters<typeof validateStocktakeDraft>[0])).length === 0
  if (value.kind === 'sale_count') return exact(value.data, ['count']) && text(value.data.count, 80)
  return value.kind === 'stocktake' && exact(value.data, ['rows']) && Array.isArray(value.data.rows) && value.data.rows.length <= 100 && value.data.rows.every(row => object(row) && exact(row, ['observation_id', 'expected_version_id', 'action']) && uuid(row.observation_id) && uuid(row.expected_version_id) && observationAction(row.action))
}
export function validCorrectionShape(value: unknown): boolean {
  if (!object(value) || !exact(value, ['expected_target_version_id', 'reason', 'action', 'observation_reviews', 'linked_receipt_changes']) || !uuid(value.expected_target_version_id) || !text(value.reason, 2000, true) || !object(value.action)) return false
  const action = value.action
  if (!(action.kind === 'void' ? exact(action, ['kind']) : action.kind === 'replace' && exact(action, ['kind', 'replacement']) && validCorrectionReplacement(action.replacement))) return false
  if (!Array.isArray(value.observation_reviews) || !value.observation_reviews.every(row => object(row) && exact(row, ['observation_id', 'expected_version_id', 'initial_value', 'excess']) && uuid(row.observation_id) && uuid(row.expected_version_id) && validValuation(row.initial_value) && validExcess(row.excess))) return false
  return Array.isArray(value.linked_receipt_changes) && value.linked_receipt_changes.every(row => object(row) && exact(row, ['receipt_id', 'expected_version_id', 'action']) && uuid(row.receipt_id) && uuid(row.expected_version_id) && object(row.action) && (row.action.kind === 'void' ? exact(row.action, ['kind']) : row.action.kind === 'replace' && exact(row.action, ['kind', 'data']) && receipt(row.action.data)))
}
export function validateCorrectionDraft(payload: CorrectionPayload): Record<string, string> {
  return validCorrectionShape(payload.data) ? {} : { reason: '입력 형식과 길이를 확인해 주세요. 정정 이유는 2000자, 수량·금액은 80자 이내입니다.' }
}
export const correctionKinds = { purchase_return: '구입 반품', receipt: '입고', production: '생산', sale_count: '판매 개수', waste: '폐기 · 직원식', stocktake: '실사' }
export function correctionReferenceVersion(reference: CorrectionPlanRef) { return reference.kind === 'existing' ? reference.version_id : reference.expected_version_id }
