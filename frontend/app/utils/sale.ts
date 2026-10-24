import type { SalePayload } from '../types/sale.ts'
export const saleConsumptionLabels = { unconfirmed: '실사 소모 미확정', partially_settled: '일부 품목 정산됨', settled: '실사 정산됨', not_applicable: '직접 구성 재료 없음' }
export const blankSaleDraft = (): SalePayload => ({ kind: 'sale', version: 1, data: { material_id: null, material_version_id: null, recipe_version_id: null, count: '', note: '' } })
export function validSaleShape(data: Record<string, unknown>) { const keys = ['material_id', 'material_version_id', 'recipe_version_id', 'count', 'note']; return Object.keys(data).length === keys.length && keys.every(key => Object.hasOwn(data, key)) }
export function validateSaleDraft(payload: SalePayload) {
  const errors: Record<string, string> = {}
  for (const field of ['material_id', 'material_version_id', 'recipe_version_id'] as const) if (payload.data[field] !== null && (typeof payload.data[field] !== 'string' || !/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(payload.data[field]))) errors[field] = '메뉴와 레시피 참조를 다시 확인해 주세요.'
  for (const [field, max] of [['count', 80], ['note', 2000]] as const) { const value = payload.data[field]; if (typeof value !== 'string' || [...value].length > max || (field === 'note' ? /[\u0000-\u0009\u000b-\u001f\u007f-\u009f]/u : /[\u0000-\u001f\u007f-\u009f]/u).test(value)) errors[field] = `${max}자 이내의 올바른 문자를 입력해 주세요.` }
  return errors
}
