import test from 'node:test'
import assert from 'node:assert/strict'
import type { BusinessPayload } from '../app/types/business.ts'
import { businessPending, copyPayload, restoreBusinessPending } from '../app/utils/business.ts'
import { correctionDraftIdentity } from '../app/utils/correction-draft.ts'
import type { CorrectionSeed } from '../app/types/correction.ts'

const id = '767bfaab-d964-4cfb-a4e6-b79ada5755aa'
const version = 'ad100fb4-3d59-4cd0-a5ee-06697d6bd244'
const state = { epoch: '65ecea4e-74a0-430e-a6d2-acaa533e9b62', revision: '9007199254740993' }
function correction() {
  return { kind: 'correction', version: 1, data: {
    expected_target_version_id: null, reason: '', action: { kind: 'replace', replacement: { kind: 'stocktake', data: { rows: [
      { observation_id: id, expected_version_id: version, action: { kind: 'replace', quantity: '0.', unit: 'g', initial_value: { kind: 'manual_amount', amount: '' }, excess: null } },
    ] } } },
    observation_reviews: [{ observation_id: null, expected_version_id: null, initial_value: null, excess: { reviewed: false, reason: '', value: { kind: 'manual_amount', amount: '0' } } }],
    linked_receipt_changes: [{ receipt_id: id, expected_version_id: version, action: { kind: 'replace', data: {
      source: { kind: 'direct', material_id: id, material_version_id: version }, quantity: '1.', unit: 'kg', amount: '', receipt_date: '', note: '',
      stock_effect: { kind: 'reconciled', selections: [
        { observation_version_id: version, source: { kind: 'source', source_key: { kind: 'observation_initial', id, position: 0 } }, included_quantity: '0.' },
        { observation_version_id: null, source: null, included_quantity: '' },
      ] },
    } } }],
  } }
}

test('correction copy owns replacement rows reviews and linked receipt selectors', () => {
  const original = correction()
  const before = JSON.stringify(original)
  const copy = copyPayload(original as unknown as BusinessPayload) as unknown as ReturnType<typeof correction>
  copy.data.action.replacement.data.rows[0]!.action.initial_value.amount = '1'
  copy.data.observation_reviews[0]!.excess.value.amount = '2'
  copy.data.linked_receipt_changes[0]!.action.data.stock_effect.selections[0]!.source!.source_key.id = version
  copy.data.linked_receipt_changes[0]!.action.data.stock_effect.selections.reverse()
  assert.equal(JSON.stringify(original), before)
})

test('correction pending owns the entire original body including linked receipt order', () => {
  const original = correction()
  const pending = businessPending('revise', id, version, state, original as unknown as BusinessPayload, version, () => id)
  const before = JSON.stringify(pending)
  original.data.action.replacement.data.rows[0]!.action.quantity = '5'
  original.data.observation_reviews[0]!.excess.reason = 'later edit'
  original.data.linked_receipt_changes[0]!.action.data.stock_effect.selections.reverse()
  assert.equal(JSON.stringify(pending), before)
})

test('correction recovery preserves unfinished replacement inputs and the original outer target', () => {
  const pending = businessPending('create', null, null, state, correction() as unknown as BusinessPayload, version, () => id)
  assert.deepEqual(restoreBusinessPending(JSON.stringify(pending)), pending)
})

test('correction recovery preserves explicit void without inventing a zero replacement', () => {
  const payload = { kind: 'correction', version: 1, data: { expected_target_version_id: version, reason: '', action: { kind: 'void' }, observation_reviews: [], linked_receipt_changes: [] } }
  const pending = businessPending('create', null, null, state, payload as unknown as BusinessPayload, version, () => id)
  assert.deepEqual(restoreBusinessPending(JSON.stringify(pending)), pending)
})

test('correction restore rejects widened actions and future reference fields without limiting affected reviews', () => {
  const pending = businessPending('create', null, null, state, correction() as unknown as BusinessPayload, version, () => id)
  const many = JSON.parse(JSON.stringify(pending))
  many.body.payload.data.observation_reviews = Array.from({ length: 101 }, () => many.body.payload.data.observation_reviews[0])
  many.body.payload.data.linked_receipt_changes = Array.from({ length: 21 }, () => many.body.payload.data.linked_receipt_changes[0])
  assert.deepEqual(restoreBusinessPending(JSON.stringify(many)), many)
  for (const change of [
    (data: any) => { data.target_confirmation_id = id },
    (data: any) => { data.action = { kind: 'void', replacement: data.action.replacement } },
    (data: any) => { data.observation_reviews[0].quantity = '0' },
    (data: any) => { data.linked_receipt_changes[0].action.data.stock_effect.selections[0].source_version_id = id },
    (data: any) => { data.action.replacement.data.rows = Array.from({ length: 101 }, () => data.action.replacement.data.rows[0]) },
    (data: any) => { data.reason = 'x'.repeat(2001) },
  ]) {
    const malformed = JSON.parse(JSON.stringify(pending)); change(malformed.body.payload.data)
    assert.equal(restoreBusinessPending(JSON.stringify(malformed)), null)
  }
})

test('correction draft identity preserves an already sent target version and starts fresh after completion', () => {
  const seed = { original: { reference: { original_confirmation_id: version } }, current: { reference: { version_id: id } } } as CorrectionSeed
  const pending = businessPending('create', null, null, state, correction() as unknown as BusinessPayload, version, () => id)
  const oldIdentity = `new.correction.${version}.${version}`
  assert.equal(correctionDraftIdentity(seed, [[`sunchang.business.${oldIdentity}.pending.v1`, JSON.stringify(pending)]]), oldIdentity)
  assert.equal(correctionDraftIdentity(seed, []), `new.correction.${version}.${id}`)
  const wrong = { ...pending, body: { ...pending.body, corrects_confirmation_id: id } }
  assert.equal(correctionDraftIdentity(seed, [[`sunchang.business.${oldIdentity}.pending.v1`, JSON.stringify(wrong)]]), `new.correction.${version}.${id}`)
})
