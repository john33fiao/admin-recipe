import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createContext, runInContext } from 'node:vm'
import { parse } from '@vue/compiler-sfc'
import { compile } from '@vue/compiler-dom'
import { renderToString } from '@vue/server-renderer'
import * as vue from 'vue'
import ts from 'typescript'
import { cloneCorrection, correctionKinds } from '../app/utils/correction.ts'

const originalVersion = '43d5cc68-e079-42e9-a79b-eed56c8de897'
const tombstoneVersion = 'c6eda4e8-24a0-4eb1-9df3-b9ba835c3566'
function replacement(version: string) {
  return { kind: 'stocktake', data: { rows: [{ observation_id: '36a2d15a-437b-4229-ac5e-f35bf6f0c2ec', expected_version_id: version, action: { kind: 'replace', quantity: '10', unit: 'g', initial_value: { kind: 'manual_amount', amount: '10' }, excess: null } }] } }
}
function editor(makeReplacement: (version: string) => any = replacement) {
  const original = makeReplacement(originalVersion)
  const seed = {
    original: { reference: { kind: 'stocktake', logical_seq: '3', original_recorded_at: '2024-01-01T00:00:00Z', business_record_id: 'original-business' }, input: original, value: { kind: 'stocktake', input: original.data } },
    current: { reference: { version_id: tombstoneVersion }, input: null, value: null },
    replacement_template: makeReplacement(tombstoneVersion),
  }
  const payload = { kind: 'correction', version: 1, data: { expected_target_version_id: tombstoneVersion, reason: '가상 전체 취소 복원', action: { kind: 'void' }, observation_reviews: [], linked_receipt_changes: [] } }
  const props = { identity: 'void-restoration', seed, initial: { payload, corrects: 'original-confirmation', recordId: null }, refresh: async () => null }
  const draft = {
    payload: vue.ref<any>(payload), status: vue.ref('draft'), issues: vue.ref([]), preview: vue.ref(null), busy: vue.ref(false), failure: vue.ref(''), notice: vue.ref(''), storageFailure: vue.ref(''), created: vue.ref(null), pending: vue.ref<any>(null), dirty: vue.ref(false), locked: vue.ref(false), canRetry: vue.ref(false), canReview: vue.ref(false),
    send: async () => undefined, showPreview: async () => undefined, review: async () => false, synchronizeTerminal: () => undefined,
  }
  const source = readFileSync(new URL('../app/components/BusinessCorrectionEditor.vue', import.meta.url), 'utf8')
  const script = parse(source).descriptor.scriptSetup!.content.replace(/^import .*$/gm, '')
  const context = createContext({
    ...vue, __props: props, __draft: draft, defineProps: () => props, useBusinessDraft: () => draft,
    useConnection: () => vue.ref('connected'), cloneCorrection, correctionKinds,
    apiRequest: async () => { throw new Error('unexpected target refresh') }, validateBusinessDraft: () => ({}), formatCreatedAt: (value: string) => value,
  })
  const expose = '\nglobalThis.bindings = { ...__props, ...__draft, target, targetFailure, checking, errors, form, previewPanel, reviewContext, archiveReview, staleTarget, mainReceiptId, mainCandidates, available, action, applyTarget, loadTarget, issue, saveDraft, previewDraft, reviewRequest, correctionKinds, formatCreatedAt };'
  const scope = vue.effectScope()
  scope.run(() => runInContext(ts.transpileModule(script + expose, { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText, context))
  return { seed, draft, scope, source, bindings: (context as any).bindings }
}

test('whole-void correction explicitly restores current-version template without rewriting originals or a pending request', () => {
  const state = editor()
  try {
    const original = JSON.stringify(state.seed.original)
    state.bindings.action({ target: { value: 'replace' } })
    const result = state.draft.payload.value.data.action.replacement
    assert.equal(result.data.rows[0].expected_version_id, tombstoneVersion)
    assert.equal(result.data.rows[0].action.quantity, '10')
    result.data.rows[0].action.initial_value.amount = '12'
    assert.equal(state.seed.replacement_template.data.rows[0]!.action.initial_value.amount, '10')
    assert.equal(JSON.stringify(state.seed.original), original)
    assert.equal(state.seed.current.input, null)
    assert.equal(state.seed.current.value, null)
    state.draft.payload.value.data.action = { kind: 'void' }
    state.draft.pending.value = { request_id: 'same-request', body: cloneCorrection(state.draft.payload.value) }
    const pending = JSON.stringify(state.draft.pending.value)
    state.draft.locked.value = true
    state.bindings.action({ target: { value: 'replace' } })
    assert.equal(state.draft.payload.value.data.action.kind, 'void')
    assert.equal(JSON.stringify(state.draft.pending.value), pending)
  } finally { state.scope.stop() }
})

test('correction row restore receives current template while readonly current remains void and original remains preserved', async () => {
  const state = editor()
  try {
    state.draft.payload.value.data.action = { kind: 'replace', replacement: replacement(tombstoneVersion) }
    let received: any = null
    const displayed: unknown[] = []
    const empty = vue.defineComponent({ render: () => null })
    const template = parse(state.source).descriptor.template!.content
    const code = compile(template, { mode: 'function', prefixIdentifiers: true, expressionPlugins: ['typescript'] }).code
    const render = runInContext(`(function () { ${ts.transpileModule(code, { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText} })()`, createContext({ Vue: vue }))
    const app = vue.createSSRApp({
      components: {
        NuxtLink: empty, CorrectionReviewFields: empty, CorrectionPlanPanel: empty,
        CorrectionReplacementFields: vue.defineComponent({ props: ['original'], setup(props) { received = props.original; return () => null } }),
        CorrectionEventValue: vue.defineComponent({ props: ['value'], setup(props) { displayed.push(props.value); return () => null } }),
      },
      setup: () => state.bindings,
      render,
    })
    app.config.warnHandler = message => { throw new Error(message) }
    await renderToString(app)
    assert.equal(received.data.rows[0].expected_version_id, tombstoneVersion)
    assert.equal(displayed[0], null)
    assert.deepEqual(displayed[1], state.seed.original.value)
  } finally { state.scope.stop() }
})

test('void return restoration uses the current source template and preserves historical source and pending', () => {
  for (const currentSource of ['6a152c30-4268-4d20-bd04-8f3695df8db3', null]) {
    const state = editor(version => ({ kind: 'purchase_return', data: { receipt_id: '7771fc36-40ed-43dd-b560-95a773d0a157', expected_receipt_version_id: version === originalVersion ? originalVersion : currentSource, quantity: '2', unit: 'g', note: '가상 원반품' } }))
    try {
      const original = JSON.stringify(state.seed.original)
      state.bindings.action({ target: { value: 'replace' } })
      assert.equal(state.draft.payload.value.data.action.replacement.data.expected_receipt_version_id, currentSource)
      assert.equal(state.draft.payload.value.data.action.replacement.data.quantity, '2')
      assert.equal(JSON.stringify(state.seed.original), original)
      assert.equal(state.seed.current.value, null)
      state.draft.payload.value.data.action.replacement.data.quantity = '3'
      assert.equal(state.seed.replacement_template.data.quantity, '2')
      const pending = JSON.stringify(state.draft.payload.value)
      state.draft.locked.value = true
      state.bindings.action({ target: { value: 'void' } })
      assert.equal(JSON.stringify(state.draft.payload.value), pending)
    } finally { state.scope.stop() }
  }
})
