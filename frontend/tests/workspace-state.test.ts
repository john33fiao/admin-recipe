import { currentAppState } from '../app/utils/current-state.ts'
import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createContext, runInContext } from 'node:vm'
import ts from 'typescript'
import * as vue from 'vue'
import * as business from '../app/utils/business.ts'
import { blankSaleDraft } from '../app/utils/sale.ts'
import { blankStocktakeDraft } from '../app/utils/stocktake.ts'
import { ApiError } from '../app/utils/api.ts'
import { failureAction } from '../app/utils/catalog.ts'

const epoch = '00000000-0000-4000-8000-000000000001'
const recordId = '00000000-0000-4000-8000-000000000002'
const headId = '00000000-0000-4000-8000-000000000003'
type Request = { path: string; body: any; resolve: (value: unknown) => void; reject: (value: unknown) => void }
function harness() {
  const states = new Map<string, vue.Ref>(), storage = new Map<string, string>(), requests: Request[] = []
  let sequence = 10
  const context = createContext({ currentAppState, ...vue, ...business, ApiError, failureAction,
    useConnection: () => vue.ref('connected'),
    useState: (key: string, init: () => unknown) => { if (!states.has(key)) states.set(key, vue.ref(init())); return states.get(key) },
    sessionStorage: { getItem: (key: string) => storage.get(key) ?? null, setItem: (key: string, value: string) => storage.set(key, value), removeItem: (key: string) => storage.delete(key) },
    crypto: { randomUUID: () => `00000000-0000-4000-8000-${String(++sequence).padStart(12, '0')}` },
    apiRequest: (path: string, body: unknown) => new Promise((resolve, reject) => requests.push({ path, body, resolve, reject })),
  })
  const source = readFileSync(new URL('../app/composables/useBusinessDraft.ts', import.meta.url), 'utf8').replace(/^import .*$/gm, '').replace('export function useBusinessDraft', 'function useBusinessDraft').replace(/export interface /g, 'interface ')
  runInContext(ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText, context)
  return { create: context.useBusinessDraft, storage, requests }
}
function seed(record: string | null = null, payload: any = blankSaleDraft()) { return { payload, state: { epoch, revision: '1' }, recordId: record, headId: record ? headId : null, status: 'draft', corrects: null, issues: [] } }
function response(payload: any, revision = '2') { return { result: { item: { id: recordId, head_id: headId, kind: payload.kind, status: 'draft', material_name: null, issue_count: 0 }, head: { id: headId, record_id: recordId, status: 'draft', payload }, issues: [], confirmation: null }, state: { epoch, revision } } }
function host() {
  const events: any[] = [], mutations: any[] = [], latest = vue.ref({ epoch, revision: '1' }), prior = vue.ref(false)
  return { events, mutations, latest, prior, api: { notify: (identity: string, value: unknown) => events.push({ identity, value }), committed: (identity: string, value: unknown) => mutations.push({ identity, value }), currentState: () => latest.value, awaitingPrior: () => prior.value } }
}

test('workspace host receives isolated input and a late successful create after switching editors', async () => {
  const h = harness(), a = host(), firstScope = vue.effectScope(), otherScope = vue.effectScope()
  try {
    const first = firstScope.run(() => h.create('workspace.local.a', seed(), async () => null, a.api))!
    const other = otherScope.run(() => h.create('workspace.local.b', seed(), async () => null, a.api))!
    first.payload.value.data.count = '3'
    other.payload.value.data.count = '0'
    await vue.nextTick()
    assert.equal(a.events.findLast(event => event.identity === 'workspace.local.a')?.value.payload.data.count, '3')
    assert.equal(a.events.findLast(event => event.identity === 'workspace.local.b')?.value.payload.data.count, '0')
    const sending = first.send('create'), sent = JSON.stringify(h.requests[0]!.body)
    firstScope.stop()
    h.requests[0]!.resolve(response(JSON.parse(sent).payload))
    await sending
    assert.equal(a.mutations.length, 1)
    assert.equal(a.mutations[0].identity, 'workspace.local.a')
    assert.equal(a.mutations[0].value.result.item.id, recordId)
    assert.equal(a.mutations[0].value.result.head.payload.data.count, '3')
    assert.equal(other.payload.value.data.count, '0')
    assert.equal(h.storage.has('sunchang.business.workspace.local.a.pending.v1'), false)
  } finally { firstScope.stop(); otherScope.stop() }
})

test('workspace invalidates an older preview without silently replacing its revision', async () => {
  const h = harness(), a = host(), scope = vue.effectScope()
  try {
    const editor = scope.run(() => h.create(recordId, seed(recordId), async () => null, a.api))!
    const showing = editor.showPreview()
    h.requests[0]!.resolve({ record_id: recordId, head_id: headId, state: { epoch, revision: '1' }, can_confirm: true, issues: [], effect: { kind: 'sale' } })
    await showing
    a.latest.value = { epoch, revision: '2' }
    const pending = editor.send('confirm')
    assert.equal(h.requests.length, 1, 'no confirm may be sent using an obsolete preview')
    assert.equal(await pending, null)
    assert.equal(editor.previewOutdated.value, true)
    assert.equal(editor.preview.value.state.revision, '1')
  } finally { scope.stop() }
})

test('workspace waits for preceding uncertain writes but retries its already sent confirm unchanged', async () => {
  const h = harness(), a = host(), scope = vue.effectScope()
  try {
    const editor = scope.run(() => h.create(recordId, seed(recordId, blankStocktakeDraft()), async () => null, a.api))!
    a.prior.value = true
    const blocked = editor.showPreview()
    assert.equal(h.requests.length, 0, 'stocktake waits for the preceding request result')
    assert.equal(await blocked, false)
    a.prior.value = false
    const showing = editor.showPreview()
    h.requests[0]!.resolve({ record_id: recordId, head_id: headId, state: { epoch, revision: '1' }, can_confirm: true, issues: [], effect: { kind: 'stocktake' } })
    await showing
    const sending = editor.send('confirm'), sent = JSON.stringify(h.requests[1]!.body)
    h.requests[1]!.reject(new ApiError(503, 'uncertain', 'unavailable')); await sending
    a.latest.value = { epoch, revision: '9' }; a.prior.value = true
    const retry = editor.send('confirm')
    assert.equal(JSON.stringify(h.requests[2]!.body), sent)
    h.requests[2]!.reject(new ApiError(503, 'uncertain', 'unavailable')); await retry
    assert.equal(JSON.stringify(editor.pending.value.body), sent)
  } finally { scope.stop() }
})
