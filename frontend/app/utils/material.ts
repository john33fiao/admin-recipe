import type { MaterialInput, PendingMaterial } from '../types/material.ts'

const controlCharacters = /[\u0000-\u001f\u007f-\u009f]/u
const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i

export function normalizeMaterial(input: MaterialInput): MaterialInput {
  return { name: input.name.trim(), unit: input.unit.trim() }
}

export function validateMaterial(input: MaterialInput): Partial<Record<keyof MaterialInput, string>> {
  const errors: Partial<Record<keyof MaterialInput, string>> = {}
  if (!input.name) errors.name = '재료 이름을 입력해 주세요.'
  else if ([...input.name].length > 100) errors.name = '재료 이름은 100자 이내로 입력해 주세요.'
  else if (controlCharacters.test(input.name)) errors.name = '재료 이름에 줄바꿈이나 제어 문자를 사용할 수 없습니다.'
  if (!input.unit) errors.unit = '관리 단위를 입력해 주세요.'
  else if ([...input.unit].length > 20) errors.unit = '관리 단위는 20자 이내로 입력해 주세요.'
  else if (controlCharacters.test(input.unit)) errors.unit = '관리 단위에 줄바꿈이나 제어 문자를 사용할 수 없습니다.'
  return errors
}

// A request key always belongs to one immutable payload, including after refresh.
export function prepareRequest(input: MaterialInput, pending: PendingMaterial | null, makeId: () => string): PendingMaterial {
  const normalized = normalizeMaterial(input)
  if (Object.keys(validateMaterial(normalized)).length) throw new Error('invalid-material')
  if (pending) {
    if (pending.name !== normalized.name || pending.unit !== normalized.unit) throw new Error('changed-pending-request')
    return pending
  }
  return { ...normalized, request_id: makeId() }
}

export function restorePending(raw: string | null): PendingMaterial | null {
  if (!raw) return null
  try {
    const value: unknown = JSON.parse(raw)
    if (!value || typeof value !== 'object') return null
    const data = value as Record<string, unknown>
    if (typeof data.name !== 'string' || typeof data.unit !== 'string' || typeof data.request_id !== 'string') return null
    if (!uuid.test(data.request_id)) return null
    const input = normalizeMaterial({ name: data.name, unit: data.unit })
    if (input.name !== data.name || input.unit !== data.unit || Object.keys(validateMaterial(input)).length) return null
    return { ...input, request_id: data.request_id }
  } catch {
    return null
  }
}

export function formatCreatedAt(value: string, timeZone?: string): string {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return '등록 시각을 확인할 수 없습니다'
  return new Intl.DateTimeFormat('ko-KR', { dateStyle: 'medium', timeStyle: 'short', timeZone }).format(date)
}
