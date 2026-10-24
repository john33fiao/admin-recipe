import test from 'node:test'
import assert from 'node:assert/strict'
import { advanceWorkspaceState, blankWorkspacePayload, linkWorkspaceRecord, newWorkspaceEntry, restoreWorkspaceTab, unresolvedWorkspaceRequest, workspaceStatus } from '../app/utils/workspace.ts'
import { businessPending, copyPayload, restoreBusinessPending } from '../app/utils/business.ts'

const a = '00000000-0000-4000-8000-000000000001', b = '00000000-0000-4000-8000-000000000002', c = '00000000-0000-4000-8000-000000000003'
test('local workspace items remain independent and only actual success links a record without changing selection', () => {
  const one = newWorkspaceEntry('sale', () => a), two = newWorkspaceEntry('sale', () => b)
  const tab = { entries: [one, two], selected: two.identity }
  const linked = linkWorkspaceRecord(tab, one.identity, c)
  assert.equal(linked.entries[0]!.identity, c); assert.equal(linked.entries[0]!.recordId, c)
  assert.equal(linked.selected, two.identity); assert.equal(tab.entries[0]!.recordId, null)
  assert.deepEqual(restoreWorkspaceTab(JSON.stringify(linked)), linked)
  assert.equal(restoreWorkspaceTab(JSON.stringify({ entries: [one, one], selected: one.identity })), null)
  assert.equal(restoreWorkspaceTab(JSON.stringify({ entries: [{ ...one, recordId: c }], selected: null })), null)
})
test('workspace nested blank inputs preserve unchecked versus zero and immutable pending in restoration', () => {
  const input = blankWorkspacePayload('stocktake')
  assert.equal(input.kind, 'stocktake'); if (input.kind !== 'stocktake') return
  input.data.rows.push({ ...input.data.rows[0]!, quantity: '0' })
  const other = copyPayload(input), pending = businessPending('create', null, null, { epoch: a, revision: '2' }, input, null, () => b)
  other.data.rows[0]!.quantity = '5'; other.data.rows[1]!.initial_value = { kind: 'manual_amount', amount: '0' }
  assert.equal(input.data.rows[0]!.quantity, ''); assert.equal(input.data.rows[1]!.quantity, '0')
  assert.deepEqual(restoreBusinessPending(JSON.stringify(pending)), pending)
  assert.equal((pending.body as any).payload.data.rows[0].quantity, '')
})
test('replayed state never moves current workspace revision or epoch backwards', () => {
  const current = { epoch: a, revision: '30' }
  assert.equal(advanceWorkspaceState(current, { epoch: a, revision: '1' }), current)
  assert.equal(advanceWorkspaceState(current, { epoch: b, revision: '90' }), current)
  assert.deepEqual(advanceWorkspaceState(current, { epoch: a, revision: '31' }), { epoch: a, revision: '31' })
})
test('partial workspace results keep uncertain, unsaved and successful records distinct', () => {
  const uncertain = { payload: blankWorkspacePayload('sale'), status: 'draft' as const, busy: false, dirty: true, pendingAction: 'create' as const, recovery: 'retry', failure: 'uncertain', issueCount: 0, previewState: null }
  assert.equal(unresolvedWorkspaceRequest(uncertain), true)
  assert.equal(workspaceStatus(uncertain, undefined, null), '처리 결과 확인 필요')
  assert.equal(workspaceStatus(undefined, 'draft', a), '임시 저장됨')
  assert.equal(workspaceStatus(undefined, undefined, null), '이 탭에만 입력')
  assert.equal(unresolvedWorkspaceRequest({ ...uncertain, recovery: 'refresh' }), false)
})
