import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createContext, runInContext } from 'node:vm'
import ts from 'typescript'
import * as vue from 'vue'
import * as business from '../app/utils/business.ts'
import { failureAction } from '../app/utils/catalog.ts'
import { ApiError } from '../app/utils/api.ts'

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
