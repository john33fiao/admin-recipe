import test from 'node:test'
import assert from 'node:assert/strict'
import { blankPriceDraft, businessPending, confirmPending, restoreBusinessPending, validateBusinessDraft } from '../app/utils/business.ts'

const id = '767bfaab-d964-4cfb-a4e6-b79ada5755aa'
const head = 'ad100fb4-3d59-4cd0-a5ee-06697d6bd244'
const epoch = '65ecea4e-74a0-430e-a6d2-acaa533e9b62'
const state = { epoch, revision: '9007199254740993' }

test('unfinished price input is a storable draft and stays an exact string', () => {
  const payload = blankPriceDraft()
  payload.data.quantity = '1.'
  payload.data.amount = '0'
  assert.deepEqual(validateBusinessDraft(payload), {})
  const pending = businessPending('create', null, null, state, payload, null, () => id)
  assert.deepEqual(restoreBusinessPending(JSON.stringify(pending)), pending)
  payload.data.amount = '10'
  assert.equal(pending.body.payload.data.amount, '0')
  assert.equal(pending.body.payload.data.quantity, '1.')
  assert.equal(restoreBusinessPending(JSON.stringify({ ...pending, body: { ...pending.body, confirmed: true } })), null)
  assert.ok(validateBusinessDraft({ ...payload, data: { ...payload.data, amount: '0'.repeat(81) } }).amount)
  assert.ok(validateBusinessDraft({ ...payload, data: { ...payload.data, note: '\u0085' } }).note)
})

test('confirmation copies only the displayed preview state and head', () => {
  const preview = { record_id: id, head_id: head, state, can_confirm: true, issues: [], effect: { kind: 'catalog_price' as const, material_id: id, material_version_id: head, quantity: '1000', amount: '0', unit: 'g', corrects_confirmation_id: null, previous_price: null, corrected_price: null } }
  const pending = confirmPending(preview, () => head)
  assert.deepEqual(pending.body, { meta: { request_id: head, epoch, expected_revision: state.revision }, expected_head_id: head })
  assert.deepEqual(restoreBusinessPending(JSON.stringify(pending)), pending)
  assert.throws(() => confirmPending({ ...preview, can_confirm: false }, () => head), /invalid-preview/)
  assert.throws(() => confirmPending({ ...preview, effect: null }, () => head), /invalid-preview/)
})

test('recovered business requests retain action, target and nested metadata', () => {
  const payload = blankPriceDraft()
  const pending = businessPending('revise', id, head, state, payload, null, () => id)
  assert.deepEqual(restoreBusinessPending(JSON.stringify(pending)), pending)
  assert.equal(restoreBusinessPending(JSON.stringify({ ...pending, record_id: null })), null)
  assert.equal(restoreBusinessPending(JSON.stringify({ ...pending, body: { ...pending.body, meta: { ...pending.body.meta, expected_revision: 2 } } })), null)
  assert.equal(restoreBusinessPending(JSON.stringify({ ...pending, body: { ...pending.body, payload: { ...payload, version: 2 } } })), null)
})
