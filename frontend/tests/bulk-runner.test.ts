import test from 'node:test'
import assert from 'node:assert/strict'
import type { BulkReview } from '../app/types/bulk.ts'
import type { ReceiptBusinessEffect } from '../app/types/receipt.ts'
import type { BusinessDetail } from '../app/types/business.ts'
import { runBulk, inspectBulkDraft, applyBulkDraft } from '../app/utils/bulk-runner.ts'
import { makeBulkRow, reviewBulk, restoreBulkSession } from '../app/utils/bulk.ts'
const epoch = '11111111-1111-4111-8111-111111111111', head = '22222222-2222-4222-8222-222222222222', target = '33333333-3333-4333-8333-333333333333'
function materials() { return reviewBulk('material', [makeBulkRow('material', { name: '가상1' }, () => head), makeBulkRow('material', { name: '가상2' }, () => target)], { state: { epoch, revision: '0' }, materials: [] }) }
test('lost response survives tab recovery and replay never duplicates a successful material', async () => {
  let writes = 0, lost = false, saved = ''
  const ledger = new Map<string, unknown>(), requests: unknown[] = []
  const transport = async <T>(_path: string, body?: unknown): Promise<T> => {
    const input = body as { request_id: string; name: string }; requests.push(structuredClone(input))
    if (ledger.has(input.request_id)) return ledger.get(input.request_id) as T
    writes++
    const result = { id: writes === 1 ? head : target, epoch, revision: `${writes}`, name: input.name }
    ledger.set(input.request_id, result)
    if (!lost) { lost = true; throw new Error('response lost after durable write') }
    return result as T
  }
  let review = materials()
  const persist = () => { saved = JSON.stringify({ version: 1, kind: 'material', rows: [makeBulkRow('material', { name: '가상1' }, () => head), makeBulkRow('material', { name: '가상2' }, () => target)], review, fingerprint: '', acceptedWarnings: true, duplicate: false }) }
  assert.equal(await runBulk(review, transport, persist, () => crypto.randomUUID()), false)
  assert.equal(writes, 1); assert.equal(review.items[0]!.recovery, 'retry')
  review = restoreBulkSession(saved)!.review!
  assert.equal(await runBulk(review, transport, persist, () => crypto.randomUUID()), true)
  assert.equal(writes, 2); assert.equal(review.items.filter(item => item.result).length, 2)
  assert.deepEqual(requests[0], requests[1]); assert.equal(review.items[0]!.result!.href, `/materials/${head}`)
})
test('storage failure before an immutable request is saved causes zero network writes', async () => {
  const review = materials(); let writes = 0
  assert.equal(await runBulk(review, async <T>() => { writes++; return {} as T }, () => { throw new Error('full') }), false)
  assert.equal(writes, 0); assert.equal(review.items[0]!.recovery, 'retry')
})
test('restore generation already_processed preserves the old request and blocks automatic replay', async () => {
  const review = materials(); let calls = 0
  const transport = async <T>(): Promise<T> => { calls++; throw Object.assign(new Error('restored'), { status: 409, code: 'stale_generation', requestStatus: 'already_processed' }) }
  assert.equal(await runBulk(review, transport, () => {}), false)
  const pending = structuredClone(review.items[0]!.pending)
  assert.equal(await runBulk(review, transport, () => {}), false)
  assert.equal(calls, 1); assert.equal(review.items[0]!.recovery, 'blocked'); assert.deepEqual(review.items[0]!.pending, pending)
})
test('receipt effect change stops before confirmation and explicit review resumes the stored draft', async () => {
  const effect: ReceiptBusinessEffect = { kind: 'receipt', material_id: target, material_version_id: head, quantity: '10', amount: '0', unit: 'g', input_quantity: '10', input_unit: 'g', receipt_date: '2026-10-20', before: null, after: { quantity: '10', value: '0', source_id: null }, basis_before: null, observation: null, order_before: null, order_after: null }
  const changed = { ...effect, before: { quantity: '10', value: '0', source_id: head }, after: { quantity: '20', value: '0', source_id: head } }
  const row = makeBulkRow('receipt', { material_id: target, quantity: '10', unit: 'g', amount: '0', receipt_date: '2026-10-20', stock_effect: 'unreflected' }, () => head)
  const review: BulkReview = { state: { epoch, revision: '0' }, errors: {}, warnings: [], items: [{ id: head, rowIds: [head], label: '가상', command: { kind: 'receipt', payload: { kind: 'receipt', version: 1, data: { source: { kind: 'direct', material_id: target, material_version_id: head }, quantity: '10', unit: 'g', amount: '0', receipt_date: '2026-10-20', note: '', stock_effect: { kind: 'unreflected' } } } }, effect, preview: null, draft: null, result: null, pending: null, recovery: '', failure: '' }] }
  let creates = 0, confirms = 0
  const transport = async <T>(path: string): Promise<T> => {
    if (path.endsWith('/drafts')) { creates++; return { state: { epoch, revision: '1' }, result: { item: { id: target }, head: { id: head } } } as T }
    if (path.endsWith('/preview')) return { record_id: target, head_id: head, state: { epoch, revision: '1' }, can_confirm: true, issues: [], effect: changed } as T
    confirms++; return { state: { epoch, revision: '2' }, result: { confirmation: { result: { receipt_id: epoch } } } } as T
  }
  assert.equal(await runBulk(review, transport, () => {}), false)
  assert.equal(creates, 1); assert.equal(confirms, 0); assert.equal(review.items[0]!.recovery, 'effect')
  review.items[0]!.effect = changed; review.items[0]!.recovery = ''
  assert.equal(await runBulk(review, transport, () => {}), true)
  assert.equal(creates, 1); assert.equal(confirms, 1); assert.equal(review.items[0]!.result!.href, `/receipts/${epoch}`)
  assert.ok(row)
})
test('explicit draft reinspection follows a matching new head, flags changed input and links external confirmation', () => {
  const payload = { kind: 'receipt' as const, version: 1 as const, data: { source: { kind: 'direct' as const, material_id: target, material_version_id: head }, quantity: '10', unit: 'g', amount: '0', receipt_date: '2026-10-20', note: '', stock_effect: { kind: 'unreflected' as const } } }
  const entry: BulkReview['items'][number] = { id: target, rowIds: [target], label: '가상', command: { kind: 'receipt', payload }, effect: null, preview: null, draft: { id: target, head_id: head }, result: null, pending: null, recovery: 'review', failure: '' }
  const detail: BusinessDetail = { item: { id: target, kind: 'receipt', head_id: epoch, status: 'draft', created_at: '', updated_at: '', corrects_confirmation_id: null, confirmation_id: null, effective_confirmation_id: null, issue_count: 0, material_name: '가상' }, head: { id: epoch, record_id: target, previous_revision_id: head, status: 'draft', payload, recorded_at: '', seq: '4' }, revisions: [], confirmations: [], corrections: [], issues: [], state: { epoch, revision: '4' } }
  assert.equal(inspectBulkDraft(entry, detail), 'unchanged'); assert.equal(entry.draft!.head_id, epoch)
  detail.head.id = head; detail.head.payload = { ...payload, data: { ...payload.data, amount: '20' } }
  assert.equal(inspectBulkDraft(entry, detail), 'changed'); assert.equal(entry.recovery, 'review'); assert.equal(entry.command.kind === 'receipt' && entry.command.payload.data.amount, '0')
  detail.item.status = 'confirmed'; detail.head.status = 'confirmed'; detail.confirmations = [{ id: epoch, record_id: target, revision_id: head, request_id: epoch, recorded_at: '', seq: '5', result: { kind: 'receipt', receipt_id: epoch, holding_version_id: head, material_id: target, material_version_id: head, order_id: null, order_version_id: null } }]
  assert.equal(inspectBulkDraft(entry, detail), 'confirmed'); assert.equal(entry.result!.href, `/receipts/${epoch}`)
  entry.result = null; entry.pending = { kind: 'receipt', request: { action: 'confirm', record_id: target, body: { meta: { request_id: epoch, epoch, expected_revision: '4' }, expected_head_id: head } } }
  const before = JSON.stringify(entry)
  assert.equal(inspectBulkDraft(entry, detail), 'pending'); assert.equal(JSON.stringify(entry), before)
})
test('an old external draft cannot replace a completed receipt row or its saved command and result', () => {
  const row = makeBulkRow('receipt', { material_id: target, quantity: '10', unit: 'g', amount: '7', receipt_date: '2026-10-20', stock_effect: 'unreflected' }, () => target)
  const payload = { kind: 'receipt' as const, version: 1 as const, data: { source: { kind: 'direct' as const, material_id: target, material_version_id: head }, quantity: '10', unit: 'g', amount: '7', receipt_date: '2026-10-20', note: '', stock_effect: { kind: 'unreflected' as const } } }
  const entry: BulkReview['items'][number] = { id: target, rowIds: [target], label: '가상 성공 입고', command: { kind: 'receipt', payload }, effect: null, preview: null, draft: { id: target, head_id: target }, result: { id: epoch, href: `/receipts/${epoch}` }, pending: null, recovery: '', failure: '' }
  const detail: BusinessDetail = { item: { id: target, kind: 'receipt', head_id: epoch, status: 'draft', created_at: '', updated_at: '', corrects_confirmation_id: null, confirmation_id: null, effective_confirmation_id: null, issue_count: 0, material_name: '가상' }, head: { id: epoch, record_id: target, previous_revision_id: head, status: 'draft', payload: { ...payload, data: { ...payload.data, amount: '5' } }, recorded_at: '', seq: '4' }, revisions: [], confirmations: [], corrections: [], issues: [], state: { epoch, revision: '4' } }
  const before = JSON.stringify({ row, entry }); let retained = 0
  assert.equal(applyBulkDraft(entry, row, detail, () => { retained++ }), false)
  assert.equal(JSON.stringify({ row, entry }), before); assert.equal(retained, 0)
  entry.result = null
  entry.pending = { kind: 'receipt', request: { action: 'confirm', record_id: target, body: { meta: { request_id: epoch, epoch, expected_revision: '4' }, expected_head_id: head } } }
  const pending = JSON.stringify({ row, entry })
  assert.equal(applyBulkDraft(entry, row, detail, () => { retained++ }), false)
  assert.equal(JSON.stringify({ row, entry }), pending); assert.equal(retained, 0)
  entry.pending = null
  const untouched = JSON.stringify({ row, entry })
  assert.throws(() => applyBulkDraft(entry, row, detail, () => { throw Error('tab storage full') }), /tab storage full/)
  assert.equal(JSON.stringify({ row, entry }), untouched)
  assert.equal(applyBulkDraft(entry, row, detail, () => { retained++ }), true)
  assert.equal(row.values.amount, '5'); assert.equal(entry.command.kind === 'receipt' && entry.command.payload.data.amount, '5'); assert.equal(entry.draft!.head_id, epoch); assert.equal(retained, 1)
})
