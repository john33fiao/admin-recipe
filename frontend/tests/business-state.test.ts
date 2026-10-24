import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createContext, runInContext } from 'node:vm'
import ts from 'typescript'
import * as vue from 'vue'
import * as business from '../app/utils/business.ts'
import { failureAction } from '../app/utils/catalog.ts'
import { ApiError } from '../app/utils/api.ts'
import { blankSaleDraft } from '../app/utils/sale.ts'
import { blankProductionDraft, blankProductionOutput } from '../app/utils/production.ts'
import { blankStocktakeDraft, blankWasteDraft } from '../app/utils/stocktake.ts'

const recordId = '00000000-0000-4000-8000-000000000001'
const headId = '00000000-0000-4000-8000-000000000002'
const epoch = '00000000-0000-4000-8000-000000000003'
const requestId = '00000000-0000-4000-8000-000000000004'
const payload = business.blankPriceDraft()
payload.data = { material_id: recordId, material_version_id: headId, quantity: '1', unit: 'g', amount: '10', note: '' }
const item = { id: recordId, head_id: headId, status: 'draft', kind: 'catalog_price', issue_count: 0 }
const head = { id: headId, record_id: recordId, status: 'draft', payload }
const initial = { payload, state: { epoch, revision: '1' }, recordId, headId, status: 'draft', corrects: null, issues: [] }
type Request = { path: string; body: unknown; resolve: (value: unknown) => void; reject: (reason: unknown) => void }
function harness() {
  const states = new Map<string, vue.Ref>()
  const storage = new Map<string, string>()
  const requests: Request[] = []
  const context = createContext({
    ...vue, ...business, ApiError, failureAction,
    useConnection: () => vue.ref('connected'),
    useState: (key: string, init: () => unknown) => { if (!states.has(key)) states.set(key, vue.ref(init())); return states.get(key) },
    sessionStorage: { getItem: (key: string) => storage.get(key) ?? null, setItem: (key: string, value: string) => storage.set(key, value), removeItem: (key: string) => storage.delete(key) },
    crypto: { randomUUID: () => requestId },
    apiRequest: (path: string, body: unknown) => new Promise((resolve, reject) => requests.push({ path, body, resolve, reject })),
  })
  const original = readFileSync(new URL('../app/composables/useBusinessDraft.ts', import.meta.url), 'utf8')
  const source = original.replace(/^import .*$/gm, '').replace('export function useBusinessDraft', 'function useBusinessDraft').replace('export interface BusinessEditorInitial', 'interface BusinessEditorInitial')
  runInContext(ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText, context)
  return { create: context.useBusinessDraft, storage, requests }
}

test('production nested edits invalidate preview and preserve original pending through navigation and retry', async () => {
  const h = harness()
  const scope = vue.effectScope()
  const seed = { ...initial, payload: blankProductionDraft() }
  const editor = scope.run(() => h.create(recordId, seed, async () => null))!
  const showing = editor.showPreview()
  h.requests[0]!.resolve({ record_id: recordId, head_id: headId, state: { epoch, revision: '5' }, can_confirm: true, issues: [], effect: { kind: 'production' } })
  await showing
  editor.payload.value.data.outputs[0].allocation_percent = '70'
  assert.equal(editor.dirty.value, true)
  assert.equal(editor.preview.value, null)
  editor.payload.value.data.outputs.push(blankProductionOutput(false))
  editor.payload.value.data.outputs[1].allocation_percent = '30'
  const saving = editor.send('revise')
  const original = JSON.stringify(h.requests[1]!.body)
  assert.equal(h.requests[1]!.path, `/api/production-drafts/${recordId}/revisions`)
  // Even a late local mutation cannot rewrite a request already sent.
  editor.payload.value.data.outputs[0].allocation_percent = '40'
  assert.equal(JSON.stringify(h.requests[1]!.body), original)
  scope.stop()
  const nextScope = vue.effectScope()
  try {
    const next = nextScope.run(() => h.create(recordId, seed, async () => null))!
    assert.equal(next.locked.value, true)
    h.requests[1]!.reject(new ApiError(503, 'uncertain', 'unavailable'))
    await saving
    const retry = next.send('revise')
    assert.equal(h.requests[2]!.path, h.requests[1]!.path)
    assert.equal(JSON.stringify(h.requests[2]!.body), original)
    const savedPayload = JSON.parse(original).payload
    h.requests[2]!.resolve({ result: { item, head: { ...head, payload: savedPayload }, issues: [] }, state: { epoch, revision: '6' } })
    await retry
    assert.equal(next.payload.value.data.outputs[0].allocation_percent, '70')
    assert.equal(next.dirty.value, false)
    assert.equal(next.pending.value, null)
    assert.equal(h.storage.size, 0)
  } finally { nextScope.stop() }
})

