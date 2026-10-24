import test from 'node:test'
import assert from 'node:assert/strict'
import type { SalePayload } from '../app/types/sale.ts'
import { blankSaleDraft } from '../app/utils/sale.ts'
import { displayDecimal } from '../app/utils/catalog.ts'
import { businessPending, restoreBusinessPending, validateBusinessDraft, copyPayload, samePayload } from '../app/utils/business.ts'

const id = '11111111-1111-4111-8111-111111111111'
const target = '22222222-2222-4222-8222-222222222222'
const payload = (): SalePayload => ({ kind: 'sale', version: 1, data: { material_id: id, material_version_id: id, recipe_version_id: id, count: '0', note: '판매 개수 정정' } })
test('sale correction pending restores exact count and original confirmation without changing metadata', () => {
  const pending = businessPending('create', null, null, { epoch: id, revision: '9007199254740993' }, payload(), target, () => id)
  assert.deepEqual(restoreBusinessPending(JSON.stringify(pending)), pending)
})


test('sale draft retains unfinished and zero counts without imposing confirmation policy', () => {
  const draft = blankSaleDraft()
  assert.deepEqual(validateBusinessDraft(draft), {})
  for (const count of ['0', '', '3.', '1000000001']) { draft.data.count = count; assert.deepEqual(validateBusinessDraft(draft), {}) }
  draft.data.note = '가상 정정 이유\n줄바꿈'
  assert.deepEqual(validateBusinessDraft(draft), {})
  draft.data.note = '\u0085'
  assert.ok(validateBusinessDraft(draft).note)
  draft.data.note = '가'.repeat(2001)
  assert.ok(validateBusinessDraft(draft).note)
})

test('sale copies keep frozen references and distinguish count edits without number rounding', () => {
  const original = payload(), changed = copyPayload(original)
  changed.data.count = '1'
  assert.equal(original.data.count, '0')
  assert.equal(samePayload(original, changed), false)
  assert.equal(changed.data.recipe_version_id, original.data.recipe_version_id)
  assert.equal(displayDecimal('999999999999999999999999.123456'), '999,999,999,999,999,999,999,999.123456')
})
