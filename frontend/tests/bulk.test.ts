import test from 'node:test'
import assert from 'node:assert/strict'
import type { Material } from '../app/types/material.ts'
import { makeBulkRow, reviewBulk, restoreBulkSession, bulkFingerprint, setBulkCell } from '../app/utils/bulk.ts'
const id = '11111111-1111-4111-8111-111111111111', version = '22222222-2222-4222-8222-222222222222', next = '33333333-3333-4333-8333-333333333333'
const material: Material = { id, version_id: version, name: '가상 재료', unit: 'g', canonical_unit: 'g', dimension: 'mass', kind: 'raw', description: '', archived: false, created_at: '', updated_at: '', price: { source: 'missing', source_id: null, amount: null, quantity: null, reason: null } }
const context = { state: { epoch: id, revision: '1' }, materials: [material] }
test('menu group creates one recipe with explicit zero and all component versions frozen', () => {
  const values = { group: 'A', name: '가상 메뉴', unit: '개', description: '', basis_count: '3', selling_price: '0', material_id: id, quantity: '100', component_unit: 'g', note: '' }
  const rows = [makeBulkRow('menu', values, () => id), makeBulkRow('menu', { ...values, quantity: '0' }, () => next)]
  const review = reviewBulk('menu', rows, context)
  assert.deepEqual(review.errors, {})
  assert.equal(review.items.length, 1)
  const command = review.items[0]!.command
  assert.equal(command.kind, 'menu')
  if (command.kind !== 'menu') return
  assert.equal(command.data.selling_price, '0'); assert.equal(command.data.basis_count, '3')
  assert.deepEqual(command.data.lines.map(line => [line.material_id, line.material_version_id, line.quantity]), [[id, version, '100'], [id, version, '0']])
  assert.ok(reviewBulk('menu', rows, { ...context, materials: [{ ...material, version_id: next }] }).errors[id]!.material_id)
})
test('blank prices remain absent, group disagreement and incompatible units block every write', () => {
  const values = { group: 'A', name: '메뉴', unit: '개', basis_count: '1', selling_price: '' }
  const rows = [makeBulkRow('menu', values, () => id)]
  const result = reviewBulk('menu', rows, context)
  assert.deepEqual(result.errors, {}); assert.equal(result.items[0]!.command.kind === 'menu' && result.items[0]!.command.data.selling_price, null)
  const mismatch = [...rows, makeBulkRow('menu', { ...values, name: '다른 이름' }, () => next)]
  assert.ok(reviewBulk('menu', mismatch, context).errors[next]!.name)
  const bad = makeBulkRow('menu', { ...values, material_id: id, quantity: '1', component_unit: 'L' }, () => next)
  assert.ok(reviewBulk('menu', [bad], context).errors[next]!.component_unit)
})
test('receipts require positive quantity, explicit amount and stock decision; inclusion zero is not invented', () => {
  const row = makeBulkRow('receipt', { material_id: id, quantity: '100', unit: 'g', amount: '0', receipt_date: '2026-10-20', stock_effect: 'unreflected' }, () => next)
  const review = reviewBulk('receipt', [row], context)
  assert.deepEqual(review.errors, {})
  assert.equal(review.items[0]!.command.kind === 'receipt' && review.items[0]!.command.payload.data.amount, '0')
  row.values.amount = ''; row.values.quantity = '0'; row.values.stock_effect = 'reconciled'
  const errors = reviewBulk('receipt', [row], context).errors[next]!
  assert.ok(errors.amount); assert.ok(errors.quantity); assert.ok(errors.stock_effect)
})
test('new import fingerprints keep exact values and strict tab recovery rejects forged pending data', () => {
  const row = makeBulkRow('material', { name: '가상', unit: 'g', kind: 'raw' }, () => next)
  const review = reviewBulk('material', [row], context)
  const session = { version: 1, kind: 'material', rows: [row], review, fingerprint: '', acceptedWarnings: false, duplicate: false }
  assert.ok(restoreBulkSession(JSON.stringify(session)))
  assert.notEqual(bulkFingerprint('material', [row]), bulkFingerprint('material', [{ ...row, id, values: { ...row.values, name: ' 가상 ' } }]))
  assert.equal(restoreBulkSession(JSON.stringify({ ...session, rows: [{ ...row, values: { ...row.values, injected: 'x' } }] })), null)
  const poisoned = structuredClone(session)
  poisoned.review.items[0]!.pending = { kind: 'material', body: { request_id: id, epoch: id, expected_revision: '1', name: '다른 내용', kind: 'raw', unit: 'g', description: '' } }
  assert.equal(restoreBulkSession(JSON.stringify(poisoned)), null)
})
test('receipt note whitespace remains a different upload instead of replacing it with an old result', () => {
  const row = makeBulkRow('receipt', { note: '가상 메모' }, () => next)
  const revised = { ...row, id, values: { ...row.values, note: ' 가상 메모 ' } }
  assert.notEqual(bulkFingerprint('receipt', [row]), bulkFingerprint('receipt', [revised]))
})
test('explicit reference reselection clears its old version without adding receipt columns to menu rows', () => {
  const row = makeBulkRow('menu', { group: 'A', name: '가상', material_id: id }, () => next)
  row.reference = { material_id: id, material_version_id: version }
  setBulkCell('menu', row, 'material_id', id)
  assert.equal(row.reference, null)
  assert.equal(Object.hasOwn(row.values, 'observation_version_id'), false)
  assert.equal(Object.hasOwn(row.values, 'included_quantity'), false)
})
test('new bulk materials cannot use legacy unclassified kind', () => {
  const row = makeBulkRow('material', { name: '가상', kind: 'unclassified' }, () => next)
  assert.ok(reviewBulk('material', [row], context).errors[next]?.kind)
})
test('v1 restore UUIDv8 references remain valid through menu review and uncertain receipt tab recovery', () => {
  const restoredVersion = '22222222-2222-8222-8222-222222222222'
  const restored = { ...context, materials: [{ ...material, version_id: restoredVersion }] }
  const menu = makeBulkRow('menu', { group: 'A', name: '가상 복원 구성', material_id: id, quantity: '1', component_unit: 'g' }, () => next)
  assert.deepEqual(reviewBulk('menu', [menu], restored).errors, {})
  const row = makeBulkRow('receipt', { material_id: id, quantity: '1', unit: 'g', amount: '0', receipt_date: '2026-10-20', stock_effect: 'unreflected' }, () => next)
  const review = reviewBulk('receipt', [row], restored)
  const command = review.items[0]!.command
  if (command.kind !== 'receipt') throw new Error('receipt required')
  review.items[0]!.pending = { kind: 'receipt', request: { action: 'create', record_id: null, body: { meta: { request_id: next, epoch: id, expected_revision: '1' }, payload: command.payload, corrects_confirmation_id: null } } }
  review.items[0]!.recovery = 'retry'
  const session = { version: 1, kind: 'receipt', rows: [row], review, fingerprint: '', acceptedWarnings: false, duplicate: false }
  const recovered = restoreBulkSession(JSON.stringify(session))
  assert.equal(recovered?.rows[0]!.reference?.material_version_id, restoredVersion)
  assert.deepEqual(recovered?.review?.items[0]!.pending, review.items[0]!.pending)
})
test('invalid long CSV cells survive review and reload for editing instead of becoming corrupt requests', () => {
  const row = makeBulkRow('material', { name: '가상 오류 수정', description: '가'.repeat(11_000) }, () => next)
  const review = reviewBulk('material', [row], context)
  assert.ok(review.errors[next]!.description)
  const recovered = restoreBulkSession(JSON.stringify({ version: 1, kind: 'material', rows: [row], review, fingerprint: '', acceptedWarnings: false, duplicate: false }))
  assert.equal(recovered?.rows[0]!.values.description.length, 11_000)
  assert.ok(recovered?.review?.errors[next]!.description)
})
test('invalid menu quantities and receipt references remain editable after validation and reload', () => {
  for (const [kind, values] of [ ['menu', { group: 'A', name: '가상 오류', material_id: id, quantity: '', component_unit: 'g' }], ['receipt', { material_id: id, quantity: '1', unit: 'g', amount: '', receipt_date: 'bad date', stock_effect: 'reconciled', observation_version_id: 'bad reference', included_quantity: '1' }] ] as const) {
    const row = makeBulkRow(kind, values, () => next), review = reviewBulk(kind, [row], context)
    assert.ok(Object.keys(review.errors).length)
    const recovered = restoreBulkSession(JSON.stringify({ version: 1, kind, rows: [row], review, fingerprint: '', acceptedWarnings: false, duplicate: false }))
    assert.equal(recovered?.rows[0]!.values.quantity, values.quantity)
    assert.equal(recovered?.review?.items[0]!.pending, null)
  }
})
