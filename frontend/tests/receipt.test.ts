import test from 'node:test'
import assert from 'node:assert/strict'
import type { BusinessPayload } from '../app/types/business.ts'
import { businessPending, copyPayload, restoreBusinessPending, samePayload } from '../app/utils/business.ts'
const id = '767bfaab-d964-4cfb-a4e6-b79ada5755aa'
const version = 'ad100fb4-3d59-4cd0-a5ee-06697d6bd244'
const state = { epoch: '65ecea4e-74a0-430e-a6d2-acaa533e9b62', revision: '9007199254740993' }
function receipt() { return { kind: 'receipt', version: 1, data: { source: { kind: 'direct', material_id: id, material_version_id: version }, quantity: '1.', unit: 'kg', amount: '', receipt_date: '2026-0', note: '', stock_effect: { kind: 'needs_reconciliation', basis_id: null, included_quantity: '' } } } }

test('receipt nested copy owns source and reconciliation data independently', () => {
  const original = receipt()
  const copy = copyPayload(original as unknown as BusinessPayload) as unknown as ReturnType<typeof receipt>
  copy.data.source.material_version_id = id
  copy.data.stock_effect.included_quantity = '1'
  assert.equal(original.data.source.material_version_id, version)
  assert.equal(original.data.stock_effect.included_quantity, '')
})

test('receipt comparison is structural and uncertain pending keeps the original nested body', () => {
  const original = receipt()
  assert.equal(samePayload(original as unknown as BusinessPayload, JSON.parse(JSON.stringify(original))), true)
  const pending = businessPending('create', null, null, state, original as unknown as BusinessPayload, null, () => id)
  const before = JSON.stringify(pending)
  original.data.source.material_id = version
  original.data.stock_effect.included_quantity = '2'
  assert.equal(JSON.stringify(pending), before)
  assert.equal(samePayload(original as unknown as BusinessPayload, JSON.parse(before).body.payload), false)
})

test('receipt recovery preserves unfinished nested input and rejects incompatible source fields', () => {
  const pending = businessPending('create', null, null, state, receipt() as unknown as BusinessPayload, null, () => id)
  assert.deepEqual(restoreBusinessPending(JSON.stringify(pending)), pending)
  const malformed = JSON.parse(JSON.stringify(pending))
  malformed.body.payload.data.source.order_id = id
  assert.equal(restoreBusinessPending(JSON.stringify(malformed)), null)
  const correction = JSON.parse(JSON.stringify(pending))
  correction.body.corrects_confirmation_id = id
  assert.equal(restoreBusinessPending(JSON.stringify(correction)), null)
})
