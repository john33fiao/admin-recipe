import type { DraftProductionInput, DraftProductionOutput, ProductionPayload } from '../types/production.ts'

export const blankProductionInput = (): DraftProductionInput => ({ material_id: null, material_version_id: null, quantity: '', unit: 'g' })
export const blankProductionOutput = (first: boolean): DraftProductionOutput => ({ ...blankProductionInput(), allocation_percent: first ? '100' : '0' })
export const blankProductionDraft = (): ProductionPayload => ({ kind: 'production', version: 1, data: { inputs: [blankProductionInput()], outputs: [blankProductionOutput(true)], note: '' } })
const object = (v: unknown): v is Record<string, unknown> => !!v && typeof v === 'object' && !Array.isArray(v)
const exact = (v: Record<string, unknown>, keys: string[]) => Object.keys(v).length === keys.length && keys.every(key => Object.hasOwn(v, key))
const uuid = (v: unknown) => v === null || typeof v === 'string' && /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(v)
export function validProductionShape(data: Record<string, unknown>) {
  if (!exact(data, ['inputs', 'outputs', 'note'])) return false
  return ['inputs', 'outputs'].every(side => Array.isArray(data[side]) && data[side].every(row => object(row) && exact(row, ['material_id', 'material_version_id', 'quantity', 'unit', ...(side === 'outputs' ? ['allocation_percent'] : [])])))
}
export function validateProductionDraft(payload: ProductionPayload) {
  const errors: Record<string, string> = {}
  const text = (v: unknown, max: number, note = false) => typeof v === 'string' && [...v].length <= max && !(note ? /[\u0000-\u0009\u000b-\u001f\u007f-\u009f]/u : /[\u0000-\u001f\u007f-\u009f]/u).test(v)
  for (const side of ['inputs', 'outputs'] as const) {
    if (payload.data[side].length > 100) errors[side] = '한쪽에 최대 100행까지 임시 저장할 수 있습니다.'
    payload.data[side].forEach((row, index) => {
      for (const field of ['material_id', 'material_version_id'] as const) if (!uuid(row[field])) errors[`${side}.${index}.${field}`] = '품목과 현재 정보를 다시 확인해 주세요.'
      for (const [field, limit] of [['quantity', 80], ['unit', 20], ...(side === 'outputs' ? [['allocation_percent', 80] as const] : [])] as const) if (!text((row as unknown as Record<string, unknown>)[field], limit)) errors[`${side}.${index}.${field}`] = `${limit}자 이내의 올바른 문자를 입력해 주세요.`
    })
  }
  if (!text(payload.data.note, 2000, true)) errors.note = '메모는 줄바꿈을 포함해 2000자 이내로 입력해 주세요.'
  return errors
}
