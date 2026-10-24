import type { CatalogInput, CatalogRegistration, CatalogState, MaterialKind, MaterialMutation, MutationMeta, PriceInput, RegistrationRequest, ResolvedPrice } from '../types/material.ts'
import { normalizeMaterial, restorePending, validateMaterial } from './material.ts'

export const materialKinds: Record<MaterialKind, string> = { raw: '원물', prepared: '손질재료', stocked: '비축품', menu: '판매메뉴', unclassified: '종류 미지정' }
export const priceSources = { holding_average: '보유 평균 단가', last_valid: '마지막 유효 단가', preregistered: '사전 등록 가격', missing: '가격 없음' }
const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i

export function normalizeCatalog(input: CatalogInput): CatalogInput {
  return { ...normalizeMaterial(input), kind: input.kind, description: input.description.trim() }
}
export function validateCatalogInput(input: CatalogInput): Partial<Record<keyof CatalogInput, string>> {
  const errors: Partial<Record<keyof CatalogInput, string>> = validateMaterial(input)
  if (!Object.hasOwn(materialKinds, input.kind)) errors.kind = '재료 종류를 선택해 주세요.'
  if ([...input.description].length > 2000) errors.description = '설명은 2,000자 이내로 입력해 주세요.'
  else if (/[\u0000-\u0009\u000b-\u001f\u007f-\u009f]/u.test(input.description)) errors.description = '설명에 줄바꿈 외 제어 문자를 사용할 수 없습니다.'
  return errors
}
export function decimalInput(raw: string, kind: 'money' | 'quantity'): string | null {
  const value = raw.trim()
  if (!/^\d+(?:\.\d{1,6})?$/.test(value)) return null
  const [whole = '0', fraction = ''] = value.split('.')
  const micros = BigInt(whole) * 1_000_000n + BigInt(fraction.padEnd(6, '0'))
  if (micros > (kind === 'money' ? 9_000_000_000_000_000_000n : 9_000_000_000_000_000n)) return null
  const normalizedFraction = fraction.replace(/0+$/, '')
  return `${BigInt(whole)}${normalizedFraction ? `.${normalizedFraction}` : ''}`
}
export function validatePriceInput(input: PriceInput): Partial<Record<keyof PriceInput, string>> {
  const errors: Partial<Record<keyof PriceInput, string>> = {}
  if (decimalInput(input.amount, 'money') === null) errors.amount = '금액을 0 이상, 소수 6자리 이내의 숫자로 입력해 주세요.'
  const quantity = decimalInput(input.quantity, 'quantity')
  if (quantity === null) errors.quantity = '가격 기준 수량을 소수 6자리 이내의 숫자로 입력해 주세요.'
  else if (quantity === '0') errors.quantity = '가격 기준 수량은 0보다 커야 합니다.'
  const unitError = validateMaterial({ name: '가격', unit: input.unit.trim() }).unit
  if (unitError) errors.unit = unitError
  return errors
}
export function displayDecimal(value: string): string {
  const [whole = '', fraction] = value.split('.')
  return whole.replace(/\B(?=(\d{3})+(?!\d))/g, ',') + (fraction ? `.${fraction}` : '')
}
export function priceText(price: ResolvedPrice, unit: string): string {
  if (price.amount === null || price.quantity === null) return '가격 없음'
  return `${displayDecimal(price.amount)}원 / ${displayDecimal(price.quantity)}${unit}`
}
export function failureAction(status: number, code?: string): 'edit' | 'refresh' | 'blocked' | 'retry' {
  if (status === 400) return 'edit'
  if (status === 409) return code === 'stale_state' ? 'refresh' : 'blocked'
  return 'retry'
}
function object(value: unknown): value is Record<string, unknown> { return !!value && typeof value === 'object' && !Array.isArray(value) }
function hasMeta(data: Record<string, unknown>): data is Record<string, unknown> & MutationMeta {
  return typeof data.request_id === 'string' && uuid.test(data.request_id) && typeof data.epoch === 'string' && uuid.test(data.epoch) && typeof data.expected_revision === 'string' && /^\d+$/.test(data.expected_revision)
}
function catalogFields(data: Record<string, unknown>): CatalogInput | null {
  if (typeof data.name !== 'string' || typeof data.unit !== 'string' || typeof data.kind !== 'string' || typeof data.description !== 'string') return null
  const value = { name: data.name, unit: data.unit, kind: data.kind as MaterialKind, description: data.description }
  if (Object.keys(validateCatalogInput(value)).length) return null
  return value
}
export function restoreCatalogRegistration(raw: string | null): RegistrationRequest | null {
  if (!raw) return null
  try {
    const data: unknown = JSON.parse(raw)
    if (!object(data)) return null
    if (!['kind', 'description', 'epoch', 'expected_revision'].some(key => key in data)) return restorePending(raw)
    const input = catalogFields(data)
    if (!input || !hasMeta(data)) return null
    return { ...input, request_id: data.request_id, epoch: data.epoch, expected_revision: data.expected_revision }
  } catch { return null }
}
export function prepareCatalogRegistration(input: CatalogInput, state: CatalogState, pending: RegistrationRequest | null, makeId: () => string): CatalogRegistration | RegistrationRequest {
  const normalized = normalizeCatalog(input)
  if (Object.keys(validateCatalogInput(normalized)).length) throw new Error('invalid-catalog-input')
  if (pending) {
    const same = pending.name === normalized.name && pending.unit === normalized.unit && (!('kind' in pending) || (pending.kind === normalized.kind && pending.description === normalized.description))
    if (!same) throw new Error('changed-pending-request')
    return pending
  }
  return { ...normalized, request_id: makeId(), epoch: state.epoch, expected_revision: state.revision }
}
export function restoreMaterialMutation(raw: string | null): MaterialMutation | null {
  if (!raw) return null
  try {
    const value: unknown = JSON.parse(raw)
    if (!object(value) || !object(value.body)) return null
    const body = value.body
    if (!hasMeta(body) || typeof body.expected_version_id !== 'string' || !uuid.test(body.expected_version_id)) return null
    const meta = { request_id: body.request_id, epoch: body.epoch, expected_revision: body.expected_revision, expected_version_id: body.expected_version_id }
    if (value.action === 'archive') return { action: 'archive', body: meta }
    if (value.action === 'version') {
      const input = catalogFields(body)
      return input ? { action: 'version', body: { ...input, ...meta } } : null
    }
    if (value.action === 'price' && typeof body.amount === 'string' && typeof body.quantity === 'string' && typeof body.unit === 'string') {
      const input = { amount: body.amount, quantity: body.quantity, unit: body.unit }
      if (!Object.keys(validatePriceInput(input)).length) return { action: 'price', body: { ...input, ...meta } }
    }
    return null
  } catch { return null }
}
