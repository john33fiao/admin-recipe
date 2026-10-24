import test from 'node:test'
import assert from 'node:assert/strict'
import { copyPayload, samePayload, businessPending, restoreBusinessPending, validBusinessPayload, validateBusinessDraft } from '../app/utils/business.ts'
import { blankProductionDraft, blankProductionOutput } from '../app/utils/production.ts'
import type { ProductionPayload } from '../app/types/production.ts'

const id = '11111111-1111-4111-8111-111111111111'
const production = (): ProductionPayload => ({ kind: 'production', version: 1, data: { inputs: [{ material_id: id, material_version_id: id, quantity: '1.', unit: 'kg' }], outputs: [{ material_id: id, material_version_id: id, quantity: '', unit: 'kg', allocation_percent: '100' }], note: '가상 생산 초안' } })

test('production copy isolates input quantities and output allocation from original payload', () => {
  const original = production()
  const copied = copyPayload(original)
  copied.data.inputs[0]!.quantity = '3'
  copied.data.outputs[0]!.allocation_percent = '70'
  assert.equal(original.data.inputs[0]!.quantity, '1.')
  assert.equal(original.data.outputs[0]!.allocation_percent, '100')
})

test('independent production arrays compare structurally and detect allocation-only edits', () => {
  const original = production()
  const independent = JSON.parse(JSON.stringify(original)) as ReturnType<typeof production>
  assert.equal(samePayload(original, independent), true)
  independent.data.outputs[0]!.allocation_percent = '70'
  assert.equal(samePayload(original, independent), false)
})

test('production defaults only allocation and accepts unfinished actual measurements', () => {
  const value = blankProductionDraft()
  assert.equal(value.data.inputs[0]!.quantity, '')
  assert.equal(value.data.outputs[0]!.quantity, '')
  assert.equal(value.data.outputs[0]!.allocation_percent, '100')
  value.data.outputs.push(blankProductionOutput(false))
  assert.equal(value.data.outputs[1]!.allocation_percent, '0')
  value.data.outputs[0]!.allocation_percent = '70'
  value.data.outputs[1]!.allocation_percent = '30'
  value.data.inputs[0]!.quantity = '1.'
  assert.deepEqual(validateBusinessDraft(value), {})
  value.data.outputs.splice(0, 1)
  assert.equal(value.data.outputs[0]!.allocation_percent, '30')
  assert.deepEqual(validateBusinessDraft(value), {})
  value.data.inputs = []; value.data.outputs = []
  assert.equal(validBusinessPayload(value), true)
})

test('production shape limits each side and preserves exact raw pending arrays', () => {
  const value = production()
  value.data.note = '가'.repeat(2000)
  const pending = businessPending('create', null, null, { epoch: id, revision: '9' }, value, null, () => id)
  value.data.inputs[0]!.quantity = '8'
  assert.equal(pending.body.payload.kind, 'production')
  assert.equal((pending.body.payload as ProductionPayload).data.inputs[0]!.quantity, '1.')
  assert.deepEqual(restoreBusinessPending(JSON.stringify(pending)), pending)
  assert.equal(validBusinessPayload({ ...value, data: { ...value.data, inputs: [{ ...value.data.inputs[0], note: '' }] } }), false)
  value.data.inputs = Array.from({ length: 101 }, () => ({ ...value.data.inputs[0]! }))
  assert.ok(validateBusinessDraft(value).inputs)
  value.data.note = '\u0085'
  assert.ok(validateBusinessDraft(value).note)
})
