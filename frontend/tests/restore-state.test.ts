import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createContext, runInContext } from 'node:vm'
import ts from 'typescript'
import * as vue from 'vue'
import * as restore from '../app/utils/restore.ts'
import { ApiError } from '../app/utils/api.ts'
import { failureAction } from '../app/utils/catalog.ts'

const a = '10000000-0000-4000-8000-000000000001', b = '10000000-0000-4000-8000-000000000002'
const state = { epoch: a, revision: '4' }, sha = 'a'.repeat(64)
const target = { target_id: 'journal:2', source: 'journal', source_kind: 'journal', sha256: sha, byte_length: '100', format_version: 2, schema_version: 14, sequence: '2', request_id: a, recorded_at: '2026-01-01T00:00:00Z' }
const preview = { target, state, can_restore: true, before: { areas: [], drafts: 0, photo_bytes: '0' }, after: { areas: [], drafts: 0, photo_bytes: '0' }, photos: { required: 0, verified: 0, missing: 0, corrupt: 0, unsafe_path: 0, examples: [], examples_truncated: false }, blockers: [] }
function harness() {
  const states = new Map<string, vue.Ref>(), storage = new Map<string, string>(), current = vue.ref({ ...state }), requests: Array<{ path: string; body: any; resolve: (value: any) => void; reject: (error: unknown) => void }> = []
  let writeFails = false, serial = 1
  const context = createContext({ ...vue, ...restore, ApiError, failureAction, currentAppState: current, announceRestore: () => {}, crypto: { randomUUID: () => `20000000-0000-4000-8000-${String(serial++).padStart(12, '0')}` }, useConnection: () => vue.ref('connected'), sessionStorage: { getItem: (key: string) => storage.get(key) ?? null, setItem: (key: string, value: string) => { if (writeFails) throw Error('quota'); storage.set(key, value) }, removeItem: (key: string) => storage.delete(key) }, useState: (key: string, init: () => unknown) => { if (!states.has(key)) states.set(key, vue.ref(init())); return states.get(key) }, apiRequest: (path: string, body?: unknown) => path.startsWith('/api/restore-targets?') ? Promise.resolve({ state: current.value, source: path.includes('source=import') ? 'import' : 'journal', rows: [], next_cursor: null }) : path === '/api/state' ? Promise.resolve(current.value) : new Promise((resolve, reject) => requests.push({ path, body, resolve, reject })) })
  const source = readFileSync(new URL('../app/composables/useRestore.ts', import.meta.url), 'utf8').replace(/^import .*$/gm, '').replace('export function useRestore', 'function useRestore')
  runInContext(ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText, context)
  const mount = () => { const scope = vue.effectScope(); return { work: scope.run(() => context.useRestore()), stop: () => scope.stop() } }
  return { mount, current, storage, requests, failStorage: () => { writeFails = true } }
}
async function prepared(h: ReturnType<typeof harness>, work: any) { work.choose(target); const pending = work.showPreview(); h.requests.at(-1)!.resolve(preview); assert.equal(await pending, true) }

test('uncertain restore replays the identical UUID, target and metadata after editor remount', async () => {
  const h = harness(), first = h.mount(); await prepared(h, first.work)
  const saving = first.work.confirm(), original = JSON.stringify(h.requests.at(-1)!.body)
  h.requests.at(-1)!.reject(new TypeError('lost-response')); await saving; first.stop()
  const second = h.mount(); assert.equal(JSON.stringify(second.work.pending.value), original)
  const retry = second.work.confirm(); assert.equal(JSON.stringify(h.requests.at(-1)!.body), original)
  h.requests.at(-1)!.resolve({ restore_id: b, target, previous_target: target, recorded_at: target.recorded_at, state: { epoch: b, revision: '5' } }); await retry
  assert.equal(second.work.pending.value, null); assert.equal(h.storage.has('sunchang.restore.pending.v1'), false); second.stop()
})
test('an already-processed generation failure preserves the request and cannot start another confirm', async () => {
  const h = harness(), instance = h.mount(); await prepared(h, instance.work)
  const saving = instance.work.confirm(), original = JSON.stringify(instance.work.pending.value)
  h.requests.at(-1)!.reject(new ApiError(409, '', 'stale_generation', { request_status: 'already_processed', state: { epoch: b, revision: '9' } })); await saving
  assert.equal(instance.work.canReview.value, false); assert.equal(instance.work.canRetry.value, false)
  const count = h.requests.length; assert.equal(await instance.work.confirm(), false); assert.equal(h.requests.length, count); assert.equal(JSON.stringify(instance.work.pending.value), original)
  instance.stop()
})
test('not-processed restore requires explicit successful preview before a new UUID is prepared', async () => {
  const h = harness(), instance = h.mount(); await prepared(h, instance.work)
  const saving = instance.work.confirm(), original = instance.work.pending.value.meta.request_id
  h.requests.at(-1)!.reject(new ApiError(409, '', 'stale_generation', { request_status: 'not_processed', state: { epoch: b, revision: '9' } })); await saving
  assert.equal(instance.work.canReview.value, true); assert.equal(await instance.work.confirm(), false)
  h.current.value = { epoch: b, revision: '9' }
  const review = instance.work.showPreview(true); h.requests.at(-1)!.resolve({ ...preview, state: h.current.value }); await review
  assert.equal(instance.work.pending.value, null); assert.equal(h.storage.has(`sunchang.restore.reviewed.${original}`), true)
  const second = instance.work.confirm(); assert.notEqual(h.requests.at(-1)!.body.meta.request_id, original); assert.equal(h.requests.at(-1)!.body.meta.epoch, b)
  h.requests.at(-1)!.reject(Error('uncertain')); await second; instance.stop()
})
test('storage failure sends no restore and a changed generation invalidates a shown preview', async () => {
  const h = harness(), instance = h.mount(); await prepared(h, instance.work)
  h.current.value = { epoch: b, revision: '5' }; assert.equal(instance.work.canConfirm.value, false)
  const count = h.requests.length; assert.equal(await instance.work.confirm(), false); assert.equal(h.requests.length, count)
  h.current.value = { ...state }; h.failStorage(); assert.equal(await instance.work.confirm(), false); assert.equal(h.requests.length, count); assert.equal(instance.work.pending.value, null); instance.stop()
})
test('late preview cannot populate a new selection and malformed persisted requests never become new requests', async () => {
  const h = harness(), instance = h.mount(); instance.work.choose(target); const loading = instance.work.showPreview(); instance.stop()
  h.requests.at(-1)!.resolve(preview); await loading
  const next = h.mount(); assert.equal(next.work.preview.value, null); next.stop()
  assert.equal(restore.restorePendingRequest(JSON.stringify({ target_id: `import:${sha}`, expected_sha256: 'b'.repeat(64), meta: { request_id: a, epoch: b, expected_revision: '1' } })), null)
  assert.equal(restore.restorePendingRequest(JSON.stringify({ target_id: 'journal:0', expected_sha256: sha, meta: { request_id: a, epoch: b, expected_revision: '1' } })), null)
})
