import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createContext, runInContext } from 'node:vm'
import ts from 'typescript'
import * as vue from 'vue'
import * as recipe from '../app/utils/recipe.ts'
import * as catalog from '../app/utils/catalog.ts'
import { ApiError } from '../app/utils/api.ts'

const id = '11111111-1111-4111-8111-111111111111', versionId = '22222222-2222-4222-8222-222222222222', epoch = '33333333-3333-4333-8333-333333333333', requestId = '44444444-4444-4444-8444-444444444444'
const item = { id, version_id: versionId, name: '가상 메뉴', unit: '개' }
const data = { ...recipe.blankRecipe(), lines: [{ material_id: id, material_version_id: versionId, quantity: '0', unit: 'g', note: '적당량' }] }
const version = { ...data, id: versionId, material_id: id, material_version_id: versionId, material_name: item.name, material_unit: item.unit, previous_version_id: null, copied_from_version_id: null, recorded_at: '', seq: '1', lines: data.lines.map(line => ({ ...line, id: requestId, position: '0', material_name: '가상 재료', input_quantity: line.quantity, input_unit: line.unit })) }
const initial = { materialId: id, owner: { mode: 'existing', material_id: id, expected_material_version_id: versionId }, data, head: version, state: { epoch, revision: '1' } }
type Request = { path: string; body: any; resolve: (value: unknown) => void; reject: (reason: unknown) => void }
function harness() {
  const states = new Map<string, vue.Ref>(), storage = new Map<string, string>(), requests: Request[] = []
  const context = createContext({ ...vue, ...recipe, ...catalog, ApiError,
    useConnection: () => vue.ref('connected'), useState: (key: string, init: () => unknown) => { if (!states.has(key)) states.set(key, vue.ref(init())); return states.get(key) },
    sessionStorage: { getItem: (key: string) => storage.get(key) ?? null, setItem: (key: string, value: string) => storage.set(key, value), removeItem: (key: string) => storage.delete(key) },
    crypto: { randomUUID: () => requestId }, apiRequest: (path: string, body: unknown) => new Promise((resolve, reject) => requests.push({ path, body, resolve, reject })) })
  const original = readFileSync(new URL('../app/composables/useRecipeEditor.ts', import.meta.url), 'utf8')
  const source = original.replace(/^import .*$/gm, '').replace('export interface RecipeEditorInitial', 'interface RecipeEditorInitial').replace('export function useRecipeEditor', 'function useRecipeEditor')
  runInContext(ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText, context)
  return { create: context.useRecipeEditor, storage, requests }
}
test('recipe nested edits keep pending immutable and a response after navigation updates the shared editor', async () => {
  const h = harness(), firstScope = vue.effectScope()
  const first = firstScope.run(() => h.create(id, initial, async () => null))!
  first.data.value.reference_output = { quantity: '1', unit: '개' }; first.data.value.lines[0].note = '변경'
  assert.equal(first.dirty.value, true); assert.equal(initial.data.lines[0]!.note, '적당량')
  const saving = first.send('revise'), original = JSON.stringify(h.requests[0]!.body)
  firstScope.stop()
  const nextScope = vue.effectScope()
  try {
    const next = nextScope.run(() => h.create(id, initial, async () => null))!
    assert.equal(next.locked.value, true)
    assert.equal(JSON.stringify(next.pending.value.body), original)
    h.requests[0]!.resolve({ item, version: { ...version, reference_output: { quantity: '1', unit: '개', input_quantity: '1', input_unit: '개' }, lines: [{ ...version.lines[0], note: '변경' }] }, state: { epoch, revision: '2' } })
    await saving
    assert.equal(next.pending.value, null); assert.equal(next.data.value.lines[0].note, '변경'); assert.equal(next.dirty.value, false); assert.equal(next.basis.value.revision, '2')
    assert.equal(h.storage.has(`sunchang.recipe.${id}.pending.v1`), false)
  } finally { nextScope.stop() }
})
test('recipe review gates concurrent saves across navigation and preserves user edits and past component versions', async () => {
  const h = harness(), scope = vue.effectScope()
  let release: (value: unknown) => void = () => {}
  const refresh = () => new Promise(resolve => { release = resolve })
  const first = scope.run(() => h.create(id, initial, refresh))!
  first.data.value.lines[0].quantity = '100'
  const save = first.send('revise'); h.requests[0]!.reject(new ApiError(409, 'stale', 'stale_state')); await save
  const reviewing = first.review(); scope.stop()
  const nextScope = vue.effectScope()
  try {
    const next = nextScope.run(() => h.create(id, initial, async () => null))!
    assert.equal(await next.review(), false); assert.equal(await next.send('revise'), null)
    release({ item: { ...item, version_id: requestId }, head: { ...version, id: requestId }, state: { epoch, revision: '9' } }); await reviewing
    assert.equal(next.data.value.lines[0].quantity, '100'); assert.equal(next.data.value.lines[0].material_version_id, versionId)
    const saving = next.send('revise')
    assert.equal(h.requests[1]!.body.expected_recipe_version_id, requestId); assert.equal(h.requests[1]!.body.meta.expected_revision, '9')
    assert.equal(h.storage.has(`sunchang.recipe.${id}.pending.v1`), true)
    h.requests[1]!.reject(new ApiError(503, 'uncertain', 'unavailable')); await saving
  } finally { nextScope.stop() }
})
test('recipe copy retries the original head and body across a reload without sending unsaved data', async () => {
  const h = harness(), scope = vue.effectScope()
  try {
    const first = scope.run(() => h.create(id, initial, async () => null))!
    first.data.value.lines[0].note = '원본에서 작성 중'
    const copying = first.send('copy'), original = JSON.stringify(h.requests[0]!.body)
    assert.equal('data' in h.requests[0]!.body, false)
    h.requests[0]!.reject(new ApiError(503, 'uncertain', 'unavailable')); await copying
    const recovered = harness(); for (const [key, value] of h.storage) recovered.storage.set(key, value)
    const next = scope.run(() => recovered.create(id, { ...initial, state: { epoch, revision: '30' } }, async () => null))!
    const retry = next.send('copy')
    assert.equal(JSON.stringify(recovered.requests[0]!.body), original)
    recovered.requests[0]!.resolve({ item: { ...item, id: requestId, name: '가상 메뉴 (2)' }, version: { ...version, material_id: requestId }, state: { epoch, revision: '31' } }); await retry
    assert.equal(next.data.value.lines[0].note, '원본에서 작성 중'); assert.equal(next.head.value.material_id, id); assert.equal(next.basis.value.revision, '31')
  } finally { scope.stop() }
})
test('413 retains the recipe input until explicit edit recovery and a new creation uses fresh state', async () => {
  const h = harness(), scope = vue.effectScope()
  try {
    const seed = { ...initial, materialId: null, head: null, owner: { mode: 'new', name: '가상 메뉴', unit: '개', kind: 'menu', description: '' } }
    const editor = scope.run(() => h.create('new.item', seed, async () => null))!
    const saving = editor.send('create'); h.requests[0]!.reject(new ApiError(413, 'too large', 'validation')); await saving
    assert.equal(editor.canReview.value, true); assert.equal(editor.canRetry.value, false); assert.ok(editor.pending.value)
    const reviewing = editor.review(); h.requests[1]!.resolve({ epoch, revision: '8' }); await reviewing
    assert.equal(editor.data.value.lines[0].note, '적당량')
    const saveAgain = editor.send('create'); assert.equal(h.requests[2]!.body.meta.expected_revision, '8')
    h.requests[2]!.resolve({ item, version, state: { epoch, revision: '9' } }); await saveAgain
    assert.equal(await editor.send('create'), null); editor.resetNew()
    assert.equal(editor.created.value, false); assert.equal(editor.basis.value.revision, '9'); assert.equal(editor.data.value.lines.length, 0)
  } finally { scope.stop() }
})
