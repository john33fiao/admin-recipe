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
import { effectivePurchaseReturn } from '../app/utils/purchase-return.ts'

test('return output keeps original unit and purchase amount apart from effective unit and zero deduction', async () => {
  const source = readFileSync(new URL('../app/components/PurchaseReturnResult.vue', import.meta.url), 'utf8')
  const code = compile(parse(source).descriptor.template!.content, { mode: 'function', prefixIdentifiers: true, expressionPlugins: ['typescript'] }).code
  const render = runInContext(`(function () { ${ts.transpileModule(code, { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText} })()`, createContext({ Vue: vue }))
  const value = { material_id: 'virtual-material', material_name: '가상 단위 변경 품목', unit: '개', quantity: '1', input_quantity: '1', input_unit: '개', deducted_value: '0', before: { quantity: '2', value: '0' }, after: { quantity: '1', value: '0' }, source: { reference: { kind: 'proposed', entity: 'receipt', id: 'actual-stable-receipt', expected_version_id: null }, original_quantity: '3', original_amount: '30000', original_unit: 'g', quantity: '2', amount: '0', remaining_quantity: '2', returned_quantity: '0' } }
  const app = vue.createSSRApp({ components: { NuxtLink: vue.defineComponent({ props: ['to'], setup: (props, { slots }) => () => vue.h('a', { href: props.to }, slots.default?.()) }) }, setup: () => ({ value, displayDecimal }), render })
  app.config.warnHandler = message => { throw new Error(message) }
  const html = await renderToString(app)
  assert.match(html, /원구입 참고 수량·금액/); assert.match(html, /3g · 30,000원/)
  assert.match(html, /2개 · 0원/); assert.match(html, /실제 차감액/); assert.match(html, /preview-result">0원/)
  assert.match(html, /href="\/receipts\/actual-stable-receipt"/)
  assert.doesNotMatch(html, /href="[^"]*(null|undefined|proposed)/)
  assert.match(html, /이번 정정에서 바꾸는 입고 근거/)
})

test('void return has no effective amount while a known zero-valued active return remains visible', () => {
  const item = { effect: { quantity: '1', deducted_value: '0' }, note: '' } as any
  assert.equal(effectivePurchaseReturn({ item }), item)
  const active = { kind: 'active', version_id: 'actual-version', effect: item.effect, note: '' } as const
  assert.equal(effectivePurchaseReturn({ item, effective: active })?.effect.deducted_value, '0')
  assert.equal(effectivePurchaseReturn({ item, effective: { kind: 'void', version_id: 'void-version' } }), null)
})
