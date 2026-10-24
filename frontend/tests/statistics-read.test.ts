import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createContext, runInContext } from 'node:vm'
import ts from 'typescript'
import * as vue from 'vue'
import { ApiError } from '../app/utils/api.ts'
import { statisticsPath } from '../app/utils/statistics.ts'
const settle = () => new Promise(resolve => setImmediate(resolve))
const range = (day: string) => ({ from: `2026-01-${day}T00:00:00Z`, until: `2026-02-${day}T00:00:00Z`, zone: 'UTC' })
function harness() {
  const connection = vue.ref('connected'), requests: Array<{ path: string; resolve: (value: any) => void; reject: (value: unknown) => void }> = []
  const context = createContext({ ...vue, ApiError, statisticsPath, useConnection: () => connection,
    useState: () => { throw Error('report must not write editor state') },
    apiRequest: (path: string, body?: unknown) => { assert.equal(body, undefined); return new Promise((resolve, reject) => requests.push({ path, resolve, reject })) },
  })
  const source = readFileSync(new URL('../app/composables/useStatistics.ts', import.meta.url), 'utf8').replace(/^import .*$/gm, '').replace(/export function /g, 'function ')
  runInContext(ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText, context)
  return { context, requests, connection }
}
test('statistics ignores late range responses and retains the previous applied period on a failed new query', async () => {
  const h = harness(), scope = vue.effectScope(), selected = vue.shallowRef(range('01'))
  try {
    const reader = scope.run(() => h.context.useStatistics(selected))!, first = h.requests[0]!
    selected.value = range('02'); const second = h.requests[1]!
    second.resolve({ state: { revision: '2' }, sales: { rows: [] } }); await settle()
    first.resolve({ state: { revision: '1' }, sales: { rows: [] } }); await settle()
    assert.equal(reader.snapshot.value.report.state.revision, '2'); assert.equal(reader.snapshot.value.range.from, range('02').from)
    selected.value = range('03'); h.requests[2]!.reject(new ApiError(400, 'too many', 'report_too_large')); await settle()
    assert.equal(reader.snapshot.value.range.from, range('02').from); assert.match(reader.failure.value, /기간을 좁혀/); assert.equal(reader.loading.value, false)
    // Browser back applies its saved absolute range, not the failed date inputs.
    selected.value = range('01'); assert.equal(h.requests[3]!.path, statisticsPath(range('01')))
    h.requests[3]!.resolve({ state: { revision: '3' } }); await settle(); assert.equal(reader.snapshot.value.range.from, range('01').from)
  } finally { scope.stop() }
})
test('disconnect or disposal invalidates in-flight statistics without publishing a mixed state', async () => {
  const h = harness(), scope = vue.effectScope(), selected = vue.shallowRef(range('01'))
  const reader = scope.run(() => h.context.useStatistics(selected))!, request = h.requests[0]!
  h.connection.value = 'unavailable'; request.resolve({ state: { revision: '1' } }); await settle()
  assert.equal(reader.snapshot.value, null); assert.equal(reader.loading.value, false)
  h.connection.value = 'connected'; const next = h.requests[1]!; scope.stop(); next.resolve({ state: { revision: '2' } }); await settle()
  assert.equal(reader.snapshot.value, null)
})

test('current connections keep each GET state visible and ignore an older material response', async () => {
  const props = vue.reactive({ materialId: 'first', revision: '1' }), requests: Array<{ resolve: (value: any) => void }> = []
  const scope = vue.effectScope(), context = createContext({ ...vue, ApiError, defineProps: () => props, useId: () => 'connections-test', useConnection: () => vue.ref('connected'), apiRequest: () => new Promise(resolve => requests.push({ resolve })) })
  const file = readFileSync(new URL('../app/components/RecipeConnectionsPanel.vue', import.meta.url), 'utf8')
  const source = file.split('<script setup lang="ts">')[1]!.split('</script>')[0]!.replace(/^import .*$/gm, '') + '\nfunction read(){return {value,uses,differentStates,loading}}'
  let state: any
  scope.run(() => { runInContext(ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText, context); state = context.read() })
  try {
    const old = requests.slice(); props.materialId = 'second'
    requests[2]!.resolve({ item: { id: 'second' }, state: { epoch: 'a', revision: '2' } }); requests[3]!.resolve({ items: [], state: { epoch: 'a', revision: '3' } }); await settle()
    assert.equal(state.differentStates.value, true); assert.equal(state.value.value.state.revision, '2'); assert.equal(state.uses.value.state.revision, '3')
    old.forEach(request => request.resolve({ item: { id: 'first' }, items: [], state: { epoch: 'a', revision: '1' } })); await settle(); assert.equal(state.value.value.item.id, 'second')
    props.revision = '2'; requests[4]!.resolve({ item: { id: 'second' }, state: { epoch: 'a', revision: '3' } }); requests[5]!.resolve({ items: [], state: { epoch: 'b', revision: '3' } }); await settle(); assert.equal(state.differentStates.value, true)
  } finally { scope.stop() }
})
