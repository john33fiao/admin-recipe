import test from 'node:test'
import assert from 'node:assert/strict'
import { decimalInput, failureAction, prepareCatalogRegistration, restoreCatalogRegistration, restoreMaterialMutation, validateCatalogInput, validatePriceInput } from '../app/utils/catalog.ts'

const requestId = '767bfaab-d964-4cfb-a4e6-b79ada5755aa'
const versionId = 'ad100fb4-3d59-4cd0-a5ee-06697d6bd244'
const state = { epoch: '65ecea4e-74a0-430e-a6d2-acaa533e9b62', revision: '9007199254740993' }
const input = { name: '가상 재료', unit: 'g', kind: 'raw' as const, description: '가상 설명' }

test('decimal form values stay exact strings and reject silent rounding', () => {
  assert.equal(decimalInput(' 00012.340000 ', 'money'), '12.34')
  assert.equal(decimalInput('0', 'money'), '0')
  assert.equal(decimalInput('9000000000000', 'money'), '9000000000000')
  assert.equal(decimalInput('0.000001', 'quantity'), '0.000001')
  for (const value of ['', '-1', '1e3', '1,000', 'NaN', '0.0000001', '9000000000000.000001']) assert.equal(decimalInput(value, 'money'), null)
  assert.equal(validatePriceInput({ amount: '0', quantity: '0', unit: 'g' }).quantity, '가격 기준 수량은 0보다 커야 합니다.')
  assert.deepEqual(validatePriceInput({ amount: '0', quantity: '100', unit: 'g' }), {})
})

test('new registration replays its original metadata and old pending stays an old payload', () => {
  const request = prepareCatalogRegistration(input, state, null, () => requestId)
  assert.equal(request.expected_revision, state.revision)
  assert.deepEqual(restoreCatalogRegistration(JSON.stringify(request)), request)
  assert.equal(prepareCatalogRegistration(input, { ...state, revision: '9007199254740994' }, request, () => assert.fail('new key')), request)
  assert.throws(() => prepareCatalogRegistration({ ...input, description: '다름' }, state, request, () => requestId), /changed-pending/)
  const old = { request_id: requestId, name: input.name, unit: input.unit }
  assert.deepEqual(restoreCatalogRegistration(JSON.stringify(old)), old)
  assert.equal(restoreCatalogRegistration(JSON.stringify({ ...old, kind: 'raw' })), null)
  assert.equal(restoreCatalogRegistration(JSON.stringify({ ...request, expected_revision: 2 })), null)
})

test('mutation recovery preserves price decimals, version and immutable state', () => {
  const mutation = { action: 'price', body: { request_id: requestId, epoch: state.epoch, expected_revision: state.revision, expected_version_id: versionId, amount: '0', quantity: '100', unit: 'g' } }
  assert.deepEqual(restoreMaterialMutation(JSON.stringify(mutation)), mutation)
  assert.equal(restoreMaterialMutation(JSON.stringify({ ...mutation, body: { ...mutation.body, quantity: 100 } })), null)
  assert.equal(restoreMaterialMutation(JSON.stringify({ ...mutation, action: 'delete' })), null)
})

test('only a known stale-state response can release a request for reconfirmation', () => {
  assert.equal(failureAction(409, 'stale_state'), 'refresh')
  assert.equal(failureAction(409, 'request_conflict'), 'blocked')
  assert.equal(failureAction(409), 'blocked')
  assert.equal(failureAction(503, 'unavailable'), 'retry')
  assert.equal(failureAction(400, 'validation'), 'edit')
  assert.ok(validateCatalogInput({ ...input, description: 'a'.repeat(2001) }).description)
  assert.deepEqual(validateCatalogInput({ ...input, description: '첫 줄\n다음 줄' }), {})
})
