import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createContext, runInContext } from 'node:vm'
import ts from 'typescript'
import * as vue from 'vue'
import * as helpers from '../app/utils/profile.ts'
import { ApiError } from '../app/utils/api.ts'
import { failureAction } from '../app/utils/catalog.ts'
const id = '11111111-1111-4111-8111-111111111111', version = '22222222-2222-4222-8222-222222222222'
const png = Uint8Array.from([137, 80, 78, 71, 13, 10, 26, 10, 1])
const file = (last = 1) => new File([Uint8Array.from([...png, last])], 'virtual.png', { type: 'image/png' })
const settle = () => new Promise(resolve => setImmediate(resolve))
type Request = { path: string; body?: FormData; resolve: (value: any) => void; reject: (value: unknown) => void }
function detail(revision = '1', saved = false) {
  return { material: { id, name: '가상 사진 품목', version_id: id }, state: { epoch: id, revision }, version: saved ? { id: version, material_id: id, material_version_id: id, previous_version_id: null, copied_from_version_id: null, recorded_at: '2026-01-01T00:00:00Z', seq: revision, suppliers: [], photos: [{ id: version, position: 0, caption: '새 사진', is_primary: true, asset: { sha256: 'a'.repeat(64), asset_key: 'assets/photos/virtual.png', format: 'png', byte_length: 10, width: 1, height: 1 } }] } : null }
}
function harness(storage = new Map<string, string>(), descriptor = helpers.describeProfileFile) {
  const states = new Map<string, vue.Ref>(), requests: Request[] = []; let failStorage = false
  const context = createContext({ ...vue, ...helpers, describeProfileFile: descriptor, ApiError, failureAction, FormData, File, TextEncoder, crypto: globalThis.crypto,
    useConnection: () => vue.ref('connected'), useState: (key: string, init: () => unknown) => { if (!states.has(key)) states.set(key, vue.ref(init())); return states.get(key) },
    sessionStorage: { getItem: (key: string) => storage.get(key) ?? null, setItem: (key: string, value: string) => { if (failStorage) throw Error('full'); storage.set(key, value) }, removeItem: (key: string) => storage.delete(key) },
    apiRequest: (path: string, body?: FormData) => new Promise((resolve, reject) => requests.push({ path, body, resolve, reject })),
  })
  const source = readFileSync(new URL('../app/composables/useMaterialProfile.ts', import.meta.url), 'utf8').replace(/^import .*$/gm, '').replace(/export function /g, 'function ')
  runInContext(ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText, context)
  return { context, requests, storage, failStorage: () => { failStorage = true } }
}
async function open(h: ReturnType<typeof harness>, scope: vue.EffectScope, value = detail()) {
  const editor = scope.run(() => h.context.useMaterialProfile(id))!
  h.requests.at(-1)!.resolve(value); await settle(); return editor
}
test('profile uncertain upload restores its original metadata and requires matching bytes before retry', async () => {
  const h = harness(), scope = vue.effectScope()
  const editor = await open(h, scope)
  await editor.chooseFile(file()); editor.input.value.photos[0].caption = '원 입력'
  const sending = editor.send(), original = h.requests.at(-1)!.body!.get('metadata')
  h.requests.at(-1)!.reject(new ApiError(503, 'unknown', 'unavailable')); await sending; scope.stop()
  const next = harness(h.storage), nextScope = vue.effectScope()
  try {
    const restored = await open(next, nextScope, detail('2', true))
    assert.equal(restored.needsFile.value, true)
    assert.equal(await restored.send(), false)
    assert.equal(await restored.chooseFile(file(2)), false)
    assert.equal(next.requests.filter(request => request.body).length, 0)
    assert.equal(await restored.chooseFile(file()), true)
    const retry = restored.send(), request = next.requests.at(-1)!
    assert.equal(request.body!.get('metadata'), original)
    request.resolve(detail('2', true)); await retry
    assert.equal(restored.pending.value, null)
    assert.equal(next.storage.has(`sunchang.profile.${id}.pending.v1`), false)
  } finally { nextScope.stop() }
})
test('a late upload success synchronizes a remounted editor and an older GET cannot restore old input', async () => {
  const h = harness(), old = vue.effectScope(), live = vue.effectScope()
  try {
    const editor = await open(h, old); await editor.chooseFile(file())
    const sending = editor.send(), write = h.requests.at(-1)!
    old.stop()
    const current = live.run(() => h.context.useMaterialProfile(id))!, delayed = h.requests.at(-1)!
    write.resolve(detail('2', true)); await sending; await settle()
    delayed.resolve(detail()); await settle()
    assert.equal(current.pending.value, null); assert.equal(current.file.value, null)
    assert.equal(current.input.value.photos[0].source, 'existing')
    assert.equal(current.basis.value.state.revision, '2')
    assert.equal(current.current.value.state.revision, '2')
    assert.equal(current.dirty.value, false)
  } finally { old.stop(); live.stop() }
})
test('profile storage failure prevents sending and concurrent explicit review cannot discard a newer request', async () => {
  const h = harness(), scope = vue.effectScope()
  try {
    const editor = await open(h, scope)
    editor.input.value.suppliers.push({ url: null, description: '가상 설명' })
    const sending = editor.send(); h.requests.at(-1)!.reject(new ApiError(409, 'stale', 'stale_state')); await sending
    const review = editor.review(), read = h.requests.at(-1)!
    assert.equal(await editor.review(), false); assert.equal(await editor.send(), false)
    read.resolve(detail('2')); await review
    assert.equal(editor.input.value.suppliers[0].description, '가상 설명')
    h.failStorage(); const before = h.requests.length
    assert.equal(await editor.send(), false); assert.equal(h.requests.length, before)
    assert.match(editor.storageFailure.value, /전송하지 않았습니다/)
  } finally { scope.stop() }
})
test('late file hashing cannot replace a more recently selected photo', async () => {
  const waits: { resolve: (value: any) => void }[] = []
  const h = harness(new Map(), () => new Promise(resolve => waits.push({ resolve }))), scope = vue.effectScope()
  try {
    const editor = await open(h, scope), a = file(), b = file(2)
    const first = editor.chooseFile(a), second = editor.chooseFile(b)
    const da = await helpers.describeProfileFile(a), db = await helpers.describeProfileFile(b)
    waits[1]!.resolve(db); assert.equal(await second, true)
    waits[0]!.resolve(da); assert.equal(await first, false)
    assert.equal(editor.file.value, b); assert.equal(editor.fileDescriptor.value.sha256, db.sha256)
    assert.equal(editor.input.value.photos.length, 1)
  } finally { scope.stop() }
})

