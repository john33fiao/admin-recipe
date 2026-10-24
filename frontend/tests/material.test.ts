import test from 'node:test'
import assert from 'node:assert/strict'
import { normalizeMaterial, prepareRequest, restorePending, validateMaterial } from '../app/utils/material.ts'

const requestId = '767bfaab-d964-4cfb-a4e6-b79ada5755aa'

test('material validation accepts arbitrary units and counts Unicode characters', () => {
  assert.deepEqual(normalizeMaterial({ name: '  가상 재료  ', unit: '  묶음  ' }), { name: '가상 재료', unit: '묶음' })
  assert.deepEqual(validateMaterial({ name: '🥕'.repeat(100), unit: '장' }), {})
  assert.ok(validateMaterial({ name: '🥕'.repeat(101), unit: '장' }).name)
  assert.ok(validateMaterial({ name: '재료', unit: '가'.repeat(21) }).unit)
  assert.ok(validateMaterial({ name: '재료\u0000', unit: 'g' }).name)
  assert.ok(validateMaterial({ name: '재료', unit: '' }).unit)
})

test('an ambiguous retry and refresh keep exactly the same request identity', () => {
  const first = prepareRequest({ name: ' 가상 재료 ', unit: 'g' }, null, () => requestId)
  const restored = restorePending(JSON.stringify(first))
  assert.deepEqual(restored, first)
  assert.equal(prepareRequest(first, first, () => assert.fail('must not generate another key')), first)
  assert.deepEqual(prepareRequest(first, restored, () => assert.fail('must reuse restored key')), first)
  assert.throws(() => prepareRequest({ name: '다른 재료', unit: 'g' }, restored, () => requestId), /changed-pending-request/)
  const unicodeRequest = prepareRequest({ name: '🥕'.repeat(100), unit: '🥣'.repeat(20) }, null, () => requestId)
  assert.deepEqual(restorePending(JSON.stringify(unicodeRequest)), unicodeRequest)
})

test('invalid or tampered session drafts cannot become write requests', () => {
  for (const value of [null, 'invalid', 'null', '{}', JSON.stringify({ request_id: requestId, name: '', unit: 'g' }), JSON.stringify({ request_id: 'bad-id', name: '재료', unit: 'g' })]) {
    assert.equal(restorePending(value), null)
  }
  assert.throws(() => prepareRequest({ name: ' ', unit: 'g' }, null, () => requestId), /invalid-material/)
})
