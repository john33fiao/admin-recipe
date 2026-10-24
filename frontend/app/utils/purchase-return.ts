import type { PurchaseReturnPayload, PurchaseReturnSummary } from '../types/purchase-return.ts'

export const blankPurchaseReturnDraft = (): PurchaseReturnPayload => ({ kind: 'purchase_return', version: 1, data: { receipt_id: null, expected_receipt_version_id: null, quantity: '', unit: '', note: '' } })
const uuid = (value: unknown) => value === null || typeof value === 'string' && /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(value)
export function validatePurchaseReturnDraft(payload: PurchaseReturnPayload): Record<string, string> {
  const errors: Record<string, string> = {}
  for (const field of ['receipt_id', 'expected_receipt_version_id'] as const) if (!uuid(payload.data[field])) errors[field] = '원입고와 선택한 버전을 확인해 주세요.'
  for (const [field, limit] of [['quantity', 80], ['unit', 20], ['note', 2000]] as const) {
    const value = payload.data[field]
    if (typeof value !== 'string' || [...value].length > limit || (field === 'note' ? /[\u0000-\u0009\u000b-\u001f\u007f-\u009f]/u : /[\u0000-\u001f\u007f-\u009f]/u).test(value)) errors[field] = `${limit}자 이내의 올바른 문자를 입력해 주세요.`
  }
  return errors
}
export function validPurchaseReturnShape(value: unknown): boolean {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false
  const keys = ['receipt_id', 'expected_receipt_version_id', 'quantity', 'unit', 'note']
  return Object.keys(value).length === keys.length && keys.every(key => Object.hasOwn(value, key)) && Object.keys(validatePurchaseReturnDraft({ kind: 'purchase_return', version: 1, data: value } as PurchaseReturnPayload)).length === 0
}
export function effectivePurchaseReturn(value: PurchaseReturnSummary) { return value.effective?.kind === 'void' ? null : value.effective?.kind === 'active' ? value.effective : value.item }
