import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createContext, runInContext } from 'node:vm'
import ts from 'typescript'
import * as vue from 'vue'
import * as business from '../app/utils/business.ts'
import * as helpers from '../app/utils/workspace.ts'
import { ApiError } from '../app/utils/api.ts'
import { failureAction } from '../app/utils/catalog.ts'
import { currentAppState } from '../app/utils/current-state.ts'
const epoch = '00000000-0000-4000-8000-000000000001', recordId = '00000000-0000-4000-8000-000000000002', headId = '00000000-0000-4000-8000-000000000003'
type Request = { path: string; body: any; resolve: (value: unknown) => void; reject: (value: unknown) => void }
function harness() {
  const storage = new Map<string, string>(), states = new Map<string, vue.Ref>(), requests: Request[] = []
  let sequence = 10, storageFails = false
  const context = createContext({ ...vue, ...business, ...helpers, ApiError, failureAction, currentAppState,
    useConnection: () => vue.ref('connected'), useState: (key: string, init: () => unknown) => { if (!states.has(key)) states.set(key, vue.ref(init())); return states.get(key) },
    sessionStorage: { getItem: (key: string) => storage.get(key) ?? null, setItem: (key: string, value: string) => { if (storageFails) throw Error('full'); storage.set(key, value) }, removeItem: (key: string) => storage.delete(key) },
    crypto: { randomUUID: () => `00000000-0000-4000-8000-${String(++sequence).padStart(12, '0')}` },
    apiRequest: (path: string, body: unknown) => new Promise((resolve, reject) => requests.push({ path, body, resolve, reject })),
  })
  for (const file of ['useBusinessDraft', 'useBusinessWorkspace']) {
    const source = readFileSync(new URL(`../app/composables/${file}.ts`, import.meta.url), 'utf8').replace(/^import .*$/gm, '').replace(/export function /g, 'function ').replace(/export interface /g, 'interface ')
    runInContext(ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText, context)
  }
  return { storage, requests, context, failStorage: () => { storageFails = true } }
}
const settle = () => new Promise(resolve => setImmediate(resolve))
function mutation() { return { result: { item: { id: recordId, head_id: headId, status: 'draft', kind: 'sale', corrects_confirmation_id: null, material_name: '가상 메뉴', issue_count: 0 }, head: { id: headId, record_id: recordId, status: 'draft', payload: helpers.blankWorkspacePayload('sale') }, issues: [], confirmation: null }, state: { epoch, revision: '2' } } }

test('workspace maps a late create to its own item and ignores old local notifications even with unavailable tab storage', async () => {
  const h = harness(), scope = vue.effectScope()
  try {
    const work = scope.run(() => h.context.useBusinessWorkspace())!
    h.requests[0]!.resolve({ items: [], state: { epoch, revision: '30' } }); await settle()
    const a = work.add('sale'), b = work.add('sale'), host = work.hostFor(a.identity)
    h.failStorage()
    assert.doesNotThrow(() => host.committed(a.identity, mutation()))
    host.notify(a.identity, { payload: helpers.blankWorkspacePayload('sale'), status: 'draft', pendingAction: null })
    assert.equal(work.tab.value.selected, b.identity)
    assert.equal(work.tab.value.entries[0].identity, recordId)
    assert.equal(work.snapshots.value[a.identity], undefined)
    assert.equal(work.latestState.value.revision, '30')
    assert.match(work.storageFailure.value, /보관하지 못했습니다/)
  } finally { scope.stop() }
})

test('a hostless detail confirm completed after workspace entry clears the prior gate on fresh list review', async () => {
  const h = harness(), normalScope = vue.effectScope(), workspaceScope = vue.effectScope()
  try {
    const payload = helpers.blankWorkspacePayload('sale')
    const editor = normalScope.run(() => h.context.useBusinessDraft(recordId, { payload, state: { epoch, revision: '1' }, recordId, headId, status: 'draft', corrects: null, issues: [] }, async () => null))!
    const preview = editor.showPreview(); h.requests[0]!.resolve({ record_id: recordId, head_id: headId, state: { epoch, revision: '1' }, can_confirm: true, issues: [], effect: { kind: 'sale' } }); await preview
    const confirming = editor.send('confirm')
    normalScope.stop()
    h.storage.set('sunchang.workspace.v1', JSON.stringify({ entries: [{ identity: recordId, recordId, kind: 'sale' }], selected: null }))
    const work = workspaceScope.run(() => h.context.useBusinessWorkspace())!
    h.requests[2]!.resolve({ items: [mutation().result.item], state: { epoch, revision: '1' } }); await settle()
    const stock = work.add('stocktake')
    assert.equal(work.hostFor(stock.identity).awaitingPrior(), true, JSON.stringify({ entries: work.tab.value.entries, snapshot: work.snapshots.value[recordId], pending: h.storage.get(`sunchang.business.${recordId}.pending.v1`) }))
    const done = mutation(); done.result.head.status = 'confirmed'; done.result.item.status = 'confirmed'
    h.requests[1]!.resolve(done); await confirming
    assert.equal(h.storage.has(`sunchang.business.${recordId}.pending.v1`), false)
    const reading = work.loadList(); h.requests.at(-1)!.resolve({ items: [done.result.item], state: done.state }); await reading
    assert.equal(work.hostFor(stock.identity).awaitingPrior(), false)
    assert.equal(work.snapshots.value[recordId].status, 'confirmed')
  } finally { normalScope.stop(); workspaceScope.stop() }
})

