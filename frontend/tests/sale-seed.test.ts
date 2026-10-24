import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createContext, runInContext } from 'node:vm'
import ts from 'typescript'
import * as vue from 'vue'
import * as business from '../app/utils/business.ts'
import { blankSaleDraft } from '../app/utils/sale.ts'
const id = '11111111-1111-4111-8111-111111111111', target = '22222222-2222-4222-8222-222222222222', newer = '33333333-3333-4333-8333-333333333333'
const payload = blankSaleDraft(); Object.assign(payload.data, { material_id: id, material_version_id: id, recipe_version_id: id, count: '0', note: '가상 정정' })
async function seed(pending: ReturnType<typeof business.businessPending> | null = null) {
  const storage = new Map(pending ? [[`sunchang.business.new.sale.${target}.pending.v1`, JSON.stringify(pending)]] : [])
  const detail = { item: { id, confirmation_id: target, effective_confirmation_id: newer }, head: { payload }, confirmations: [{ id: target, result: { kind: 'sale', sale_id: id } }] }
  const context = createContext({ ...vue, ...business, blankSaleDraft, watch: () => {}, useHead: () => {}, useRoute: () => ({ query: { kind: 'sale', corrects: target, record: id } }), useConnection: () => vue.ref('connected'), sessionStorage: { getItem: (key: string) => storage.get(key) ?? null }, apiRequest: async (path: string) => path.startsWith('/api/business/') ? detail : { item: { id, material_name: '가상 메뉴' } } })
  const source = readFileSync(new URL('../app/pages/business/index.vue', import.meta.url), 'utf8').match(/<script setup lang="ts">([\s\S]*?)<\/script>/)![1]!.replace(/^import .*$/gm, '')
  runInContext(ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText, context)
  await context.loadSeed()
  return runInContext('({ payload: seed.value, failure: seedFailure.value })', context)
}
test('an already sent sale correction remains recoverable after another confirmation replaces its target', async () => {
  const pending = business.businessPending('create', null, null, { epoch: id, revision: '9' }, payload, target, () => id)
  const result = await seed(pending)
  assert.equal(result.payload?.kind, 'sale')
  assert.equal(result.payload?.data.count, '0')
})
test('an unsent stale correction target remains blocked', async () => { assert.equal((await seed()).payload, null) })


test('wrong target or kind cannot unlock a stale sale correction screen', async () => {
  for (const pending of [business.businessPending('create', null, null, { epoch: id, revision: '9' }, payload, newer, () => id), business.businessPending('create', null, null, { epoch: id, revision: '9' }, business.blankPriceDraft(), target, () => id)]) assert.equal((await seed(pending)).payload, null)
})
