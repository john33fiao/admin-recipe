import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createContext, runInContext } from 'node:vm'
import { parse } from '@vue/compiler-sfc'
import { compile } from '@vue/compiler-dom'
import { renderToString } from '@vue/server-renderer'
import * as vue from 'vue'
import ts from 'typescript'
import { displayDecimal } from '../app/utils/catalog.ts'
import type { CorrectionPlanRef } from '../app/types/correction.ts'

function componentRender(filename: string) {
  const sourceFile = readFileSync(new URL(`../app/components/${filename}`, import.meta.url), 'utf8')
  const template = parse(sourceFile).descriptor.template!.content
  const generated = compile(template, { mode: 'function', prefixIdentifiers: true, expressionPlugins: ['typescript'] }).code
  return runInContext(`(function () { ${ts.transpileModule(generated, { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText} })()`, createContext({ Vue: vue }))
}

async function manualAllowed(mode: 'initial' | 'excess', quantity: string, source: 'holding_average' | 'missing') {
  let received: unknown = 'not rendered'
  const request = { mode, measured_quantity: quantity, proposed_price: { source }, observation: { kind: 'existing', version_id: 'version' } }
  const empty = vue.defineComponent({ render: () => null })
  const app = vue.createSSRApp({
    components: {
      NuxtLink: empty,
      CorrectionReceiptFields: empty,
      CorrectionValuationFields: vue.defineComponent({ props: ['manualAllowed'], setup(props) { received = props.manualAllowed; return () => null } }),
    },
    setup: () => ({
      data: { observation_reviews: [{ observation_id: 'observation', expected_version_id: 'version', initial_value: null, excess: null }], linked_receipt_changes: [] },
      requests: [], disabled: false, failure: '', loading: null,
      valuationRequest: () => request,
      correctionReferenceVersion: () => 'version',
    }),
    render: componentRender('CorrectionReviewFields.vue'),
  })
  app.config.warnHandler = message => { throw new Error(message) }
  await renderToString(app)
  return received
}

test('initial observation review permits explicit manual value even with a known price while excess keeps its price restriction', async () => {
  assert.equal(await manualAllowed('initial', '2', 'holding_average'), true)
  assert.equal(await manualAllowed('initial', '0', 'holding_average'), true)
  assert.equal(await manualAllowed('initial', '2', 'missing'), true)
  assert.equal(await manualAllowed('excess', '2', 'holding_average'), false)
  assert.equal(await manualAllowed('excess', '2', 'missing'), true)
})

test('correction observation shows exact valuation evidence and its logical position without guessing a reference URL', async () => {
  const sourceId = '8947a8d1-bf5b-4e93-8605-deec91b93bc0'
  const empty = vue.defineComponent({ render: () => null })
  async function show(source: CorrectionPlanRef | null) {
    const app = vue.createSSRApp({
      components: {
        NuxtLink: vue.defineComponent({ props: ['to'], setup: (props, { slots }) => () => vue.h('a', { href: props.to }, slots.default?.()) }),
        CorrectionEventValue: empty, StocktakeResult: empty, ReceiptReconciliationResult: empty,
      },
      setup: () => ({
        displayDecimal,
        plan: { events: [], materials: [], allocations: [], inclusions: [], separate_adjustments: [], order_closures: [], observations: [{
          reference: { kind: 'proposed', id: 'observation' }, stocktake_id: 'stocktake', logical_seq: '20', before: null, after: { unit: 'g' },
          valuation_basis: source ? { amount: '30', quantity: '2', logical_seq: '7', source } : null,
        }] },
      }),
      render: componentRender('CorrectionPlanPanel.vue'),
    })
    app.config.warnHandler = message => { throw new Error(message) }
    return renderToString(app)
  }
  for (const source of [
    { kind: 'proposed', entity: 'price_basis', id: sourceId, expected_version_id: null },
    { kind: 'existing', entity: 'price_basis', id: sourceId, version_id: sourceId },
  ] as CorrectionPlanRef[]) {
    const html = await show(source)
    assert.match(html, /정정 후 가치 근거/)
    assert.match(html, /30원 \/ 2g/)
    assert.match(html, /근거 기록 7/)
    assert.ok(html.includes(sourceId))
    assert.ok(html.includes(source.kind === 'proposed' ? '함께 바뀌는 근거' : '보존된 근거'))
    assert.doesNotMatch(html, new RegExp(`href="[^"]*${sourceId}`))
  }
  assert.doesNotMatch(await show(null), /정정 후 가치 근거/)
})
