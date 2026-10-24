import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createContext, runInContext } from 'node:vm'
import ts from 'typescript'
import * as vue from 'vue'
import * as catalog from '../app/utils/catalog.ts'
import { restoreCatalogDraft } from '../app/utils/draft.ts'

const epoch = '00000000-0000-4000-8000-000000000002'
const originalId = '00000000-0000-4000-8000-000000000001'
const newId = '00000000-0000-4000-8000-000000000003'
const materialId = '00000000-0000-4000-8000-000000000004'
const versionId = '00000000-0000-4000-8000-000000000005'
const fields = { name: '가상 검토 재료', unit: 'g', kind: 'raw', description: '' }
type Request = { body: unknown; resolve: (value: unknown) => void }

// Execute the real composable with Vue scopes and shared Nuxt state. Only I/O is
// deferred so response order and page unmounts are deterministic.
function harness(name: string, initialStates: Record<string, unknown>) {
  const states = new Map(Object.entries(initialStates).map(([key, value]) => [key, vue.ref(value)]))
  const storage = new Map<string, string>()
  const gets: Request[] = []
  const posts: Request[] = []
  const context = createContext({
    ...vue, ...catalog, restoreCatalogDraft,
    useConnection: () => vue.ref('connected'),
    useState: (key: string, init: () => unknown) => { if (!states.has(key)) states.set(key, vue.ref(init())); return states.get(key) },
    sessionStorage: { getItem: (key: string) => storage.get(key) ?? null, setItem: (key: string, value: string) => storage.set(key, value), removeItem: (key: string) => storage.delete(key) },
    crypto: { randomUUID: () => newId }, ApiError: class extends Error {},
    apiRequest: (_url: string, body?: unknown) => new Promise(resolve => { (body === undefined ? gets : posts).push({ body, resolve }) }),
  })
  const original = readFileSync(new URL(`../app/composables/${name}.ts`, import.meta.url), 'utf8')
  const source = original.replace(/^import .*$/gm, '').replace(`export function ${name}`, `function ${name}`)
  runInContext(ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText, context)
  return { create: context[name], storage, gets, posts }
}

test('registration review remains exclusive across navigation and preserves the next pending request', async () => {
  const pending = { ...fields, request_id: originalId, epoch, expected_revision: '0' }
  const h = harness('useMaterialRegistration', { 'pending-material': pending, 'material-recovery': 'refresh' })
  const key = 'sunchang.pending-material.v1'
  h.storage.set(key, JSON.stringify(pending))
  const firstScope = vue.effectScope()
  const firstInstance = firstScope.run(() => h.create())!
  const first = firstInstance.editRejected()
  firstScope.stop()
  const secondScope = vue.effectScope()
  try {
    const secondInstance = secondScope.run(() => h.create())!
    assert.equal(await secondInstance.editRejected(), false)
    assert.equal(h.gets.length, 1)
    assert.equal(await secondInstance.save(), null)
    assert.equal(h.posts.length, 0)
    h.gets[0]!.resolve({ epoch, revision: '1' })
    assert.equal(await first, true)
    const saving = secondInstance.save()
    assert.equal(h.posts.length, 1)
    assert.equal(await firstInstance.editRejected(), false)
    assert.equal(h.gets.length, 1)
    assert.equal(firstInstance.pending.value.request_id, newId)
    assert.equal(JSON.parse(h.storage.get(key)!).request_id, newId)
    h.posts[0]!.resolve({ id: materialId, ...fields, epoch, revision: '2' })
    await saving
    assert.equal(firstInstance.pending.value, null)
    assert.equal(h.storage.has(key), false)
  } finally { secondScope.stop() }
})

test('editor review remains exclusive across navigation and preserves the next pending mutation', async () => {
  const key = `sunchang.material-editor.${materialId}`
  const pending = { action: 'version', body: { ...fields, request_id: originalId, epoch, expected_revision: '0', expected_version_id: versionId } }
  const h = harness('useMaterialEditor', { [`${key}.pending`]: pending, [`${key}.recovery`]: 'refresh' })
  h.storage.set(`${key}.pending.v1`, JSON.stringify(pending))
  const refreshes: Array<(value: unknown) => void> = []
  const refresh = () => new Promise(resolve => refreshes.push(resolve))
  const initial = { id: materialId, ...fields, epoch, revision: '0', version_id: versionId }
  const firstScope = vue.effectScope()
  const firstInstance = firstScope.run(() => h.create(initial, refresh))!
  const first = firstInstance.review()
  firstScope.stop()
  const secondScope = vue.effectScope()
  try {
    const secondInstance = secondScope.run(() => h.create(initial, refresh))!
    assert.equal(await secondInstance.review(), false)
    assert.equal(refreshes.length, 1)
    assert.equal(await secondInstance.send('version'), false)
    assert.equal(h.posts.length, 0)
    refreshes[0]!({ ...initial, revision: '1' })
    assert.equal(await first, true)
    const saving = secondInstance.send('version')
    assert.equal(h.posts.length, 1)
    assert.equal(await firstInstance.review(), false)
    assert.equal(refreshes.length, 1)
    assert.equal(firstInstance.pending.value.body.request_id, newId)
    assert.equal(JSON.parse(h.storage.get(`${key}.pending.v1`)!).body.request_id, newId)
    h.posts[0]!.resolve({ ...initial, revision: '2' })
    assert.equal(await saving, true)
    await vue.nextTick()
    assert.equal(refreshes.length, 2)
    refreshes[1]!({ ...initial, revision: '2' })
    assert.equal(firstInstance.pending.value, null)
    assert.equal(h.storage.has(`${key}.pending.v1`), false)
  } finally { secondScope.stop() }
})
