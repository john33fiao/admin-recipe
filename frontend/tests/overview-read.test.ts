import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createContext, runInContext } from 'node:vm'
import ts from 'typescript'
import * as vue from 'vue'
import { ApiError } from '../app/utils/api.ts'
import * as helpers from '../app/utils/overview.ts'
import type { OverviewTarget } from '../app/types/overview.ts'
const settle = () => new Promise(resolve => setImmediate(resolve))
const epoch = '10000000-0000-4000-8000-000000000001', material = '20000000-0000-4000-8000-000000000001'
const state = (revision = '1', key = epoch) => ({ epoch: key, revision })
const target = (section: OverviewTarget['section'] = 'materials', offset = '0', material_id: string | null = null): OverviewTarget => ({ section, offset, material_id })
function response(wanted = target(), basis = state(), ids = ['first']) {
  return { state: basis, scope: { section: wanted.section, material_id: wanted.material_id }, list: { section: wanted.section, material: { id: wanted.material_id }, page: { total_count: 26, returned_count: ids.length, offset: Number(wanted.offset), page_size: wanted.section === 'materials' ? 25 : 20, next_offset: wanted.offset === '0' ? 25 : null, rows: ids.map(id => ({ id })) } } } as any
}
function harness(name: 'useOverviewList' | 'useOperationsOverview') {
  const connection = vue.ref('connected'), requests: Array<{ path: string; resolve: (data: any) => void; reject: (cause: unknown) => void }> = []
  const context = createContext({ ...vue, ...helpers, ApiError, useConnection: () => connection, useState: () => { throw Error('readonly reader must not create editor state') }, apiRequest: (path: string, body?: unknown) => { assert.equal(body, undefined); return new Promise((resolve, reject) => requests.push({ path, resolve, reject })) } })
  const source = readFileSync(new URL(`../app/composables/${name}.ts`, import.meta.url), 'utf8').replace(/^import .*$/gm, '').replace(/export function /g, 'function ')
  runInContext(ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText, context)
  return { connection, requests, context }
}
test('overview URLs and page acceptance keep canonical scope, epoch, revision and exact page together', () => {
  assert.deepEqual(helpers.overviewTarget({ section: 'materials' }), target())
  assert.throws(() => helpers.overviewTarget({ section: 'materials', offset: '9007199254740993' }), /목록 위치/)
  assert.equal(helpers.overviewTarget({ section: 'materials', offset: '9007199254740991' }).offset, '9007199254740991')
  for (const query of [{ section: 'other' }, { section: ['materials'] }, { section: 'materials', offset: '01' }, { section: 'materials', material_id: material }, { section: 'material_changes' }, { section: 'materials', limit: '10' }]) assert.throws(() => helpers.overviewTarget(query))
  const wanted = target('material_changes', '20', material), result = response(wanted)
  assert.equal(helpers.overviewResponseMatches(result, wanted, state()), true)
  assert.equal(helpers.overviewResponseMatches(result, wanted, state('1', material)), false)
  assert.equal(helpers.overviewResponseMatches(result, wanted, state('2')), false)
  assert.equal(helpers.overviewResponseMatches(result, { ...wanted, material_id: epoch }, state()), false)
  assert.equal(helpers.overviewResponseMatches(result, { ...wanted, section: 'sales' }, state()), false)
  assert.equal(helpers.overviewResponseMatches(result, { ...wanted, offset: '0' }, state()), false)
  const unsafe = response(wanted); unsafe.list.page.next_offset = 9007199254740992
  assert.equal(helpers.overviewResponseMatches(unsafe, wanted, state()), false)
  assert.match(helpers.overviewPageText(result.list.page), /전체 26건 중 21–21건/)
  assert.match(helpers.overviewListApi(wanted, state('2')), /epoch=.*&revision=2&material_id=/)
})
test('overview list rejects a different scope/state and late pages without replacing the previous exact count', async () => {
  const h = harness('useOverviewList'), scope = vue.effectScope(), selected = vue.shallowRef(target())
  const reader = scope.run(() => h.context.useOverviewList(selected, state()))!
  try {
    h.requests[0]!.resolve(response()); await settle()
    selected.value = target('materials', '25'); const late = h.requests[1]!
    selected.value = target('sales'); h.requests[2]!.resolve(response(target('sales'), state(), ['sale'])); await settle()
    late.resolve(response(target('materials', '25'), state(), ['last-material'])); await settle()
    assert.equal(reader.snapshot.value.response.list.section, 'sales')
    selected.value = target('material_changes', '0', material)
    h.requests[3]!.resolve(response(target('material_changes', '0', epoch))); await settle()
    assert.equal(reader.snapshot.value.response.list.section, 'sales'); assert.equal(reader.oldResult.value, true)
    void reader.load(); h.requests[4]!.resolve(response(selected.value, state('2'))); await settle()
    assert.equal(reader.snapshot.value.response.list.page.total_count, 26); assert.match(reader.failure.value, /같은 조회/)
  } finally { scope.stop() }
})
test('overview 409 preserves the old page until explicit summary refresh then starts the first page', async () => {
  const h = harness('useOverviewList'), scope = vue.effectScope(), selected = vue.shallowRef(target())
  const reader = scope.run(() => h.context.useOverviewList(selected, state()))!
  try {
    h.requests[0]!.resolve(response()); await settle()
    selected.value = target('materials', '25'); h.requests[1]!.reject(new ApiError(409, 'changed', 'stale_state')); await settle()
    assert.equal(reader.stale.value, true); assert.equal(reader.snapshot.value.response.list.page.offset, 0)
    await reader.load(); assert.equal(h.requests.length, 2)
    const refresh = reader.restart(); assert.equal(h.requests[2]!.path, '/api/overview')
    h.requests[2]!.resolve({ state: state('2') }); await settle()
    assert.match(h.requests[3]!.path, /offset=0&epoch=.*&revision=2/)
    h.requests[3]!.resolve(response(target(), state('2'), ['new-first'])); assert.equal(await refresh, true)
    assert.equal(reader.stale.value, false); assert.equal(reader.snapshot.value.response.state.revision, '2')
    selected.value = target(); h.requests[4]!.resolve(response(target(), state('2'), ['new-first'])); await settle()
    assert.equal(reader.oldResult.value, false)
  } finally { scope.stop() }
})
test('fresh overview list reads a summary once and ignores changed scope, disconnect and disposed responses', async () => {
  const h = harness('useOverviewList'), scope = vue.effectScope(), selected = vue.shallowRef(target())
  const reader = scope.run(() => h.context.useOverviewList(selected))!
  assert.equal(h.requests[0]!.path, '/api/overview')
  selected.value = target('sales'); h.requests[0]!.resolve({ state: state('1') }); h.requests[1]!.resolve({ state: state('2') }); await settle()
  assert.equal(h.requests.length, 3); assert.match(h.requests[2]!.path, /section=sales/)
  h.connection.value = 'unavailable'; h.requests[2]!.resolve(response(target('sales'), state('2'))); await settle()
  assert.equal(reader.snapshot.value, null); assert.equal(reader.loading.value, false)
  h.connection.value = 'connected'; scope.stop(); h.requests[3]!.resolve({ state: state('3') }); await settle()
  assert.equal(h.requests.length, 4); assert.equal(reader.snapshot.value, null)
})
test('overview summary preserves complete previous data on failure and cancels older or disconnected GETs', async () => {
  const h = harness('useOperationsOverview'), scope = vue.effectScope(), reader = scope.run(() => h.context.useOperationsOverview())!
  try {
    const old = h.requests[0]!; void reader.load(); h.requests[1]!.resolve({ state: state('2'), materials: { total_count: 26, rows: ['current'] } }); await settle()
    old.resolve({ state: state(), materials: { total_count: 2, rows: ['old'] } }); await settle()
    assert.equal(reader.snapshot.value.report.materials.total_count, 26)
    void reader.load(); h.requests[2]!.reject(Error('offline')); await settle()
    assert.equal(reader.snapshot.value.report.materials.rows[0], 'current'); assert.equal(reader.oldResult.value, true)
    void reader.load(); h.connection.value = 'unavailable'; h.requests[3]!.resolve({ state: state('3') }); await settle()
    assert.equal(reader.snapshot.value.report.state.revision, '2'); assert.equal(reader.loading.value, false)
    h.connection.value = 'connected'; h.requests[4]!.resolve({ state: state('4') }); await settle()
    assert.equal(reader.snapshot.value.report.state.revision, '4'); assert.equal(reader.oldResult.value, false)
  } finally { scope.stop() }
})

