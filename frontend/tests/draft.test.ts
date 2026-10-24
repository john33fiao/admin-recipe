import test from 'node:test'
import assert from 'node:assert/strict'
import { restoreMaterialDraft } from '../app/utils/draft.ts'

test('a tab draft retains unfinished input without becoming a write request', () => {
  assert.deepEqual(restoreMaterialDraft(JSON.stringify({ name: '  가상 재료 ', unit: '' })), { name: '  가상 재료 ', unit: '' })
  assert.deepEqual(restoreMaterialDraft(JSON.stringify({ name: '', unit: '묶음' })), { name: '', unit: '묶음' })
  assert.deepEqual(restoreMaterialDraft(JSON.stringify({ name: '가상 재료', unit: 'g', request_id: 'ignored' })), { name: '가상 재료', unit: 'g' })
})

test('malformed tab data cannot populate the material form', () => {
  for (const raw of [null, '', 'invalid', 'null', '[]', '{}', '{"name":4,"unit":"g"}', '{"name":"재료","unit":null}']) {
    assert.equal(restoreMaterialDraft(raw), null)
  }
})
