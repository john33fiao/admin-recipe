import test from 'node:test'
import assert from 'node:assert/strict'
import { blankRecipe, copyRecipe, normalizeRecipe, ratioPercent, recipeFailure, recipeFraction, recipeShape, restoreRecipePending, sameRecipe, validateRecipe, validateRecipeOwner } from '../app/utils/recipe.ts'

const id = '11111111-1111-4111-8111-111111111111'
const line = { material_id: id, material_version_id: id, quantity: '0', unit: 'g', note: '적당량' }
test('recipe reference output and each line are independent from baseline and pending input', () => {
  const original = { ...blankRecipe(), reference_output: { quantity: '2', unit: 'kg' }, lines: [{ ...line }] }
  const copied = copyRecipe(original)
  copied.reference_output!.quantity = '3'; copied.lines[0]!.note = '새 메모'
  assert.equal(original.reference_output.quantity, '2')
  assert.equal(original.lines[0]!.note, '적당량')
  assert.equal(sameRecipe(original, copied), false)
})
test('recipe allows zero and missing sale price but rejects blank quantity and incompatible reference basis', () => {
  assert.deepEqual(validateRecipe({ ...blankRecipe(), lines: [line] }), {})
  assert.ok(validateRecipe({ ...blankRecipe(), lines: [{ ...line, quantity: '' }] })['line-0-quantity'])
  assert.ok(validateRecipe({ ...blankRecipe(), basis_count: '3', reference_output: { quantity: '1', unit: 'kg' } }).basis_count)
  for (const basis_count of ['0', '1000000001', '1e3']) assert.ok(validateRecipe({ ...blankRecipe(), basis_count }).basis_count)
  const normalized = normalizeRecipe({ ...blankRecipe(), selling_price: '0', process: '씻기\r\n끓이기', lines: [{ ...line, quantity: '00100.000' }] })
  assert.equal(normalized.lines[0]!.quantity, '00100.000'); assert.equal(normalized.selling_price, '0'); assert.equal(normalized.process, '씻기\n끓이기')
})
test('recipe field limits use unicode scalars and pending preserves exact nested data', () => {
  const data = { ...blankRecipe(), lines: [{ ...line, note: '🍚'.repeat(200) }] }
  assert.deepEqual(validateRecipe(data), {})
  assert.ok(validateRecipe({ ...data, lines: [{ ...line, note: '🍚'.repeat(201) }] })['line-0-note'])
  assert.ok(validateRecipe({ ...data, lines: Array.from({ length: 101 }, () => ({ ...line })) }).lines)
  const request = { action: 'create', material_id: null, body: { meta: { request_id: id, epoch: id, expected_revision: '1' }, owner: { mode: 'new', name: '가상 메뉴', unit: '개', kind: 'menu', description: '' }, data } }
  assert.deepEqual(restoreRecipePending(JSON.stringify(request)), request)
  assert.equal(recipeShape({ ...data, extra: true }), false)
  assert.equal(recipeShape({ ...data, reference_output: { quantity: '1', unit: 'kg', guessed: true } }), false)
  assert.equal(recipeFailure(413), 'edit'); assert.equal(recipeFailure(409, 'request_conflict'), 'blocked'); assert.equal(recipeFailure(503), 'retry')
})
test('recipe display keeps fractions exact and rounds ratios only for display', () => {
  assert.equal(recipeFraction('100', '3', 'g'), '100 ÷ 3 g')
  assert.equal(ratioPercent({ numerator: '333333333', denominator: '1000000000' }), '33.33%')
  assert.equal(ratioPercent({ numerator: '0', denominator: '1000' }), '0%')
  assert.equal(ratioPercent({ numerator: '1', denominator: '0' }), '계산 불가')
})
test('new recipe owners require an explicit kind while old unclassified requests remain recoverable', () => {
  const owner = { mode: 'new' as const, name: '가상 품목', unit: 'g', kind: 'unclassified' as const, description: '' }
  assert.ok(validateRecipeOwner(owner).kind)
  assert.deepEqual(validateRecipeOwner({ mode: 'existing', material_id: id, expected_material_version_id: id }), {})
  const pending = { action: 'create', material_id: null, body: { meta: { request_id: id, epoch: id, expected_revision: '1' }, owner, data: blankRecipe() } }
  assert.deepEqual(restoreRecipePending(JSON.stringify(pending)), pending)
})
test('restored UUIDv8 material versions are valid references while new request IDs remain UUIDv4', () => {
  const version = '22222222-2222-8222-8222-222222222222'
  const data = { ...blankRecipe(), lines: [{ ...line, material_version_id: version }] }
  assert.deepEqual(validateRecipe(data), {})
  const pending = { action: 'create', material_id: null, body: { meta: { request_id: id, epoch: id, expected_revision: '1' }, owner: { mode: 'existing', material_id: id, expected_material_version_id: version }, data } }
  assert.deepEqual(restoreRecipePending(JSON.stringify(pending)), pending)
  assert.equal(restoreRecipePending(JSON.stringify({ ...pending, body: { ...pending.body, meta: { ...pending.body.meta, request_id: version } } })), null)
})