test('replayed local create adopts an existing record without clearing that record dirty input or uncertain revision', async () => {
  const h = harness(), scope = vue.effectScope()
  try {
    const work = scope.run(() => h.context.useBusinessWorkspace())!
    h.requests[0]!.resolve({ items: [mutation().result.item], state: { epoch, revision: '8' } }); await settle()
    const local = work.add('sale'), actual = { identity: recordId, recordId, kind: 'sale' }
    const latest = mutation(); latest.result.head.id = epoch; latest.result.head.payload.data.count = '5'; latest.state.revision = '8'
    const selecting = work.select(actual)
    h.requests.at(-1)!.resolve({ item: latest.result.item, head: latest.result.head, state: latest.state, issues: [], revisions: [], confirmations: [], corrections: [] }); await selecting
    const editor = scope.run(() => h.context.useBusinessDraft(recordId, work.initial.value, async () => null, work.hostFor(recordId)))!
    editor.payload.value.data.count = '7'
    const revising = editor.send('revise'), request = h.requests.at(-1)!, original = JSON.stringify(request.body)
    request.reject(new ApiError(503, 'uncertain', 'unavailable')); await revising
    work.hostFor(local.identity).committed(local.identity, mutation())
    assert.equal(work.snapshots.value[recordId].pendingAction, 'revise')
    assert.equal(work.snapshots.value[recordId].dirty, true)
    assert.equal(work.snapshots.value[recordId].payload.data.count, '7')
    assert.equal(work.selectedDetail.value.head.id, epoch)
    assert.equal(JSON.stringify(JSON.parse(h.storage.get(`sunchang.business.${recordId}.pending.v1`)!).body), original)
    assert.equal(work.tab.value.selected, recordId)
    assert.equal(work.tab.value.entries.filter((entry: any) => entry.recordId === recordId).length, 1)
  } finally { scope.stop() }
})

function detail() { const value = mutation(); return { ...value.result, state: value.state, revisions: [], confirmations: [], corrections: [] } }

test('late create after workspace remount loads the newly selected actual record in the live workspace', async () => {
  const h = harness(), oldScope = vue.effectScope(), liveScope = vue.effectScope()
  try {
    const old = oldScope.run(() => h.context.useBusinessWorkspace())!
    h.requests[0]!.resolve({ items: [], state: { epoch, revision: '1' } }); await settle()
    const local = old.add('sale')
    const editor = oldScope.run(() => h.context.useBusinessDraft(local.identity, old.initial.value, old.refreshFor(local), old.hostFor(local.identity)))!
    editor.payload.value.data.count = '1'
    const sending = editor.send('create'), write = h.requests.at(-1)!
    oldScope.stop()
    const live = liveScope.run(() => h.context.useBusinessWorkspace())!
    h.requests.at(-1)!.resolve({ items: [mutation().result.item], state: { epoch, revision: '2' } }); await settle()
    liveScope.run(() => h.context.useBusinessDraft(local.identity, live.initial.value, live.refreshFor(local), live.hostFor(local.identity)))!
    const before = h.requests.length
    write.resolve(mutation()); await sending; await settle()
    for (const request of h.requests.slice(before)) {
      if (request.path === '/api/business?status=all') request.resolve({ items: [mutation().result.item], state: { epoch, revision: '2' } })
      else if (request.path === `/api/business/${recordId}`) request.resolve(detail())
    }
    await settle()
    assert.equal(live.selected.value.identity, recordId)
    assert.ok(live.initial.value, `selected=${live.selected.value.identity}; post-response GETs=${h.requests.slice(before).map(item => item.path).join(',')}`)
    assert.equal(live.initial.value.recordId, recordId)
  } finally { oldScope.stop(); liveScope.stop() }
})

test('inactive dirty in-memory input remains marked unsaved if session storage failed', async () => {
  const h = harness(), scope = vue.effectScope()
  try {
    const work = scope.run(() => h.context.useBusinessWorkspace())!
    h.requests[0]!.resolve({ items: [mutation().result.item], state: { epoch, revision: '2' } }); await settle()
    const other = work.add('stocktake')
    const item = { identity: recordId, recordId, kind: 'sale' }
    const selecting = work.select(item); h.requests.at(-1)!.resolve(detail()); await selecting
    const editor = scope.run(() => h.context.useBusinessDraft(recordId, work.initial.value, work.refreshFor(item), work.hostFor(recordId)))!
    h.failStorage(); editor.payload.value.data.count = '7'
    assert.equal(work.snapshots.value[recordId].dirty, true)
    await work.select(other)
    const reading = work.loadList(); h.requests.at(-1)!.resolve({ items: [mutation().result.item], state: { epoch, revision: '2' } }); await reading
    assert.equal(editor.dirty.value, true)
    assert.equal(work.snapshots.value[recordId].payload.data.count, '7')
    assert.equal(work.snapshots.value[recordId].dirty, true)
  } finally { scope.stop() }
})
