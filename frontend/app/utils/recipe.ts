import type { RecipeDataInput, RecipeOwnerInput, RecipePending, RecipeVersion } from '../types/recipe.ts'
import { decimalInput, displayDecimal, failureAction, validateCatalogInput } from './catalog.ts'

// Imported/restored entity references can include deterministic UUIDv8 IDs.
// Newly generated mutation identities keep the existing UUIDv4 contract.
const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i
const requestUuid = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i
const object = (value: unknown): value is Record<string, unknown> => !!value && typeof value === 'object' && !Array.isArray(value)
const keys = (value: Record<string, unknown>, fields: string[]) => Object.keys(value).length === fields.length && fields.every(key => Object.hasOwn(value, key))
const textFields = (value: Record<string, unknown>, fields: string[]) => fields.every(key => typeof value[key] === 'string')
const cleanText = (value: string) => value.replace(/\r\n?/g, '\n')
export function blankRecipe(): RecipeDataInput { return { basis_count: '1', reference_output: null, selling_price: null, cooking_description: '', process: '', reference_time: '', lines: [] } }
export function copyRecipe(data: RecipeDataInput): RecipeDataInput { return { ...data, reference_output: data.reference_output ? { ...data.reference_output } : null, lines: data.lines.map(line => ({ ...line })) } }
export function sameRecipe(left: RecipeDataInput, right: RecipeDataInput) {
  return left.basis_count === right.basis_count && left.selling_price === right.selling_price && left.cooking_description === right.cooking_description && left.process === right.process && left.reference_time === right.reference_time && (left.reference_output === null ? right.reference_output === null : !!right.reference_output && left.reference_output.quantity === right.reference_output.quantity && left.reference_output.unit === right.reference_output.unit) && left.lines.length === right.lines.length && left.lines.every((line, index) => { const other = right.lines[index]!; return line.material_id === other.material_id && line.material_version_id === other.material_version_id && line.quantity === other.quantity && line.unit === other.unit && line.note === other.note })
}
export function recipeFromVersion(value: RecipeVersion): RecipeDataInput {
  return { basis_count: value.basis_count, reference_output: value.reference_output ? { quantity: value.reference_output.input_quantity, unit: value.reference_output.input_unit } : null, selling_price: value.selling_price, cooking_description: value.cooking_description, process: value.process, reference_time: value.reference_time, lines: value.lines.map(line => ({ material_id: line.material_id, material_version_id: line.material_version_id, quantity: line.input_quantity, unit: line.input_unit, note: line.note })) }
}
export function recipeShape(value: unknown): value is RecipeDataInput {
  if (!object(value) || !keys(value, ['basis_count', 'reference_output', 'selling_price', 'cooking_description', 'process', 'reference_time', 'lines']) || !textFields(value, ['basis_count', 'cooking_description', 'process', 'reference_time']) || !(value.selling_price === null || typeof value.selling_price === 'string') || !Array.isArray(value.lines)) return false
  const reference = value.reference_output
  if (reference !== null && (!object(reference) || !keys(reference, ['quantity', 'unit']) || !textFields(reference, ['quantity', 'unit']))) return false
  return value.lines.every(line => object(line) && keys(line, ['material_id', 'material_version_id', 'quantity', 'unit', 'note']) && textFields(line, ['material_id', 'material_version_id', 'quantity', 'unit', 'note']))
}
export function validateRecipe(data: RecipeDataInput): Record<string, string> {
  const errors: Record<string, string> = {}
  const text = (field: string, value: string, max: number) => { if ([...cleanText(value)].length > max) errors[field] = `${max.toLocaleString()}자 이내로 입력해 주세요.`; else if (/[\u0000-\u0009\u000b-\u001f\u007f-\u009f]/u.test(cleanText(value))) errors[field] = '줄바꿈 외 제어 문자를 사용할 수 없습니다.' }
  const unit = (field: string, value: string) => { const error = validateCatalogInput({ name: '구성', kind: 'unclassified', description: '', unit: value }).unit; if (error) errors[field] = error }
  const basis = data.basis_count.trim()
  if (!/^\d+$/.test(basis) || BigInt(basis || '0') < 1n || BigInt(basis || '0') > 1_000_000_000n) errors.basis_count = '기준 개수를 1~1,000,000,000의 정수로 입력해 주세요.'
  if (data.selling_price !== null && decimalInput(data.selling_price, 'money') === null) errors.selling_price = '판매가는 0 이상, 소수 6자리 이내로 입력해 주세요.'
  text('cooking_description', data.cooking_description, 2000); text('process', data.process, 4000); text('reference_time', data.reference_time, 200)
  if (/[\r\n]/u.test(data.reference_time)) errors.reference_time = '참고 시간은 한 줄로 입력해 주세요.'
  if (data.reference_output) {
    const quantity = decimalInput(data.reference_output.quantity, 'quantity')
    if (!quantity || quantity === '0') errors.reference_quantity = '참고 생산량은 0보다 큰 수량으로 입력해 주세요.'
    unit('reference_unit', data.reference_output.unit)
    if (!/^0*1$/.test(basis)) errors.basis_count = '참고 생산량을 적을 때 기준 개수는 1이어야 합니다.'
  }
  if (data.lines.length > 100) errors.lines = '구성 재료는 100행까지 저장할 수 있습니다.'
  data.lines.forEach((line, index) => {
    const field = `line-${index}`
    if (!uuid.test(line.material_id) || !uuid.test(line.material_version_id)) errors[`${field}-material`] = '구성 재료를 선택해 주세요.'
    if (decimalInput(line.quantity, 'quantity') === null) errors[`${field}-quantity`] = '수량은 0 이상, 소수 6자리 이내로 입력해 주세요.'
    unit(`${field}-unit`, line.unit); text(`${field}-note`, line.note, 200)
  })
  return errors
}
export function normalizeRecipe(data: RecipeDataInput): RecipeDataInput {
  return { basis_count: BigInt(data.basis_count.trim()).toString(), reference_output: data.reference_output ? { quantity: data.reference_output.quantity.trim(), unit: data.reference_output.unit.trim() } : null, selling_price: data.selling_price === null ? null : data.selling_price.trim(), cooking_description: cleanText(data.cooking_description), process: cleanText(data.process), reference_time: data.reference_time, lines: data.lines.map(line => ({ ...line, quantity: line.quantity.trim(), unit: line.unit.trim(), note: cleanText(line.note) })) }
}
export function ownerDraftShape(value: unknown): value is RecipeOwnerInput {
  if (!object(value)) return false
  return value.mode === 'existing' ? keys(value, ['mode', 'material_id', 'expected_material_version_id']) && textFields(value, ['material_id', 'expected_material_version_id']) : value.mode === 'new' && keys(value, ['mode', 'name', 'unit', 'kind', 'description']) && textFields(value, ['name', 'unit', 'kind', 'description']) && ['raw', 'prepared', 'stocked', 'menu', 'unclassified'].includes(value.kind as string)
}
export function validateRecipeOwner(owner: RecipeOwnerInput): Record<string, string> {
  if (owner.mode === 'existing') return uuid.test(owner.material_id) && uuid.test(owner.expected_material_version_id) ? {} : { material: '레시피를 연결할 품목을 선택해 주세요.' }
  const errors: Record<string, string> = { ...validateCatalogInput(owner) } as Record<string, string>
  if (owner.kind === 'unclassified') errors.kind = '새 품목의 종류를 선택해 주세요.'
  return errors
}
export function ownerShape(value: unknown): value is RecipeOwnerInput {
  if (!object(value)) return false
  if (value.mode === 'existing') return keys(value, ['mode', 'material_id', 'expected_material_version_id']) && typeof value.material_id === 'string' && uuid.test(value.material_id) && typeof value.expected_material_version_id === 'string' && uuid.test(value.expected_material_version_id)
  return value.mode === 'new' && keys(value, ['mode', 'name', 'unit', 'kind', 'description']) && textFields(value, ['name', 'unit', 'kind', 'description']) && !Object.keys(validateCatalogInput(value as unknown as Extract<RecipeOwnerInput, { mode: 'new' }>)).length
}
export function restoreRecipePending(raw: string | null): RecipePending | null {
  try {
    const value: unknown = JSON.parse(raw ?? 'null')
    if (!object(value) || !keys(value, ['action', 'material_id', 'body']) || !object(value.body) || !object(value.body.meta)) return null
    const body = value.body, meta = value.body.meta
    if (!keys(meta, ['request_id', 'epoch', 'expected_revision']) || !textFields(meta, ['request_id', 'epoch', 'expected_revision']) || !requestUuid.test(meta.request_id as string) || !requestUuid.test(meta.epoch as string) || !/^\d+$/.test(meta.expected_revision as string)) return null
    const dataValid = () => recipeShape(body.data) && !Object.keys(validateRecipe(body.data)).length
    if (value.action === 'create') { if (value.material_id !== null || !keys(body, ['meta', 'owner', 'data']) || !ownerShape(body.owner) || !dataValid()) return null }
    else {
      if (typeof value.material_id !== 'string' || !uuid.test(value.material_id) || typeof body.expected_material_version_id !== 'string' || !uuid.test(body.expected_material_version_id)) return null
      if (value.action === 'revise') { if (!keys(body, ['meta', 'expected_material_version_id', 'expected_recipe_version_id', 'data']) || typeof body.expected_recipe_version_id !== 'string' || !uuid.test(body.expected_recipe_version_id) || !dataValid()) return null }
      else if (value.action === 'copy') { if (!keys(body, ['meta', 'expected_material_version_id', 'source_recipe_version_id']) || typeof body.source_recipe_version_id !== 'string' || !uuid.test(body.source_recipe_version_id)) return null }
      else return null
    }
    return value as unknown as RecipePending
  } catch { return null }
}
export function recipeFailure(status: number, code?: string, requestStatus?: string) { return status === 413 ? 'edit' : failureAction(status, code, requestStatus) }
export function recipeFraction(quantity: string, basis: string, unit: string) { return basis === '1' ? `${displayDecimal(quantity)}${unit}` : `${displayDecimal(quantity)} ÷ ${displayDecimal(basis)} ${unit}` }
export function ratioPercent(ratio: { numerator: string; denominator: string }): string {
  const n = BigInt(ratio.numerator), d = BigInt(ratio.denominator)
  if (!d) return '계산 불가'
  const hundredths = (n * 10_000n * 2n + d) / (d * 2n)
  const fraction = (hundredths % 100n).toString().padStart(2, '0').replace(/0+$/, '')
  return `${displayDecimal((hundredths / 100n).toString())}${fraction ? `.${fraction}` : ''}%`
}
