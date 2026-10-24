import test from 'node:test'
import assert from 'node:assert/strict'
import type { BusinessPayload } from '../app/types/business.ts'
import { businessPending, copyPayload, restoreBusinessPending, samePayload } from '../app/utils/business.ts'
const id = '767bfaab-d964-4cfb-a4e6-b79ada5755aa'
const version = 'ad100fb4-3d59-4cd0-a5ee-06697d6bd244'
const state = { epoch: '65ecea4e-74a0-430e-a6d2-acaa533e9b62', revision: '9007199254740993' }
function receipt() { return { kind: 'receipt', version: 1, data: { source: { kind: 'direct', material_id: id, material_version_id: version }, quantity: '1.', unit: 'kg', amount: '', receipt_date: '2026-0', note: '', stock_effect: { kind: 'needs_reconciliation', basis_id: null, included_quantity: '' } } } }
function reconciledReceipt() {
  return { ...receipt(), data: { ...receipt().data, stock_effect: { kind: 'reconciled', selections: [
    { observation_version_id: version, source: { kind: 'source', source_key: { kind: 'observation_initial', id, position: 0 } }, included_quantity: '1000' },
    { observation_version_id: id, source: { kind: 'source', source_key: { kind: 'observation_excess', id: version, position: 0 } }, included_quantity: '2000' },
  ] } } }
}

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

test('reconciled receipt copy owns selection arrays and nested source keys', () => {
  const original = reconciledReceipt()
  const before = JSON.stringify(original)
  const copy = copyPayload(original as unknown as BusinessPayload) as unknown as ReturnType<typeof reconciledReceipt>
  copy.data.stock_effect.selections[0]!.included_quantity = '7.'
  copy.data.stock_effect.selections[0]!.source.source_key.id = version
  copy.data.stock_effect.selections.pop()
  assert.equal(JSON.stringify(original), before)
})

test('reconciled receipt pending freezes selection order quantity and source references', () => {
  const original = reconciledReceipt()
  const pending = businessPending('revise', id, version, state, original as unknown as BusinessPayload, null, () => id)
  const before = JSON.stringify(pending)
  original.data.stock_effect.selections[0]!.included_quantity = '0'
  original.data.stock_effect.selections[0]!.source.source_key.id = version
  original.data.stock_effect.selections.reverse()
  assert.equal(JSON.stringify(pending), before)
})

test('reconciled receipt recovery keeps unfinished selections and the original metadata', () => {
  const original = reconciledReceipt()
  const pending = businessPending('create', null, null, state, original as unknown as BusinessPayload, null, () => id)
  const raw = JSON.parse(JSON.stringify(pending))
  raw.body.payload.data.stock_effect.selections[0].source = { kind: 'all_available' }
  raw.body.payload.data.stock_effect.selections[1] = { observation_version_id: null, source: null, included_quantity: '0.' }
  assert.deepEqual(restoreBusinessPending(JSON.stringify(raw)), raw)
})

test('reconciled receipt accepts explicit zero selections but rejects widened selectors and row limits', () => {
  const pending = businessPending('create', null, null, state, reconciledReceipt() as unknown as BusinessPayload, null, () => id)
  const raw = JSON.parse(JSON.stringify(pending))
  raw.body.payload.data.stock_effect.selections = []
  assert.deepEqual(restoreBusinessPending(JSON.stringify(raw)), raw)
  const row = { observation_version_id: version, source: { kind: 'all_available' }, included_quantity: '0' }
  raw.body.payload.data.stock_effect.selections = [row]
  assert.deepEqual(restoreBusinessPending(JSON.stringify(raw)), raw)
  for (const bad of [
    { ...row, source: { kind: 'all_available', source_key: { kind: 'receipt_inflow', id, position: 0 } } },
    { ...row, source: { kind: 'source', source_key: { kind: 'receipt_inflow', id, position: 1 } } },
    { ...row, source: { kind: 'source', source_key: { kind: 'unknown', id, position: 0 } } },
    { ...row, source_version_id: id },
    { ...row, included_quantity: '1\u0080' },
  ]) {
    raw.body.payload.data.stock_effect.selections = [bad]
    assert.equal(restoreBusinessPending(JSON.stringify(raw)), null)
  }
  raw.body.payload.data.stock_effect.selections = Array.from({ length: 21 }, () => row)
  assert.equal(restoreBusinessPending(JSON.stringify(raw)), null)
})
