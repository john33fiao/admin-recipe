import type { BusinessPayload, BusinessPending, BusinessPreview, DraftPriceV1 } from '../types/business.ts'
import type { CatalogState } from '../types/material.ts'

export const businessStatuses = { draft: '임시 저장', confirmed: '확정', archived: '보관' }
export const blankPriceDraft = (): BusinessPayload => ({ kind: 'catalog_price', version: 1, data: { material_id: null, material_version_id: null, quantity: '', unit: 'g', amount: '', note: '' } })
const uuid = (value: unknown): value is string => typeof value === 'string' && /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(value)
const object = (value: unknown): value is Record<string, unknown> => !!value && typeof value === 'object' && !Array.isArray(value)
const exact = (value: Record<string, unknown>, keys: string[]) => Object.keys(value).length === keys.length && keys.every(key => Object.hasOwn(value, key))

export function validateBusinessDraft(payload: BusinessPayload): Partial<Record<keyof DraftPriceV1, string>> {
  const errors: Partial<Record<keyof DraftPriceV1, string>> = {}
  const data = payload.data
  for (const field of ['material_id', 'material_version_id'] as const) if (data[field] !== null && !uuid(data[field])) errors[field] = '재료를 다시 선택해 주세요.'
  for (const [field, limit] of [['quantity', 80], ['amount', 80], ['unit', 20], ['note', 2000]] as const) {
    const value = data[field]
    if (typeof value !== 'string' || [...value].length > limit || (field === 'note' ? /[\u0000-\u0009\u000b-\u001f\u007f-\u009f]/u : /[\u0000-\u001f\u007f-\u009f]/u).test(value)) errors[field] = `${limit}자 이내의 올바른 문자를 입력해 주세요.`
  }
  return errors
}

export function copyPayload(payload: BusinessPayload): BusinessPayload { return { kind: 'catalog_price', version: 1, data: { ...payload.data } } }
export function samePayload(a: BusinessPayload, b: BusinessPayload) { return Object.keys(a.data).every(key => a.data[key as keyof DraftPriceV1] === b.data[key as keyof DraftPriceV1]) }

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

function validPayload(value: unknown): value is BusinessPayload {
  if (!object(value) || !exact(value, ['kind', 'version', 'data']) || value.kind !== 'catalog_price' || value.version !== 1 || !object(value.data)) return false
  if (!exact(value.data, ['material_id', 'material_version_id', 'quantity', 'unit', 'amount', 'note'])) return false
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
      if (value.record_id !== null || !exact(body, ['meta', 'payload', 'corrects_confirmation_id']) || !validPayload(body.payload) || !(body.corrects_confirmation_id === null || uuid(body.corrects_confirmation_id))) return null
    } else {
      if (!uuid(value.record_id) || !uuid(body.expected_head_id)) return null
      if (value.action === 'revise') { if (!exact(body, ['meta', 'expected_head_id', 'payload']) || !validPayload(body.payload)) return null }
      else if (value.action === 'confirm' || value.action === 'archive') { if (!exact(body, ['meta', 'expected_head_id'])) return null }
      else return null
    }
    return value as unknown as BusinessPending
  } catch { return null }
}