test('only production size rejection enables explicit review while immutable create is retained', async () => {
  for (const value of [blankProductionDraft(), business.blankPriceDraft()]) {
    const h = harness()
    const scope = vue.effectScope()
    try {
      const editor = scope.run(() => h.create('new', { ...initial, payload: value, recordId: null, headId: null }, async () => null))!
      const sending = editor.send('create')
      assert.equal(h.requests[0]!.path, value.kind === 'production' ? '/api/production-drafts' : '/api/business/drafts')
      const original = JSON.stringify(editor.pending.value)
      h.requests[0]!.reject(new ApiError(413, '입력이 너무 큽니다', 'validation'))
      await sending
      assert.equal(JSON.stringify(editor.pending.value), original)
      assert.equal(editor.canReview.value, value.kind === 'production')
      if (value.kind === 'production') {
        assert.equal(await editor.send('create'), null)
        const reviewing = editor.review()
        h.requests[1]!.resolve({ epoch, revision: '8' })
        await reviewing
        assert.equal(editor.pending.value, null)
        assert.equal(editor.locked.value, false)
        assert.equal(editor.payload.value.data.outputs[0].allocation_percent, '100')
      }
    } finally { scope.stop() }
  }
})

test('creating a database draft after navigation clears the old local input and shares its result', async () => {
  const h = harness()
  const seed = { ...initial, payload: business.blankPriceDraft(), recordId: null, headId: null }
  const firstScope = vue.effectScope()
  const first = firstScope.run(() => h.create('new.price', seed, async () => null))!
  first.payload.value.data.amount = '0'
  const saving = first.send('create')
  firstScope.stop()
  const nextScope = vue.effectScope()
  try {
    const next = nextScope.run(() => h.create('new.price', seed, async () => null))!
    assert.equal(next.payload.value.data.amount, '0')
    h.requests[0]!.resolve({ result: { item, head: { ...head, payload: business.blankPriceDraft() }, issues: [] }, state: { epoch, revision: '2' } })
    await saving
    assert.equal(next.created.value.id, recordId)
    assert.equal(next.payload.value.data.amount, '')
    assert.equal(next.pending.value, null)
    assert.equal(h.storage.has('sunchang.business.new.price.draft.v1'), false)
    assert.equal(h.storage.has('sunchang.business.new.price.pending.v1'), false)
    assert.equal(await next.send('create'), null)
  } finally { nextScope.stop() }
})

test('confirmation uses the shown preview tuple and uncertain retries preserve the whole request', async () => {
  const h = harness()
  const scope = vue.effectScope()
  try {
    const editor = scope.run(() => h.create(recordId, initial, async () => null))!
    const showing = editor.showPreview()
    h.requests[0]!.resolve({ record_id: recordId, head_id: headId, state: { epoch, revision: '9' }, can_confirm: true, issues: [], effect: { kind: 'catalog_price', amount: '10', quantity: '1', unit: 'g' } })
    assert.equal(await showing, true)
    const sending = editor.send('confirm')
    const original = JSON.stringify(h.requests[1]!.body)
    assert.equal(JSON.parse(original).meta.expected_revision, '9')
    assert.equal(Object.hasOwn(JSON.parse(original), 'payload'), false)
    h.requests[1]!.reject(new ApiError(503, 'uncertain', 'unavailable'))
    assert.equal(await sending, null)
    const retry = editor.send('confirm')
    assert.equal(JSON.stringify(h.requests[2]!.body), original)
    h.requests[2]!.resolve({ result: { item: { ...item, status: 'confirmed' }, head: { ...head, status: 'confirmed' }, issues: [] }, state: { epoch, revision: '10' } })
    await retry
    assert.equal(editor.status.value, 'confirmed')
  } finally { scope.stop() }
})

