import type { ReceiptPayload } from '../types/receipt.ts'
export const blankReceiptDraft = (): ReceiptPayload => ({ kind: 'receipt', version: 1, data: { source: { kind: 'direct', material_id: null, material_version_id: null }, quantity: '', unit: 'kg', amount: '', receipt_date: '', note: '', stock_effect: { kind: 'unreflected' } } })
const object = (value: unknown): value is Record<string, unknown> => !!value && typeof value === 'object' && !Array.isArray(value)
const exact = (value: Record<string, unknown>, keys: string[]) => Object.keys(value).length === keys.length && keys.every(key => Object.hasOwn(value, key))
const nullableUuid = (value: unknown) => value === null || typeof value === 'string' && /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(value)
export function validReceiptShape(data: Record<string, unknown>): boolean {
  if (!exact(data, ['source', 'quantity', 'unit', 'amount', 'receipt_date', 'note', 'stock_effect']) || !object(data.source) || !object(data.stock_effect)) return false
  const sourceKeys = data.source.kind === 'direct' ? ['kind', 'material_id', 'material_version_id'] : data.source.kind === 'order' ? ['kind', 'order_id', 'expected_order_version_id', 'material_version_id'] : null
  const effectKeys = data.stock_effect.kind === 'unreflected' ? ['kind'] : data.stock_effect.kind === 'needs_reconciliation' ? ['kind', 'basis_id', 'included_quantity'] : null
  return !!sourceKeys && !!effectKeys && exact(data.source, sourceKeys) && exact(data.stock_effect, effectKeys)
}
export function validateReceiptDraft(payload: ReceiptPayload): Record<string, string> {
  const errors: Record<string, string> = {}
  const { data } = payload
  const source = data.source
  for (const [field, value] of Object.entries(source)) if (field !== 'kind' && !nullableUuid(value)) errors[`source.${field}`] = '대상과 현재 버전을 다시 확인해 주세요.'
  const fields: Array<[string, unknown, number]> = [['quantity', data.quantity, 80], ['amount', data.amount, 80], ['unit', data.unit, 20], ['receipt_date', data.receipt_date, 32], ['note', data.note, 2000]]
  if (data.stock_effect.kind === 'needs_reconciliation') {
    if (!nullableUuid(data.stock_effect.basis_id)) errors['stock_effect.basis_id'] = '보관한 근거 참조를 확인해 주세요.'
    fields.push(['stock_effect.included_quantity', data.stock_effect.included_quantity, 80])
  }
  for (const [field, value, limit] of fields) if (typeof value !== 'string' || [...value].length > limit || (field === 'note' ? /[\u0000-\u0009\u000b-\u001f\u007f-\u009f]/u : /[\u0000-\u001f\u007f-\u009f]/u).test(value)) errors[field] = `${limit}자 이내의 올바른 문자를 입력해 주세요.`
  return errors
}
