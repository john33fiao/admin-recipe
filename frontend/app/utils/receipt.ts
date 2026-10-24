import type { ReceiptPayload, ReceiptStockEffectDraft } from '../types/receipt.ts'
import type { ReceiptInclusionSelection, ReceiptSourceKey, SourceShare } from '../types/reconciliation.ts'
export const blankReceiptDraft = (): ReceiptPayload => ({ kind: 'receipt', version: 1, data: { source: { kind: 'direct', material_id: null, material_version_id: null }, quantity: '', unit: 'kg', amount: '', receipt_date: '', note: '', stock_effect: { kind: 'unreflected' } } })
const object = (value: unknown): value is Record<string, unknown> => !!value && typeof value === 'object' && !Array.isArray(value)
const exact = (value: Record<string, unknown>, keys: string[]) => Object.keys(value).length === keys.length && keys.every(key => Object.hasOwn(value, key))
const nullableUuid = (value: unknown) => value === null || typeof value === 'string' && /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(value)
export const blankReceiptSelection = (): ReceiptInclusionSelection => ({ observation_version_id: null, source: null, included_quantity: '' })
export const receiptSourceLabels = { receipt_inflow: '입고에서 이어진 수량', observation_initial: '확인한 시작 수량', observation_excess: '실사 초과로 확인한 수량' }
export const sourceKeyText = (key: ReceiptSourceKey): string => `${key.kind}:${key.id}:${key.position}`
export function shareText(share: SourceShare): string { return `${share.denominator === '1' ? share.numerator : `${share.numerator}/${share.denominator}`}${share.unit}` }
export function copyReceiptStockEffect(effect: ReceiptStockEffectDraft): ReceiptStockEffectDraft {
  return effect.kind === 'reconciled' ? { kind: 'reconciled', selections: effect.selections.map(row => ({ ...row, source: row.source?.kind === 'source' ? { kind: 'source', source_key: { ...row.source.source_key } } : row.source ? { ...row.source } : null })) } : { ...effect }
}
function validSelection(value: unknown): boolean {
  if (!object(value) || !exact(value, ['observation_version_id', 'source', 'included_quantity']) || !nullableUuid(value.observation_version_id)) return false
  if (value.source === null) return true
  if (!object(value.source)) return false
  if (value.source.kind === 'all_available') return exact(value.source, ['kind'])
  if (value.source.kind !== 'source' || !exact(value.source, ['kind', 'source_key']) || !object(value.source.source_key)) return false
  const key = value.source.source_key
  return exact(key, ['kind', 'id', 'position']) && typeof key.kind === 'string' && Object.hasOwn(receiptSourceLabels, key.kind) && typeof key.id === 'string' && nullableUuid(key.id) && key.position === 0
}
export function validReceiptShape(data: Record<string, unknown>): boolean {
  if (!exact(data, ['source', 'quantity', 'unit', 'amount', 'receipt_date', 'note', 'stock_effect']) || !object(data.source) || !object(data.stock_effect)) return false
  const sourceKeys = data.source.kind === 'direct' ? ['kind', 'material_id', 'material_version_id'] : data.source.kind === 'order' ? ['kind', 'order_id', 'expected_order_version_id', 'material_version_id'] : null
  const effectKeys = data.stock_effect.kind === 'unreflected' ? ['kind'] : data.stock_effect.kind === 'needs_reconciliation' ? ['kind', 'basis_id', 'included_quantity'] : data.stock_effect.kind === 'reconciled' ? ['kind', 'selections'] : null
  return !!sourceKeys && !!effectKeys && exact(data.source, sourceKeys) && exact(data.stock_effect, effectKeys) && (data.stock_effect.kind !== 'reconciled' || Array.isArray(data.stock_effect.selections) && data.stock_effect.selections.length <= 20 && data.stock_effect.selections.every(validSelection))
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
  } else if (data.stock_effect.kind === 'reconciled') {
    if (!Array.isArray(data.stock_effect.selections) || data.stock_effect.selections.length > 20) errors['stock_effect.selections'] = '포함 근거는 20행 이내로 입력해 주세요.'
    else data.stock_effect.selections.forEach((row, index) => {
      if (!validSelection(row)) errors[`stock_effect.selections.${index}.source`] = '선택한 실사와 근거를 다시 확인해 주세요.'
      fields.push([`stock_effect.selections.${index}.included_quantity`, row.included_quantity, 80])
    })
  }
  for (const [field, value, limit] of fields) if (typeof value !== 'string' || [...value].length > limit || (field === 'note' ? /[\u0000-\u0009\u000b-\u001f\u007f-\u009f]/u : /[\u0000-\u001f\u007f-\u009f]/u).test(value)) errors[field] = `${limit}자 이내의 올바른 문자를 입력해 주세요.`
  return errors
}