test('stale review is exclusive and terminal server input remains distinct from unsaved input', async () => {
  const h = harness()
  const refreshes: Array<(value: unknown) => void> = []
  const refresh = () => new Promise(resolve => refreshes.push(resolve))
  const firstScope = vue.effectScope()
  const editor = firstScope.run(() => h.create(recordId, initial, refresh))!
  editor.payload.value.data.amount = '100'
  const saving = editor.send('revise')
  h.requests[0]!.reject(new ApiError(409, 'stale', 'stale_state'))
  await saving
  const reviewing = editor.review()
  firstScope.stop()
  const nextScope = vue.effectScope()
  try {
    const next = nextScope.run(() => h.create(recordId, initial, refresh))!
    assert.equal(await next.review(), false)
    assert.equal(await next.send('revise'), null)
    assert.equal(refreshes.length, 1)
    refreshes[0]!({ item: { ...item, status: 'confirmed' }, head: { ...head, status: 'confirmed' }, issues: [], state: { epoch, revision: '2' } })
    assert.equal(await reviewing, true)
    assert.equal(next.status.value, 'confirmed')
    assert.equal(next.payload.value.data.amount, '100')
    assert.equal(next.dirty.value, true)
    assert.equal(next.locked.value, true)
    assert.equal(await next.send('revise'), null)
    assert.equal(next.pending.value, null)
  } finally { nextScope.stop() }
})

test('order actions reuse immutable preview retry and share completion across navigation', async () => {
  for (const action of ['place', 'revise', 'cancel']) {
    const h = harness()
    const fields = { material_version_id: headId, quantity: '12', unit: 'kg', supplier: '', expected_date: '', estimated_amount: '0', note: '' }
    const orderPayload = { kind: 'purchase_order', version: 1, data: action === 'place' ? { action, material_id: recordId, ...fields } : action === 'revise' ? { action, order_id: recordId, expected_order_version_id: headId, ...fields } : { action, order_id: recordId, expected_order_version_id: headId, note: '' } }
    const seed = { ...initial, payload: orderPayload }
    const firstScope = vue.effectScope()
    const first = firstScope.run(() => h.create(recordId, seed, async () => null))!
    const showing = first.showPreview()
    h.requests[0]!.resolve({ record_id: recordId, head_id: headId, state: { epoch, revision: '9' }, can_confirm: true, issues: [], effect: { kind: 'purchase_order', action, before: null, after: {} } })
    await showing
    const sending = first.send('confirm')
    const original = JSON.stringify(h.requests[1]!.body)
    h.requests[1]!.reject(new ApiError(503, 'uncertain', 'unavailable'))
    await sending
    firstScope.stop()
    const nextScope = vue.effectScope()
    try {
      const next = nextScope.run(() => h.create(recordId, seed, async () => null))!
      const retry = next.send('confirm')
      assert.equal(JSON.stringify(h.requests[2]!.body), original)
      h.requests[2]!.resolve({ result: { item: { ...item, kind: 'purchase_order', status: 'confirmed' }, head: { ...head, payload: orderPayload, status: 'confirmed' }, issues: [] }, state: { epoch, revision: '10' } })
      await retry
      assert.equal(next.status.value, 'confirmed')
      assert.equal(next.pending.value, null)
      assert.equal(next.dirty.value, false)
      assert.equal(h.storage.has(`sunchang.business.${recordId}.pending.v1`), false)
    } finally { nextScope.stop() }
  }
})

