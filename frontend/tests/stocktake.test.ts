import test from 'node:test'
import assert from 'node:assert/strict'
import { businessPending, copyPayload, restoreBusinessPending, samePayload, validBusinessPayload } from '../app/utils/business.ts'
import type { BusinessPayload } from '../app/types/business.ts'
import { blankStocktakeDraft, explicitZeroValue } from '../app/utils/stocktake.ts'

const id = '11111111-1111-4111-8111-111111111111'
const draft = () => ({ kind: 'stocktake' as const, version: 1 as const, data: { rows: [{ material_id: id, material_version_id: id, quantity: '0', unit: 'g', initial_value: { kind: 'manual_amount' as const, amount: '0' }, excess: { reviewed: true, reason: '가상 확인', value: { kind: 'manual_amount' as const, amount: '1.' } } }], note: '' } })

test('zero starting value needs an explicit choice and incomplete quantities never create an amount', () => {
  const value = blankStocktakeDraft(); value.data.rows[0]!.quantity = '0'
  assert.equal(value.data.rows[0]!.initial_value, null)
  for (const input of ['', ' ', '0.', '0.0000000', '-0', '1']) assert.equal(explicitZeroValue(input), null)
  assert.deepEqual(explicitZeroValue('0.000000'), { kind: 'manual_amount', amount: '0' })
})

test('stocktake copy isolates both nested valuation paths and original row array', () => {
  const original = draft(), copied = copyPayload(original as unknown as BusinessPayload) as unknown as ReturnType<typeof draft>
  copied.data.rows[0]!.initial_value.amount = '9'
  copied.data.rows[0]!.excess.value.amount = '4'
  copied.data.rows.push(draft().data.rows[0]!)
  assert.equal(original.data.rows[0]!.initial_value.amount, '0')
  assert.equal(original.data.rows[0]!.excess.value.amount, '1.')
  assert.equal(original.data.rows.length, 1)
})

test('stocktake pending retains exact unfinished values and restores null optional valuations', () => {
  const original = draft(), pending = businessPending('create', null, null, { epoch: id, revision: '12' }, original as unknown as BusinessPayload, null, () => id)
  const raw = JSON.stringify(pending)
  original.data.rows[0]!.excess.reason = '다른 입력'
  assert.equal(JSON.stringify(pending), raw)
  assert.deepEqual(restoreBusinessPending(raw), JSON.parse(raw))
  assert.equal(validBusinessPayload({ kind: 'stocktake', version: 1, data: { rows: [{ material_id: null, material_version_id: null, quantity: '', unit: '', initial_value: null, excess: null }], note: '' } }), true)
})

test('stocktake structural comparison observes nested edits and never treats blank as zero', () => {
  const original = draft(), next = structuredClone(original)
  assert.equal(samePayload(original as unknown as BusinessPayload, next as unknown as BusinessPayload), true)
  next.data.rows[0]!.quantity = ''
  assert.equal(samePayload(original as unknown as BusinessPayload, next as unknown as BusinessPayload), false)
})

test('stocktake and waste drafts preserve incompleteness but reject unknown nested fields and controls', () => {
  assert.equal(validBusinessPayload({ kind: 'stocktake', version: 1, data: { rows: [], note: '' } }), true)
  assert.equal(validBusinessPayload({ kind: 'waste', version: 1, data: { material_id: null, material_version_id: null, quantity: '1.', unit: 'g', reason: 'staff_meal', note: '' } }), true)
  const invalid = draft(); Object.assign(invalid.data.rows[0]!.excess.value, { guessed: true })
  assert.equal(validBusinessPayload(invalid), false)
  const control = draft(); control.data.rows[0]!.excess.reason = '\u0085'
  assert.equal(validBusinessPayload(control), false)
  const oversized = draft(); oversized.data.rows = Array.from({ length: 101 }, () => draft().data.rows[0]!)
  assert.equal(validBusinessPayload(oversized), false)
})
