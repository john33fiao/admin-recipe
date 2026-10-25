import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createContext, runInContext } from 'node:vm'
import ts from 'typescript'
import * as vue from 'vue'
import * as business from '../app/utils/business.ts'
import { failureAction } from '../app/utils/catalog.ts'
import { ApiError, apiRequest } from '../app/utils/api.ts'
import { currentAppState } from '../app/utils/current-state.ts'
import * as catalog from '../app/utils/catalog.ts'
import { restoreCatalogDraft } from '../app/utils/draft.ts'

const oldEpoch = '10000000-0000-4000-8000-000000000001', newEpoch = '10000000-0000-4000-8000-000000000002'
const record = '20000000-0000-4000-8000-000000000001', head = '20000000-0000-4000-8000-000000000002', uuid = '30000000-0000-4000-8000-000000000001'
const state = (epoch: string, revision = '1') => ({ epoch, revision })
const json = (data: unknown, status = 200) => new Response(JSON.stringify(data), { status, headers: { 'Content-Type': 'application/json' } })

test('learning the initial state does not invalidate an ordinary startup list', async () => {
  const originalFetch = globalThis.fetch, requests: Array<(response: Response) => void> = []
  globalThis.fetch = (() => new Promise(resolve => requests.push(resolve))) as typeof fetch
  try {
    const list = apiRequest('/api/materials'), current = apiRequest('/api/state')
    requests[1]!(json(state(oldEpoch))); await current
    requests[0]!(json([{ id: record }])); assert.deepEqual(await list, [{ id: record }])
  } finally { globalThis.fetch = originalFetch }
})

test('a successful pre-restore response arriving after the new state preserves the original business request and inputs', async () => {
  const originalFetch = globalThis.fetch, requests: Array<{ path: string; body: string; resolve: (response: Response) => void }> = []
  globalThis.fetch = ((path: string, options: RequestInit) => new Promise(resolve => requests.push({ path, body: String(options?.body ?? ''), resolve }))) as typeof fetch
  const states = new Map<string, vue.Ref>(), storage = new Map<string, string>(), scope = vue.effectScope()
  const payload = business.blankPriceDraft()
  payload.data = { material_id: record, material_version_id: head, quantity: '1', unit: 'g', amount: '10', note: '보존할 입력' }
  const initial = { payload, state: state(oldEpoch), recordId: record, headId: head, status: 'draft', corrects: null, issues: [] }
  const context = createContext({ currentAppState, ...vue, ...business, ApiError, apiRequest, failureAction, useConnection: () => vue.ref('connected'), useState: (key: string, init: () => unknown) => { if (!states.has(key)) states.set(key, vue.ref(init())); return states.get(key) }, crypto: { randomUUID: () => uuid }, sessionStorage: { getItem: (key: string) => storage.get(key) ?? null, setItem: (key: string, value: string) => storage.set(key, value), removeItem: (key: string) => storage.delete(key) } })
  const source = readFileSync(new URL('../app/composables/useBusinessDraft.ts', import.meta.url), 'utf8').replace(/^import .*$/gm, '').replace(/export (function|interface) /g, '$1 ')
  scope.run(() => runInContext(ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText, context))
  try {
    const firstState = apiRequest('/api/state'); requests[0]!.resolve(json(state(oldEpoch))); await firstState
    const editor = scope.run(() => context.useBusinessDraft(record, initial, async () => null))!
    editor.payload.value.data.amount = '20'
    const saving = editor.send('revise'), pending = JSON.stringify(editor.pending.value)
    const current = apiRequest('/api/state'); requests[2]!.resolve(json(state(newEpoch, '8'))); await current
    requests[1]!.resolve(json({ state: state(oldEpoch, '2'), result: { item: { id: record, kind: 'catalog_price', status: 'draft' }, head: { id: head, record_id: record, status: 'draft', payload: JSON.parse(requests[1]!.body).payload }, issues: [] } }))
    await saving
    assert.equal(JSON.stringify(editor.pending.value), pending, 'a past success must not clear the immutable request')
    assert.equal(editor.payload.value.data.amount, '20')
    assert.equal(editor.notice.value, '')
    assert.equal(storage.has(`sunchang.business.${record}.pending.v1`), true)
  } finally { scope.stop(); globalThis.fetch = originalFetch }
})

test('a late GET cannot republish the pre-restore state or unversioned list', async () => {
  const originalFetch = globalThis.fetch
  const requests: Array<(response: Response) => void> = []
  globalThis.fetch = (() => new Promise(resolve => requests.push(resolve))) as typeof fetch
  try {
    const old = currentAppState.value!
    const read = apiRequest('/api/state'), list = apiRequest('/api/materials')
    const restored = apiRequest('/api/restores', { meta: {} })
    requests[2]!(json({ state: state('10000000-0000-4000-8000-000000000003', '12') })); await restored
    requests[0]!(json(old)); requests[1]!(json([{ id: record }]))
    await assert.rejects(read, (error: ApiError) => error.code === 'stale_read')
    await assert.rejects(list, (error: ApiError) => error.code === 'stale_read')
    assert.equal(currentAppState.value!.revision, '12')
  } finally { globalThis.fetch = originalFetch }
})

