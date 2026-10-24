import type { DraftStocktakeRow, StocktakePayload, StocktakeValueDraft } from '../types/stocktake.ts'
import type { WastePayload } from '../types/waste.ts'

export const blankStocktakeRow = (): DraftStocktakeRow => ({ material_id: null, material_version_id: null, quantity: '', unit: 'g', initial_value: null, excess: null })
export const blankStocktakeDraft = (): StocktakePayload => ({ kind: 'stocktake', version: 1, data: { rows: [blankStocktakeRow()], note: '' } })
export const blankWasteDraft = (): WastePayload => ({ kind: 'waste', version: 1, data: { material_id: null, material_version_id: null, quantity: '', unit: 'g', reason: 'discard', note: '' } })
export const wasteReasons = { discard: '명시 폐기', staff_meal: '직원식' }
const object = (value: unknown): value is Record<string, unknown> => !!value && typeof value === 'object' && !Array.isArray(value)
const shape = (value: Record<string, unknown>, required: string[], optional: string[] = []) => required.every(key => Object.hasOwn(value, key)) && Object.keys(value).every(key => [...required, ...optional].includes(key))
const uuid = (value: unknown) => value == null || typeof value === 'string' && /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(value)
const text = (value: unknown, limit: number, multiline = false) => typeof value === 'string' && [...value].length <= limit && !(multiline ? /[\u0000-\u0009\u000b-\u001f\u007f-\u009f]/u : /[\u0000-\u001f\u007f-\u009f]/u).test(value)
export function validValuation(value: unknown): boolean { return value == null || object(value) && (value.kind === 'existing_price' ? shape(value, ['kind']) : value.kind === 'manual_amount' && shape(value, ['kind', 'amount']) && text(value.amount, 80)) }
export function validStocktakeShape(value: unknown): boolean {
  return object(value) && shape(value, ['rows', 'note']) && Array.isArray(value.rows) && value.rows.length <= 100 && value.rows.every(row => object(row) && shape(row, ['quantity', 'unit'], ['material_id', 'material_version_id', 'initial_value', 'excess']) && validValuation(row.initial_value) && (row.excess == null || object(row.excess) && shape(row.excess, ['reviewed', 'reason'], ['value']) && typeof row.excess.reviewed === 'boolean' && validValuation(row.excess.value)))
}
export function validWasteShape(value: unknown): boolean { return object(value) && shape(value, ['quantity', 'unit', 'reason', 'note'], ['material_id', 'material_version_id']) && (value.reason === 'discard' || value.reason === 'staff_meal') }
export function validateStocktakeDraft(payload: StocktakePayload | WastePayload): Record<string, string> {
  const errors: Record<string, string> = {}
  if (!text(payload.data.note, 2000, true)) errors.note = '메모는 올바른 문자 2000자 이내로 입력해 주세요.'
  const rows = payload.kind === 'stocktake' ? payload.data.rows : [payload.data]
  if (rows.length > 100) errors.rows = '한 번에 100행까지 임시 저장할 수 있습니다.'
  rows.forEach((row, i) => {
    const prefix = payload.kind === 'stocktake' ? `rows.${i}.` : ''
    for (const field of ['material_id', 'material_version_id'] as const) if (!uuid(row[field])) errors[`${prefix}${field}`] = '품목과 버전을 다시 확인해 주세요.'
    for (const [field, limit] of [['quantity', 80], ['unit', 20]] as const) if (!text(row[field], limit)) errors[`${prefix}${field}`] = `${limit}자 이내의 올바른 문자를 입력해 주세요.`
    if ('initial_value' in row && !validValuation(row.initial_value)) errors[`${prefix}initial_value`] = '시작 가치 확인 내용을 다시 확인해 주세요.'
    if ('excess' in row && row.excess) {
      if (!text(row.excess.reason, 2000, true)) errors[`${prefix}excess.reason`] = '초과 확인 이유는 올바른 문자 2000자 이내로 입력해 주세요.'
      if (!validValuation(row.excess.value)) errors[`${prefix}excess.value`] = '초과분 금액을 다시 확인해 주세요.'
    }
  })
  return errors
}
export function copyValuation(value: StocktakeValueDraft | null | undefined) { return value == null ? value : { ...value } }
export function explicitZeroValue(quantity: string): StocktakeValueDraft | null { return /^0+(?:\.0{1,6})?$/.test(quantity.trim()) ? { kind: 'manual_amount', amount: '0' } : null }