test('nested receipt edits invalidate preview while baseline and pending survive navigation unchanged', async () => {
  const h = harness()
  const receiptPayload = { kind: 'receipt', version: 1, data: { source: { kind: 'direct', material_id: recordId, material_version_id: headId }, quantity: '1', unit: 'kg', amount: '0', receipt_date: '2024-02-29', note: '', stock_effect: { kind: 'unreflected' } } }
  const seed = { ...initial, payload: receiptPayload }
  const firstScope = vue.effectScope()
  const first = firstScope.run(() => h.create(recordId, seed, async () => null))!
  const showing = first.showPreview()
  h.requests[0]!.resolve({ record_id: recordId, head_id: headId, state: { epoch, revision: '9' }, can_confirm: true, issues: [], effect: { kind: 'receipt', after: { quantity: '1000', value: '0', source_id: null } } })
  await showing
  first.payload.value.data.source.material_version_id = requestId
  assert.equal(first.dirty.value, true)
  assert.equal(first.preview.value, null)
  assert.equal(seed.payload.data.source.material_version_id, headId)
  const saving = first.send('revise')
  const original = JSON.stringify(h.requests[1]!.body)
  first.payload.value.data.source.material_version_id = epoch
  assert.equal(JSON.stringify(h.requests[1]!.body), original)
  h.requests[1]!.reject(new ApiError(503, 'uncertain', 'unavailable'))
  await saving
  firstScope.stop()
  const nextScope = vue.effectScope()
  try {
    const next = nextScope.run(() => h.create(recordId, seed, async () => null))!
    const retry = next.send('revise')
    assert.equal(JSON.stringify(h.requests[2]!.body), original)
    const saved = JSON.parse(original).payload
    h.requests[2]!.resolve({ result: { item: { ...item, kind: 'receipt' }, head: { ...head, payload: saved }, issues: [] }, state: { epoch, revision: '10' } })
    await retry
    assert.equal(next.dirty.value, false)
    assert.equal(next.payload.value.data.source.material_version_id, requestId)
    assert.equal(next.pending.value, null)
  } finally { nextScope.stop() }
})

test('reconciliation-only edits are dirty and a recovered receipt request restores exact nested input', async () => {
  const h = harness()
  const payload = { kind: 'receipt', version: 1, data: { source: { kind: 'direct', material_id: recordId, material_version_id: headId }, quantity: '1.', unit: 'kg', amount: '', receipt_date: '2024-0', note: '', stock_effect: { kind: 'needs_reconciliation', basis_id: null, included_quantity: '' } } }
  const seed = { ...initial, payload }
  const scope = vue.effectScope()
  try {
    const first = scope.run(() => h.create(recordId, seed, async () => null))!
    first.payload.value.data.stock_effect.included_quantity = '0.'
    assert.equal(first.dirty.value, true)
    assert.equal(payload.data.stock_effect.included_quantity, '')
    const sending = first.send('revise')
    const original = JSON.stringify(h.requests[0]!.body)
    h.requests[0]!.reject(new ApiError(503, 'uncertain', 'unavailable'))
    await sending
    const fresh = harness()
    fresh.storage.set(`sunchang.business.${recordId}.pending.v1`, h.storage.get(`sunchang.business.${recordId}.pending.v1`)!)
    const recovered = scope.run(() => fresh.create(recordId, seed, async () => null))!
    assert.equal(recovered.payload.value.data.stock_effect.included_quantity, '0.')
    assert.equal(recovered.locked.value, true)
    const retry = recovered.send('revise')
    assert.equal(JSON.stringify(fresh.requests[0]!.body), original)
    fresh.requests[0]!.reject(new ApiError(503, 'uncertain', 'unavailable'))
    await retry
  } finally { scope.stop() }
})

test('explicit new input uses the freshly loaded state after returning from a previous receipt', async () => {
  const h = harness()
  const seed = { ...initial, recordId: null, headId: null }
  const firstScope = vue.effectScope()
  const first = firstScope.run(() => h.create('new.direct', seed, async () => null))!
  const saving = first.send('create')
  h.requests[0]!.resolve({ result: { item, head, issues: [] }, state: { epoch, revision: '2' } })
  await saving
  firstScope.stop()
  const nextScope = vue.effectScope()
  try {
    const next = nextScope.run(() => h.create('new.direct', { ...seed, state: { epoch, revision: '9' } }, async () => null))!
    next.resetNew()
    const sending = next.send('create')
    assert.equal((h.requests[1]!.body as { meta: { expected_revision: string } }).meta.expected_revision, '9')
    h.requests[1]!.reject(new ApiError(503, 'uncertain', 'unavailable'))
    await sending
  } finally { nextScope.stop() }
})


