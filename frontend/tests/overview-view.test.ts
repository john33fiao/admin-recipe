import test from 'node:test'
import assert from 'node:assert/strict'
import { existsSync, readFileSync } from 'node:fs'
import { createContext, runInContext } from 'node:vm'
import { parse } from '@vue/compiler-sfc'
import { compile } from '@vue/compiler-dom'
import { renderToString } from '@vue/server-renderer'
import * as vue from 'vue'
import { displayDecimal } from '../app/utils/catalog.ts'
import { noAllocationText } from '../app/utils/overview.ts'

test('overview uses the exact archive-inclusive count instead of a filtered material array', async () => {
  const next = new URL('../app/pages/overview/index.vue', import.meta.url)
  const source = readFileSync(existsSync(next) ? next : new URL('../app/pages/overview.vue', import.meta.url), 'utf8')
  const code = compile(parse(source).descriptor.template!.content, { mode: 'function', prefixIdentifiers: true }).code
  const render = runInContext(`(function () { ${code} })()`, createContext({ Vue: vue }))
  const empty = { total_count: 0, returned_count: 0, offset: 0, page_size: 20, next_offset: null, rows: [] }
  const report = { state: { epoch: 'epoch', revision: '4' }, materials: { ...empty, total_count: 26, returned_count: 25, page_size: 25, next_offset: 25 }, order_counts: { open: 0, received: 0, cancelled: 0 }, open_orders: empty, received_orders: empty, receipts: empty, sales: empty, observations: empty, unallocated: empty, unconfirmed: empty, drafts: empty }
  const stub = vue.defineComponent({ setup: (_, { slots }) => () => vue.h('div', slots.default?.()) })
  const app = vue.createSSRApp({ components: { NuxtLink: stub, OverviewSection: stub }, setup: () => ({
    materials: [{ id: 'filtered-a' }, { id: 'filtered-b' }], loaded: true, loading: false, loadFailure: '', failure: '', available: true,
    snapshot: { report, receivedAt: '2026-01-01T00:00:00Z' }, oldResult: false, connection: 'connected', sections: [],
    formatCreatedAt: () => '조회 시각', loadMaterials: () => {}, load: () => {}, openList: () => {},
  }), render })
  app.config.warnHandler = message => { throw Error(message) }
  const html = await renderToString(app)
  assert.match(html, /전체 품목\(보관 포함\)/)
  assert.match(html, /class="overview-count">26개/)
  assert.doesNotMatch(html, /사용 중인 재료/)
})

test('overview observation renders confirmed zero while an initial basis never becomes zero consumption', async () => {
  const source = readFileSync(new URL('../app/components/OverviewObservation.vue', import.meta.url), 'utf8')
  const code = compile(parse(source).descriptor.template!.content, { mode: 'function', prefixIdentifiers: true }).code
  const render = runInContext(`(function () { ${code} })()`, createContext({ Vue: vue }))
  const value = { quantity: '0', unit: 'g', mode: 'initial', observed_at: '2026-01-01T00:00:00Z', material_id: 'actual-material', material_name: '가상 확인 당시 품목', origin: {}, changes: [], consumption: null } as any
  const stub = vue.defineComponent({ setup: (_, { slots }) => () => vue.h('span', slots.default?.()) })
  async function output() {
    const app = vue.createSSRApp({ components: { NuxtLink: stub, StatisticsReferences: stub }, setup: () => ({ value, detailed: true, zone: 'UTC', displayDecimal, noAllocationText, formatCreatedAt: () => '확인 원시각' }), render })
    app.config.warnHandler = message => { throw Error(message) }
    return renderToString(app)
  }
  const initial = await output(); assert.match(initial, /<strong>0g<\/strong>/); assert.match(initial, /처음 확인한 기준/); assert.doesNotMatch(initial, /실사 소모|0원/)
  value.mode = 'settlement'; value.consumption = { quantity: '0', value: '0', excess_quantity: '0', excess_value: '0', unallocated_quantity: '0', unallocated_value: '0', reason: null, candidate_count: 1 }
  const settled = await output(); assert.match(settled, /실사 소모/); assert.match(settled, /0g · 0원/); assert.match(settled, /판매별 배분 근거 있음/)
})
