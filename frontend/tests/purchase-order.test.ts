import test from 'node:test'
import assert from 'node:assert/strict'
import { businessPending, copyPayload, restoreBusinessPending, samePayload, validateBusinessDraft } from '../app/utils/business.ts'
import type { BusinessPayload } from '../app/types/business.ts'
import type { OrderPayload } from '../app/types/purchase-order.ts'
import { orderDraftIdentity } from '../app/utils/purchase-order.ts'

const id = '767bfaab-d964-4cfb-a4e6-b79ada5755aa'
const version = 'ad100fb4-3d59-4cd0-a5ee-06697d6bd244'
const state = { epoch: '65ecea4e-74a0-430e-a6d2-acaa533e9b62', revision: '9007199254740993' }
const order = { kind: 'purchase_order', version: 1, data: { action: 'place', material_id: null, material_version_id: null, quantity: '1.', unit: 'kg', supplier: '', expected_date: '2028-0', estimated_amount: '', note: '' } } as unknown as BusinessPayload

test('order drafts preserve their tagged kind and unfinished optional values', () => {
  const copy = copyPayload(order)
  assert.deepEqual(copy, order)
  assert.notEqual(copy.data, order.data)
  assert.deepEqual(validateBusinessDraft(order), {})
  const pending = businessPending('create', null, null, state, order, null, () => id)
  assert.deepEqual(restoreBusinessPending(JSON.stringify(pending)), pending)
})

test('revise and cancel notes are optional and incompatible fields cannot be recovered', () => {
  const revised = { kind: 'purchase_order', version: 1, data: { action: 'revise', order_id: id, expected_order_version_id: version, material_version_id: version, quantity: '12', unit: 'kg', supplier: '', expected_date: '', estimated_amount: '0', note: '' } } as unknown as BusinessPayload
  const cancelled = { kind: 'purchase_order', version: 1, data: { action: 'cancel', order_id: id, expected_order_version_id: version, note: '' } } as unknown as BusinessPayload
  for (const payload of [revised, cancelled]) {
    assert.deepEqual(validateBusinessDraft(payload), {})
    const pending = businessPending('create', null, null, state, payload, null, () => id)
    assert.deepEqual(restoreBusinessPending(JSON.stringify(pending)), pending)
    assert.equal(restoreBusinessPending(JSON.stringify({ ...pending, body: { ...pending.body, payload: { ...payload, data: { ...payload.data, material_id: id } } } })), null)
  }
  assert.equal(samePayload(revised, cancelled), false)
})

test('legacy price pending preserves the exact wire while order payloads reject unknown actions', () => {
  const price = { kind: 'catalog_price', version: 1, data: { material_id: id, material_version_id: version, quantity: '100', unit: 'g', amount: '0', note: '' } } as BusinessPayload
  const pending = businessPending('create', null, null, state, price, null, () => id)
  assert.equal(JSON.stringify(restoreBusinessPending(JSON.stringify(pending))), JSON.stringify(pending))
  const bad = { ...pending, body: { ...pending.body, payload: { kind: 'purchase_order', version: 1, data: { action: 'receive', order_id: id, expected_order_version_id: version, note: '' } } } }
  assert.equal(restoreBusinessPending(JSON.stringify(bad)), null)
})

test('a new order head starts a fresh editor while an unresolved original request keeps its identity', () => {
  const first: OrderPayload = { kind: 'purchase_order', version: 1, data: { action: 'cancel', order_id: id, expected_order_version_id: version, note: '' } }
  const second: OrderPayload = { ...first, data: { ...first.data, expected_order_version_id: state.epoch } as typeof first.data }
  const oldIdentity = orderDraftIdentity(first, [])
  assert.notEqual(orderDraftIdentity(second, []), oldIdentity)
  const pending = businessPending('create', null, null, state, first, null, () => id)
  assert.equal(orderDraftIdentity(second, [[`sunchang.business.${oldIdentity}.pending.v1`, JSON.stringify(pending)]]), oldIdentity)
  assert.equal(orderDraftIdentity(second, [[`sunchang.business.${oldIdentity}.draft.v1`, JSON.stringify({ payload: first, state, headId: null })]]), oldIdentity)
  assert.notEqual(orderDraftIdentity(second, [[`sunchang.business.${oldIdentity}.pending.v1`, '{broken']]), oldIdentity)
})
