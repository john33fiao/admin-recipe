import { currentAppState } from '../app/utils/current-state.ts'
import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createContext, runInContext } from 'node:vm'
import ts from 'typescript'
import * as vue from 'vue'
import * as business from '../app/utils/business.ts'
import { ApiError } from '../app/utils/api.ts'
import { failureAction } from '../app/utils/catalog.ts'
import type { BusinessPayload } from '../app/types/business.ts'

const id = '00000000-0000-4000-8000-000000000001'
const version = '00000000-0000-4000-8000-000000000002'
const epoch = '00000000-0000-4000-8000-000000000003'
const requestId = '00000000-0000-4000-8000-000000000004'
const draft = () => ({ kind: 'purchase_return', version: 1, data: { receipt_id: id, expected_receipt_version_id: version, quantity: '1.', unit: 'g', note: '가상 반품\n확인 중' } } as unknown as BusinessPayload)
type Request = { path: string; body: any; resolve: (value: unknown) => void; reject: (reason: unknown) => void }
function harness() {
  const storage = new Map<string, string>(), states = new Map<string, vue.Ref>(), requests: Request[] = []
  let serial = 10
  const context = createContext({ currentAppState, ...vue, ...business, ApiError, failureAction,
    useConnection: () => vue.ref('connected'),
    useState: (key: string, init: () => unknown) => { if (!states.has(key)) states.set(key, vue.ref(init())); return states.get(key) },
    sessionStorage: { getItem: (key: string) => storage.get(key) ?? null, setItem: (key: string, value: string) => storage.set(key, value), removeItem: (key: string) => storage.delete(key) },
    crypto: { randomUUID: () => `00000000-0000-4000-8000-${String(serial++).padStart(12, '0')}` },
    apiRequest: (path: string, body: unknown) => new Promise((resolve, reject) => requests.push({ path, body, resolve, reject })),
  })
  const source = readFileSync(new URL('../app/composables/useBusinessDraft.ts', import.meta.url), 'utf8').replace(/^import .*$/gm, '').replace('export function useBusinessDraft', 'function useBusinessDraft').replace('export interface BusinessEditorInitial', 'interface BusinessEditorInitial')
  runInContext(ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText, context)
  return { create: context.useBusinessDraft, storage, requests }
}
const initial = () => ({ payload: draft(), state: { epoch, revision: '1' }, recordId: null, headId: null, status: 'draft', corrects: null, issues: [] })

test('return incomplete strings and null source restore while unknown fields remain rejected', () => {
  const payload = draft()
  assert.equal(business.validBusinessPayload(payload), true)
  assert.equal(business.validBusinessPayload({ ...payload, data: { receipt_id: null, expected_receipt_version_id: null, quantity: '', unit: '', note: '' } }), true)
  assert.equal(business.validBusinessPayload({ ...payload, data: { ...payload.data, refund_amount: '0' } }), false)
  const pending = business.businessPending('create', null, null, { epoch, revision: '1' }, payload, null, () => requestId)
  assert.deepEqual(business.restoreBusinessPending(JSON.stringify(pending)), pending)
})

test('return replacement and pending preserve source version separately from correction target', () => {
  const payload = { kind: 'correction', version: 1, data: { expected_target_version_id: requestId, reason: '가상 수량 정정', action: { kind: 'replace', replacement: { kind: 'purchase_return', data: draft().data } }, observation_reviews: [], linked_receipt_changes: [] } } as unknown as BusinessPayload
  const pending = business.businessPending('create', null, null, { epoch, revision: '9' }, payload, id, () => requestId)
  assert.deepEqual(business.restoreBusinessPending(JSON.stringify(pending)), pending)
  const copied = business.copyPayload(payload) as any
  copied.data.action.replacement.data.quantity = '2'
  assert.equal((payload as any).data.action.replacement.data.quantity, '1.')
})

test('uncertain return create restores original body even when the newly loaded source is void', async () => {
  const h = harness(), scope = vue.effectScope()
  try {
    const editor = scope.run(() => h.create('new.purchase_return.' + id, initial(), async () => null))!
    const sending = editor.send('create'), original = JSON.stringify(h.requests[0]!.body)
    assert.equal(h.requests[0]!.path, '/api/business/drafts')
    h.requests[0]!.reject(new ApiError(503, 'uncertain', 'unavailable')); await sending
    const fresh = harness(); for (const [key, value] of h.storage) fresh.storage.set(key, value)
    const seed = initial(); (seed.payload.data as any).expected_receipt_version_id = null
    const recovered = scope.run(() => fresh.create('new.purchase_return.' + id, seed, async () => null))!
    assert.equal(recovered.pending.value?.body.meta.request_id, JSON.parse(original).meta.request_id)
    assert.equal(recovered.payload.value.data.expected_receipt_version_id, version)
    assert.equal(recovered.locked.value, true)
    const retry = recovered.send('create'); assert.equal(JSON.stringify(fresh.requests[0]!.body), original)
    fresh.requests[0]!.reject(new ApiError(503, 'uncertain', 'unavailable')); await retry
  } finally { scope.stop() }
})