test('a restored clean profile uses the latest state before the next edit', async () => {
  const saved = detail('3', true), input = helpers.profileFromVersion(saved.version as any)
  const storage = new Map([[`sunchang.profile.${id}.draft.v1`, JSON.stringify({ input, basis: { state: saved.state, material_version_id: id, profile_version_id: version }, file: null })]])
  const h = harness(storage), scope = vue.effectScope()
  try {
    const editor = await open(h, scope, detail('4', true))
    assert.equal(editor.dirty.value, false)
    editor.input.value.photos[0].caption = '교체 설명'
    const sending = editor.send(), request = h.requests.at(-1)!
    const body = JSON.parse(String(request.body!.get('metadata')))
    request.resolve(detail('5', true)); await sending
    assert.equal(body.meta.expected_revision, '4')
  } finally { scope.stop() }
})

test('restored edits and uncertain requests keep their original basis after a newer GET', async () => {
  for (const uncertain of [false, true]) {
    const saved = detail('3', true), input = helpers.profileFromVersion(saved.version as any)
    if (!uncertain) input.photos[0]!.caption = '보관한 미저장 설명'
    const body = { ...input, meta: { request_id: version, epoch: id, expected_revision: '3' }, expected_material_version_id: id, expected_profile_version_id: version }
    const key = `sunchang.profile.${id}.${uncertain ? 'pending' : 'draft'}.v1`
    const value = uncertain ? { material_id: id, body, file: null } : { input, basis: { state: saved.state, material_version_id: id, profile_version_id: version }, file: null }
    const h = harness(new Map([[key, JSON.stringify(value)]])), scope = vue.effectScope()
    try {
      const editor = await open(h, scope, detail('4', true))
      assert.equal(editor.basis.value.state.revision, '3')
      assert.equal(editor.input.value.photos[0].caption, input.photos[0]!.caption)
      assert.equal(editor.dirty.value, !uncertain)
      const sending = editor.send(), request = h.requests.at(-1)!
      const sent = JSON.parse(String(request.body!.get('metadata')))
      request.reject(new ApiError(409, 'stale', 'stale_state')); await sending
      assert.equal(sent.meta.expected_revision, '3')
      if (uncertain) assert.deepEqual(sent, body)
    } finally { scope.stop() }
  }
})