test('processed generation failures block new UUID review while unprocessed failures require explicit review', () => {
  assert.equal(failureAction(409, 'stale_generation', 'already_processed'), 'blocked')
  assert.equal(failureAction(409, 'stale_generation', 'not_processed'), 'refresh')
  assert.equal(failureAction(409, 'stale_generation'), 'blocked')
  assert.equal(failureAction(409, 'stale_state'), 'refresh')
  assert.equal(failureAction(409, 'request_conflict'), 'blocked')
})

test('a pre-registration stale read preserves editable input and retries the read without a mutation conflict gate', async () => {
  const states = new Map<string, vue.Ref>(), storage = new Map<string, string>(), calls: string[] = [], scope = vue.effectScope()
  let stale = true
  const context = createContext({ ...vue, ...catalog, restoreCatalogDraft, ApiError, crypto: { randomUUID: () => uuid }, useConnection: () => vue.ref('connected'), useState: (key: string, init: () => unknown) => { if (!states.has(key)) states.set(key, vue.ref(init())); return states.get(key) }, sessionStorage: { getItem: (key: string) => storage.get(key) ?? null, setItem: (key: string, value: string) => storage.set(key, value), removeItem: (key: string) => storage.delete(key) }, apiRequest: async (path: string, body?: unknown) => { calls.push(path); if (path === '/api/state') { if (stale) { stale = false; throw new ApiError(409, '현재 자료를 다시 조회해 주세요.', 'stale_read') } return state(newEpoch) } return { ...(body as object), id: record, ...state(newEpoch, '2') } } })
  const source = readFileSync(new URL('../app/composables/useMaterialRegistration.ts', import.meta.url), 'utf8').replace(/^import .*$/gm, '').replace('export function useMaterialRegistration', 'function useMaterialRegistration')
  scope.run(() => runInContext(ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText, context))
  try {
    const editor = scope.run(() => context.useMaterialRegistration())!; editor.name.value = '가상 조회 재시도'
    await editor.save(); assert.deepEqual(calls, ['/api/state']); assert.equal(editor.pending.value, null); assert.equal(editor.rejected.value, false); assert.equal(editor.name.value, '가상 조회 재시도'); assert.equal(editor.failure.value, '현재 자료를 다시 조회해 주세요.')
    await editor.save(); assert.deepEqual(calls, ['/api/state', '/api/state', '/api/materials']); assert.equal(editor.pending.value, null); assert.equal(editor.name.value, '')
  } finally { scope.stop() }
})

test('explicit new material input retains a processed request, sends nothing and starts blank with current metadata', async () => {
  const states = new Map<string, vue.Ref>(), storage = new Map<string, string>(), calls: Array<{ path: string; body: any }> = [], scope = vue.effectScope()
  let processed = false, storageFails = false, serial = 1
  const context = createContext({ ...vue, ...catalog, restoreCatalogDraft, ApiError, crypto: { randomUUID: () => `30000000-0000-4000-8000-${String(serial++).padStart(12, '0')}` }, useConnection: () => vue.ref('connected'), useState: (key: string, init: () => unknown) => { if (!states.has(key)) states.set(key, vue.ref(init())); return states.get(key) }, sessionStorage: { getItem: (key: string) => storage.get(key) ?? null, setItem: (key: string, value: string) => { if (storageFails) throw Error('quota'); storage.set(key, value) }, removeItem: (key: string) => storage.delete(key) }, apiRequest: async (path: string, body?: unknown) => { calls.push({ path, body }); if (path === '/api/state') return state(processed ? newEpoch : oldEpoch, processed ? '9' : '1'); if (!processed) { processed = true; throw new ApiError(409, '', 'stale_generation', { request_status: 'already_processed', state: state(newEpoch, '9') }) } return { ...(body as object), id: record, ...state(newEpoch, '10') } } })
  const source = readFileSync(new URL('../app/composables/useMaterialRegistration.ts', import.meta.url), 'utf8').replace(/^import .*$/gm, '').replace('export function useMaterialRegistration', 'function useMaterialRegistration')
  scope.run(() => runInContext(ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText, context))
  try {
    const editor = scope.run(() => context.useMaterialRegistration())!; editor.name.value = '가상 처리된 원입력'; await editor.save()
    const original = JSON.stringify(editor.pending.value); assert.equal(editor.rejected.value, true); assert.equal(typeof editor.newInput, 'function')
    storageFails = true; assert.equal(await editor.newInput(), false); assert.equal(JSON.stringify(editor.pending.value), original); assert.equal(editor.name.value, '가상 처리된 원입력')
    storageFails = false; assert.equal(await editor.newInput(), true); assert.equal(calls.filter(value => value.path === '/api/materials').length, 1); assert.equal(editor.pending.value, null); assert.equal(editor.name.value, ''); assert.equal(editor.unit.value, 'g'); assert.equal(editor.rejected.value, false)
    const retained = [...storage.entries()].find(([key]) => key.startsWith('sunchang.retained.material.')); assert.ok(retained); assert.deepEqual(JSON.parse(retained[1]).pending, JSON.parse(original))
    editor.name.value = '가상 별도 새입력'; await editor.save(); const writes = calls.filter(value => value.path === '/api/materials'); assert.equal(writes.length, 2); assert.notEqual(writes[1]!.body.request_id, writes[0]!.body.request_id); assert.equal(writes[1]!.body.epoch, newEpoch); assert.equal(writes[1]!.body.expected_revision, '9'); assert.equal(writes[1]!.body.name, '가상 별도 새입력')
  } finally { scope.stop() }
})