test('sale correction lost create survives a fresh instance with the exact original target and body', async () => {
  const h = harness(), scope = vue.effectScope()
  const sale = blankSaleDraft(); Object.assign(sale.data, { material_id: recordId, material_version_id: headId, recipe_version_id: requestId, count: '0', note: '가상 정정' })
  const seed = { ...initial, payload: sale, recordId: null, headId: null, corrects: headId }
  try {
    const editor = scope.run(() => h.create('new.sale.correction', seed, async () => null))!
    const saving = editor.send('create'), original = JSON.stringify(h.requests[0]!.body)
    h.requests[0]!.reject(new ApiError(503, 'uncertain', 'unavailable')); await saving
    const fresh = harness(); for (const [key, value] of h.storage) fresh.storage.set(key, value)
    const recovered = scope.run(() => fresh.create('new.sale.correction', seed, async () => null))!
    assert.equal(recovered.payload.value.data.count, '0'); assert.equal(recovered.locked.value, true)
    const retry = recovered.send('create'); assert.equal(JSON.stringify(fresh.requests[0]!.body), original)
    assert.equal(JSON.parse(original).corrects_confirmation_id, headId)
    fresh.requests[0]!.reject(new ApiError(503, 'uncertain', 'unavailable')); await retry
    const different = harness(); for (const [key, value] of h.storage) different.storage.set(key, value)
    const rejected = scope.run(() => different.create('new.sale.correction', { ...seed, corrects: epoch }, async () => null))!
    assert.equal(rejected.pending.value, null)
  } finally { scope.stop() }
})

test('sale count edit invalidates preview and confirmation retry preserves frozen preview state', async () => {
  const h = harness(), scope = vue.effectScope(), sale = blankSaleDraft()
  Object.assign(sale.data, { material_id: recordId, material_version_id: headId, recipe_version_id: requestId, count: '3' })
  try {
    const editor = scope.run(() => h.create(recordId, { ...initial, payload: sale }, async () => null))!
    const showing = editor.showPreview(); h.requests[0]!.resolve({ record_id: recordId, head_id: headId, state: { epoch, revision: '9' }, can_confirm: true, issues: [], effect: { kind: 'sale' } }); await showing
    editor.payload.value.data.count = '1'; assert.equal(editor.preview.value, null); assert.equal(editor.dirty.value, true)
    assert.equal(await editor.send('confirm'), null)
    editor.payload.value.data.count = '3'
    const again = editor.showPreview(); h.requests[1]!.resolve({ record_id: recordId, head_id: headId, state: { epoch, revision: '10' }, can_confirm: true, issues: [], effect: { kind: 'sale' } }); await again
    const confirming = editor.send('confirm'), original = JSON.stringify(h.requests[2]!.body)
    h.requests[2]!.reject(new ApiError(503, 'uncertain', 'unavailable')); await confirming
    const retry = editor.send('confirm'); assert.equal(JSON.stringify(h.requests[3]!.body), original); assert.equal(JSON.parse(original).meta.expected_revision, '10')
    h.requests[3]!.resolve({ result: { item: { ...item, kind: 'sale', status: 'confirmed' }, head: { ...head, payload: sale, status: 'confirmed' }, issues: [] }, state: { epoch, revision: '11' } }); await retry
    assert.equal(editor.status.value, 'confirmed'); assert.equal(editor.payload.value.data.recipe_version_id, requestId); assert.equal(editor.pending.value, null)
  } finally { scope.stop() }
})