test('internal list navigation leaves no handoff that can pin a later fresh entry to an old state', () => {
  const handoff = vue.ref<any>({ url: helpers.overviewListUrl(target()), state: state() }), bases: any[] = []
  const source = readFileSync(new URL('../app/pages/overview/list.vue', import.meta.url), 'utf8').split('<script setup lang="ts">')[1]!.split('</script>')[0]!.replace(/^import .*$/gm, '')
  function mount(wanted: OverviewTarget) {
    const route = vue.reactive({ query: { section: wanted.section, offset: wanted.offset, ...(wanted.material_id ? { material_id: wanted.material_id } : {}) } })
    const scope = vue.effectScope(), context = createContext({ ...vue, ...helpers, formatCreatedAt: () => '', useRoute: () => route, useRouter: () => ({ replace: async () => {} }), useState: () => handoff, useHead: () => {}, useOverviewList: (_: unknown, initial: unknown) => { bases.push(initial); return { snapshot: vue.shallowRef({ response: response(wanted), receivedAt: '' }), loading: vue.ref(false), failure: vue.ref(''), stale: vue.ref(false), connection: vue.ref('connected'), oldResult: vue.ref(false), load: async () => {}, restart: async () => true } } })
    scope.run(() => runInContext(ts.transpileModule(source + '\nglobalThis.follow = typeof openList === "function" ? openList : () => {};', { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText, context))
    return { scope, context, route }
  }
  const first = mount(target())
  assert.equal(bases[0].revision, '1'); assert.equal(handoff.value, null)
  first.context.follow({ section: 'material_changes', material_id: material }, 0)
  first.route.query = { section: 'material_changes', offset: '0', material_id: material }; first.scope.stop()
  const fresh = mount(target('material_changes', '0', material))
  try { assert.equal(bases[1], null, 'a fresh entry must fetch its own latest summary') } finally { fresh.scope.stop() }
})

test('list automatic rereads preserve focused controls while entry, page, scope and explicit refresh focus the heading', async () => {
  const snapshot = vue.shallowRef<any>(null), scope = vue.effectScope()
  const route = vue.reactive({ query: { section: 'materials', offset: '0' } })
  let active = 'unfocused', focusCount = 0
  const heading = { focus() { active = 'heading'; focusCount++ } }
  const source = readFileSync(new URL('../app/pages/overview/list.vue', import.meta.url), 'utf8').split('<script setup lang="ts">')[1]!.split('</script>')[0]!.replace(/^import .*$/gm, '')
  const context = createContext({ ...vue, ...helpers, formatCreatedAt: () => '', useRoute: () => route, useRouter: () => ({ replace: async () => {} }), useState: () => vue.ref(null), useHead: () => {}, useOverviewList: () => ({ snapshot, loading: vue.ref(false), failure: vue.ref(''), stale: vue.ref(false), connection: vue.ref('connected'), oldResult: vue.ref(false), load: async () => true, restart: async () => true }) })
  scope.run(() => runInContext(ts.transpileModule(source + '\nglobalThis.heading = heading; globalThis.refresh = refresh;', { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText, context))
  context.heading.value = heading
  function publish(wanted = target(), basis = state()) { snapshot.value = { response: response(wanted, basis), receivedAt: new Date().toISOString() } }
  try {
    publish(); await settle(); assert.equal(active, 'heading'); assert.equal(focusCount, 1)
    active = 'back-link'; publish(); await settle(); assert.equal(active, 'back-link', 'same-page reconnect must preserve user focus')
    publish(target(), state('2')); await settle(); assert.equal(active, 'back-link', 'a reread with a changed revision is still the same page')
    publish(target('materials', '25')); await settle(); assert.equal(active, 'heading')
    active = 'page-link'; publish(target('material_changes', '0', material)); await settle(); assert.equal(active, 'heading')
    active = 'other-material-link'; publish(target('material_changes', '0', epoch)); await settle(); assert.equal(active, 'heading')
    active = 'refresh'; await context.refresh(); await settle(); assert.equal(active, 'heading')
  } finally { scope.stop() }
})