test('return input edits invalidate preview and immutable pending keeps the sent flat fields', async () => {
  const h = harness(), scope = vue.effectScope()
  try {
    const seed = { ...initial(), recordId: id, headId: version }
    const editor = scope.run(() => h.create(id, seed, async () => null))!
    const showing = editor.showPreview(); h.requests[0]!.resolve({ record_id: id, head_id: version, state: { epoch, revision: '2' }, can_confirm: true, issues: [], effect: { kind: 'purchase_return' } }); await showing
    editor.payload.value.data.quantity = '2'
    assert.equal(editor.dirty.value, true); assert.equal(editor.preview.value, null)
    assert.equal(await editor.send('confirm'), null)
    const sending = editor.send('revise'), original = JSON.stringify(h.requests[1]!.body)
    editor.payload.value.data.quantity = '3'
    assert.equal(JSON.stringify(h.requests[1]!.body), original)
    h.requests[1]!.reject(new ApiError(503, 'uncertain', 'unavailable')); await sending
  } finally { scope.stop() }
})

test('explicit new return after a completed create has a fresh request and keeps the same source', async () => {
  const h = harness(), scope = vue.effectScope()
  try {
    const seed = initial(), editor = scope.run(() => h.create('new.purchase_return.' + id, seed, async () => null))!
    const first = editor.send('create'), originalId = h.requests[0]!.body.meta.request_id
    h.requests[0]!.resolve({ result: { item: { id, kind: 'purchase_return', status: 'draft' }, head: { id: version, payload: seed.payload, status: 'draft' }, issues: [] }, state: { epoch, revision: '2' } }); await first
    assert.ok(editor.created.value)
    editor.resetNew(); assert.equal(editor.created.value, null)
    const second = editor.send('create')
    assert.notEqual(h.requests[1]!.body.meta.request_id, originalId)
    assert.equal(h.requests[1]!.body.payload.data.receipt_id, id)
    assert.equal(h.requests[1]!.body.meta.expected_revision, '2')
    h.requests[1]!.reject(new ApiError(503, 'uncertain', 'unavailable')); await second
  } finally { scope.stop() }
})

function sourceHarness() {
  const requests: Request[] = [], data = vue.ref<any>((draft() as any).data), disabled = vue.ref(false)
  const context = createContext({ currentAppState, ...vue, useConnection: () => vue.ref('connected'), apiRequest: (path: string, body: unknown) => new Promise((resolve, reject) => requests.push({ path, body, resolve, reject })) })
  const source = readFileSync(new URL('../app/composables/usePurchaseReturnSource.ts', import.meta.url), 'utf8').replace(/^import .*$/gm, '').replace('export function usePurchaseReturnSource', 'function usePurchaseReturnSource')
  runInContext(ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText, context)
  const scope = vue.effectScope(), editor = scope.run(() => context.usePurchaseReturnSource(data, disabled))!
  return { data, disabled, requests, scope, editor }
}
function sourceResponse(receiptId: string, versionId: string, unit = 'g') {
  return { original: { id: receiptId }, current: { reference: { kind: 'existing', entity: 'receipt', id: receiptId, version_id: versionId }, material_id: epoch, unit, quantity: '3', amount: '30', returned_quantity: '0', remaining_quantity: '3' }, state: { epoch, revision: '90' } }
}

test('source reads never change editing CAS and late explicit selection cannot overwrite the next source', async () => {
  const h = sourceHarness()
  try {
    h.requests[0]!.resolve(sourceResponse(id, requestId)); h.requests[1]!.resolve({ items: [] }); await new Promise(resolve => setImmediate(resolve))
    assert.equal(h.data.value.expected_receipt_version_id, version)
    h.requests.find(item => item.path.startsWith('/api/holdings/'))!.resolve({ material_id: epoch })
    const selecting = h.editor.select(requestId); await new Promise(resolve => setImmediate(resolve))
    const old = h.requests.filter(item => item.path.includes(requestId + '/return-source'))
    assert.equal(old.length, 2)
    h.data.value.receipt_id = epoch; h.data.value.quantity = '2.'; await vue.nextTick()
    old.forEach(request => request.resolve(sourceResponse(requestId, id, 'kg')))
    await selecting
    assert.equal(h.data.value.receipt_id, epoch); assert.equal(h.data.value.expected_receipt_version_id, null)
    assert.equal(h.data.value.unit, 'g'); assert.equal(h.data.value.quantity, '2.')
    assert.equal(h.editor.source.value, null)
  } finally { h.scope.stop() }
})

test('explicit source response does not modify a submitted or unmounted form', async () => {
  for (const unmount of [false, true]) {
    const h = sourceHarness()
    try {
      const applying = h.editor.load(true), request = h.requests.at(-1)!
      const before = JSON.stringify(h.data.value)
      if (unmount) h.scope.stop(); else h.disabled.value = true
      request.resolve(sourceResponse(id, requestId, 'kg')); await new Promise(resolve => setImmediate(resolve))
      const holding = h.requests.find(item => item.path.startsWith('/api/holdings/')); holding?.resolve({ material_id: epoch })
      await applying
      assert.equal(JSON.stringify(h.data.value), before)
    } finally { h.scope.stop() }
  }
})