test('stocktake nested valuation edits invalidate preview while retry restores exact rows and alias', async () => {
  const h = harness(), scope = vue.effectScope(), stock = blankStocktakeDraft()
  Object.assign(stock.data.rows[0]!, { material_id: recordId, material_version_id: headId, quantity: '0', initial_value: { kind: 'manual_amount', amount: '0' }, excess: { reviewed: false, reason: '', value: { kind: 'manual_amount', amount: '' } } })
  const seed = { ...initial, payload: stock }
  try {
    const editor = scope.run(() => h.create(recordId, seed, async () => null))!
    const showing = editor.showPreview(); h.requests[0]!.resolve({ record_id: recordId, head_id: headId, state: { epoch, revision: '9' }, can_confirm: true, issues: [], effect: { kind: 'stocktake', rows: [], unchecked_positions: [] } }); await showing
    editor.payload.value.data.rows[0].excess.value.amount = '1.'
    assert.equal(editor.dirty.value, true); assert.equal(editor.preview.value, null)
    assert.equal(stock.data.rows[0]!.excess!.value!.kind === 'manual_amount' && stock.data.rows[0]!.excess!.value!.amount, '')
    const sending = editor.send('revise'), original = JSON.stringify(h.requests[1]!.body)
    assert.equal(h.requests[1]!.path, '/api/stocktake-drafts/' + recordId + '/revisions')
    editor.payload.value.data.rows[0].excess.value.amount = '5'
    assert.equal(JSON.stringify(h.requests[1]!.body), original)
    h.requests[1]!.reject(new ApiError(503, 'uncertain', 'unavailable')); await sending
    const fresh = harness(); for (const [key, value] of h.storage) fresh.storage.set(key, value)
    const recovered = scope.run(() => fresh.create(recordId, seed, async () => null))!
    assert.equal(recovered.payload.value.data.rows[0].excess.value.amount, '1.')
    assert.equal(recovered.locked.value, true)
    const retry = recovered.send('revise'); assert.equal(JSON.stringify(fresh.requests[0]!.body), original)
    fresh.requests[0]!.reject(new ApiError(503, 'uncertain', 'unavailable')); await retry
  } finally { scope.stop() }
})

test('stocktake 413 preserves original create until explicit review and waste uses common endpoint', async () => {
  for (const value of [blankStocktakeDraft(), blankWasteDraft()]) {
    const h = harness(), scope = vue.effectScope()
    try {
      const editor = scope.run(() => h.create('new.stock', { ...initial, payload: value, recordId: null, headId: null }, async () => null))!
      const sending = editor.send('create'), raw = JSON.stringify(editor.pending.value)
      assert.equal(h.requests[0]!.path, value.kind === 'stocktake' ? '/api/stocktake-drafts' : '/api/business/drafts')
      h.requests[0]!.reject(new ApiError(413, '입력이 너무 큽니다', 'validation')); await sending
      assert.equal(JSON.stringify(editor.pending.value), raw)
      assert.equal(editor.canReview.value, value.kind === 'stocktake')
      if (value.kind === 'stocktake') {
        const reviewing = editor.review(); h.requests[1]!.resolve({ epoch, revision: '12' }); await reviewing
        assert.equal(editor.pending.value, null); assert.equal(editor.payload.value.data.rows[0].quantity, '')
        const again = editor.send('create'); assert.equal((h.requests[2]!.body as any).meta.expected_revision, '12')
        h.requests[2]!.reject(new ApiError(503, 'uncertain', 'unavailable')); await again
      }
    } finally { scope.stop() }
  }
})

test('stocktake stale branch review keeps the entered initial and excess confirmations', async () => {
  const h = harness(), scope = vue.effectScope(), stock = blankStocktakeDraft()
  Object.assign(stock.data.rows[0]!, { quantity: '0', initial_value: { kind: 'manual_amount', amount: '0' }, excess: { reviewed: true, reason: '입고 누락 확인', value: null } })
  const latest = { item, head: { ...head, payload: blankStocktakeDraft() }, issues: [], state: { epoch, revision: '11' } }
  try {
    const editor = scope.run(() => h.create(recordId, { ...initial, payload: stock }, async () => latest))!
    const sending = editor.send('revise'); h.requests[0]!.reject(new ApiError(409, 'stale', 'stale_state')); await sending
    await editor.review()
    assert.equal(editor.payload.value.data.rows[0].initial_value.amount, '0')
    assert.equal(editor.payload.value.data.rows[0].excess.reason, '입고 누락 확인')
    assert.equal(editor.dirty.value, true); assert.equal(editor.pending.value, null); assert.equal(editor.preview.value, null)
  } finally { scope.stop() }
})
