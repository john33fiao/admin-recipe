// Run from the repository after `node scripts/build.mjs` and Chromium setup.
// Uses only synthetic input through the real release HTTP API, retaining a fresh
// isolated app/db and evidence under ignored tmp/verification/. No SQL writes.
// The shutdown check uses SIGINT and is currently verified on macOS only.
import assert from 'node:assert/strict'
import { mkdirSync } from 'node:fs'
import { join } from 'node:path'
import { randomUUID } from 'node:crypto'
import { chromium, App, root, sha, save, fileSizeTree } from './verification/app.mjs'

const run = join(root, 'tmp/verification', `t021-integration-${new Date().toISOString().replace(/[:.]/g, '-')}-${randomUUID().slice(0, 8)}`)
mkdirSync(run, { recursive: true })
const browser = await chromium.launch(), app = new App(join(run, 'app'), browser)
let restart
const checks = [], evidence = {}
const today = new Date(), day = [today.getFullYear(), String(today.getMonth() + 1).padStart(2, '0'), String(today.getDate()).padStart(2, '0')].join('-')
const eq = (label, actual, expected) => { assert.deepEqual(actual, expected, label); checks.push(label) }
const withoutState = value => { const { state, ...rest } = value; return rest }
async function holding(item, quantity, value, label) {
  const result = await app.get(`/api/holdings/${item.id}`)
  eq(label + ' quantity', result.settlement_basis.quantity, quantity)
  eq(label + ' value', result.settlement_basis.value, value)
  return result
}
async function correction(confirmation, change) {
  const seed = await app.get(`/api/business/confirmations/${confirmation}/correction-seed`)
  const data = seed.draft; data.reason = '가상 통합 검증 정정'; change(data, seed)
  const saved = await app.post('/api/corrections/drafts', { meta: await app.meta(), payload: { kind: 'correction', version: 1, data }, corrects_confirmation_id: seed.original.reference.original_confirmation_id })
  return app.confirm(saved, { retry: true })
}
try {
  await app.launch()
  const raw = await app.material('가상 통합 원재료'), prepared = await app.material('가상 통합 손질품', 'g', 'prepared')
  await app.price(raw, '2')
  eq('price only keeps inventory absent', (await app.get(`/api/holdings/${raw.id}`)).settlement_basis, null)
  const order = await app.business('purchase_order', { action: 'place', material_id: raw.id, material_version_id: raw.version_id, quantity: '50', unit: 'g', supplier: '가상 공급처', expected_date: '', estimated_amount: '100', note: '' }, { retry: true })
  const orderId = order.result.confirmation.result.order_id
  const orderDetail = await app.get(`/api/orders/${orderId}`)
  eq('order only keeps inventory absent', (await app.get(`/api/holdings/${raw.id}`)).settlement_basis, null)
  const received = await app.business('receipt', { source: { kind: 'order', order_id: orderId, expected_order_version_id: orderDetail.head.id, material_version_id: raw.version_id }, quantity: '50', unit: 'g', amount: '150', receipt_date: day, note: '', stock_effect: { kind: 'unreflected' } }, { retry: true })
  const firstReceipt = received.result.confirmation.result.receipt_id
  await holding(raw, '50', '150', 'order receipt')
  await app.receipt(raw, '50', '50', day, { retry: true })
  await holding(raw, '100', '200', 'mixed direct receipt')
  eq('order closed received', (await app.get(`/api/orders/${orderId}`)).item.status, 'received')
  const production = await app.business('production', { inputs: [{ material_id: raw.id, material_version_id: raw.version_id, quantity: '40', unit: 'g' }], outputs: [{ material_id: prepared.id, material_version_id: prepared.version_id, quantity: '20', unit: 'g', allocation_percent: '100' }], note: '' }, { retry: true })
  await holding(raw, '60', '120', 'production input')
  await holding(prepared, '20', '80', 'production output')
  const menu = await app.recipe('가상 통합 메뉴', [{ material_id: prepared.id, material_version_id: prepared.version_id, quantity: '2', unit: 'g', note: '' }], '2', '10')
  const baseline = await app.stocktake([app.row(prepared, '20')], { retry: true })
  const beforeSales = withoutState(await app.get(`/api/holdings/${prepared.id}`))
  const saleOne = await app.sale(menu, '3', { retry: true }), saleTwo = await app.sale(menu, '1', { retry: true })
  eq('sales preserve all inventory evidence', withoutState(await app.get(`/api/holdings/${prepared.id}`)), beforeSales)
  const saleOneId = saleOne.result.confirmation.result.sale_id, saleTwoId = saleTwo.result.confirmation.result.sale_id
  eq('sale recipe per item cost', (await app.get(`/api/sales/${saleOneId}`)).item.recipe_cost.total_cost, '4')
  const waste = await app.business('waste', { material_id: raw.id, material_version_id: raw.version_id, quantity: '10', unit: 'g', reason: 'discard', note: '' }, { retry: true })
  // Explicit waste lowers physical quantity; its loss remains in held value.
  await holding(raw, '50', '120', 'waste loss retained')
  const ending = await app.stocktake([app.row(prepared, '16')], { retry: true })
  const endingId = ending.result.confirmation.result.stocktake_id
  const beforeCorrection = await app.get(`/api/stocktakes/${endingId}`)
  const calc = beforeCorrection.observations[0].effect.calculation
  eq('settled consumption quantity', calc.consumption_quantity, '4')
  eq('settled consumption cost', calc.consumption_value, '16')
  eq('count based allocation', calc.allocation.candidates.map(row => [row.sale_id, row.quantity, row.value]), [[saleOneId, '3', '12'], [saleTwoId, '1', '4']])
  const preparedBeforeCorrection = withoutState(await holding(prepared, '16', '64', 'settled output'))
  const changed = await correction(saleOne.result.confirmation.id, data => { data.action.replacement.data.count = '1' })
  eq('sale count correction preserves stock', withoutState(await app.get(`/api/holdings/${prepared.id}`)), preparedBeforeCorrection)
  const afterCorrection = await app.get(`/api/stocktakes/${endingId}`)
  eq('original stocktake allocation stays immutable', afterCorrection.observations[0].effect.calculation.allocation, calc.allocation)
  const correctionDetail = await app.get(`/api/corrections/${changed.result.confirmation.result.correction_id}`)
  const changedAllocation = correctionDetail.item.plan.allocations.find(row => row.after?.candidates.some(candidate => candidate.sale_id === saleOneId)).after
  eq('fixed total reallocated equally', changedAllocation.candidates.map(row => [row.quantity, row.value]), [['2', '8'], ['2', '8']])
  for (const saleId of [saleOneId, saleTwoId]) {
    const current = await app.get(`/api/sales/${saleId}`)
    eq('current consumption uses corrected count ' + saleId, current.consumption.map(row => [row.quantity, row.value]), [['2', '8']])
  }
  const source = await app.get(`/api/receipts/${firstReceipt}/return-source`)
  const returned = await app.business('purchase_return', { receipt_id: firstReceipt, expected_receipt_version_id: source.current.reference.version_id, quantity: '5', unit: 'g', note: '' }, { retry: true })
  const returnDetail = await app.get(`/api/purchase-returns/${returned.result.confirmation.result.purchase_return_id}`)
  eq('return uses current average', returnDetail.item.effect.deducted_value, '12')
  eq('return retains original amount', returnDetail.item.effect.source.original_amount, '150')
  await holding(raw, '45', '108', 'returned raw')
  const oldCost = (await app.get(`/api/sales/${saleOneId}`)).item.recipe_cost
  const revised = await app.post(`/api/recipes/${menu.item.id}/versions`, { meta: await app.meta(), expected_material_version_id: menu.item.version_id, expected_recipe_version_id: menu.version.id, data: { ...menu.version, basis_count: '2' } }, 400)
  // Use the same closed data fields as the regular recipe editor.
  const version = await app.post(`/api/recipes/${menu.item.id}/versions`, { meta: await app.meta(), expected_material_version_id: menu.item.version_id, expected_recipe_version_id: menu.version.id, data: { basis_count: '2', reference_output: null, selling_price: '0', cooking_description: '', process: '', reference_time: '', lines: [{ material_id: prepared.id, material_version_id: prepared.version_id, quantity: '4', unit: 'g', note: '' }] } })
  const zeroSale = await app.sale(version, '0', { retry: true })
  const zeroDetail = await app.get(`/api/sales/${zeroSale.result.confirmation.result.sale_id}`)
  eq('zero count accepted', zeroDetail.head.count, '0')
  eq('zero price produces known zero amount', zeroDetail.head.amount, '0')
  eq('new recipe cost applies future sales', zeroDetail.item.recipe_cost.total_cost, '8')
  eq('old sale recipe snapshot retained', (await app.get(`/api/sales/${saleOneId}`)).item.recipe_cost, oldCost)
  await holding(prepared, '16', '64', 'zero sale does not mutate inventory')
  const zero = await app.material('가상 통합 무상 품목')
  const invalidZero = await app.draft('receipt', { source: { kind: 'direct', material_id: zero.id, material_version_id: zero.version_id }, quantity: '0', unit: 'g', amount: '0', receipt_date: day, note: '', stock_effect: { kind: 'unreflected' } })
  eq('zero actual receipt rejected by existing positive quantity contract', (await app.post(`/api/business/${invalidZero.result.item.id}/preview`, { expected_head_id: invalidZero.result.head.id }, 200)).can_confirm, false)
  await app.receipt(zero, '1', '0', day, { retry: true })
  await app.stocktake([app.row(zero, '0')], { retry: true })
  await holding(zero, '0', '0', 'explicit zero counted quantity and free receipt')
  const targets = await app.get('/api/restore-targets'), target = targets.rows[0]
  const selectedViews = await Promise.all([`/api/holdings/${raw.id}`, `/api/holdings/${prepared.id}`, `/api/sales/${saleOneId}`, `/api/stocktakes/${endingId}`, `/api/purchase-returns/${returnDetail.item.id}`].map(path => app.get(path)))
  const transient = await app.material('가상 통합 복원 이후 품목')
  const preview = await app.post('/api/restore/preview', { target_id: target.target_id, expected_sha256: target.sha256 }, 200)
  eq('full restore preview available', preview.can_restore, true)
  const restoreInput = { meta: await app.meta(), target_id: target.target_id, expected_sha256: target.sha256 }
  const restored = await app.post('/api/restores', restoreInput)
  eq('restore request replay identical', await app.post('/api/restores', restoreInput), restored)
  eq('restored new material absent', (await app.get('/api/materials')).some(item => item.id === transient.id), false)
  const restoredViews = await Promise.all([`/api/holdings/${raw.id}`, `/api/holdings/${prepared.id}`, `/api/sales/${saleOneId}`, `/api/stocktakes/${endingId}`, `/api/purchase-returns/${returnDetail.item.id}`].map(path => app.get(path)))
  eq('full restore retains quantities versions allocation corrections return', restoredViews.map(withoutState), selectedViews.map(withoutState))
  const replayCall = app.entries.find(entry => entry.path === `/api/business/${saleOne.result.item.id}/confirm` && entry.status === 201)
  const stateBeforeReplay = await app.get('/api/state')
  const oldRequest = await app.post(replayCall.path, replayCall.input, 409)
  eq('old processed confirmation is identified after restore', oldRequest.request_status, 'already_processed')
  eq('old generation confirmation requires explicit review', oldRequest.code, 'stale_generation')
  eq('old replay makes no new mutation', await app.get('/api/state'), stateBeforeReplay)
  const foreign = await app.page.request.post(new URL('/api/materials', app.launchUrl).toString(), { headers: { Origin: 'https://example.invalid' }, data: { name: '가상 거절', unit: 'g' } })
  eq('cross origin rejected', foreign.status(), 403)
  const unauthenticated = await fetch(new URL('/api/state', app.launchUrl))
  eq('unauthenticated api rejected', unauthenticated.status, 401)
  evidence.final_statistics = await app.get('/api/statistics?' + new URLSearchParams({ from: new Date(today.valueOf() - 86400000).toISOString(), until: new Date(Date.now() + 86400000).toISOString() }))
  const stateBeforeRestart = await app.get('/api/state')
  const shutdown = await app.stop(); app.saveEvidence(join(run, 'api-evidence.json'))
  restart = new App(app.directory, browser, { copy: false }); await restart.launch()
  eq('restart preserves final state', await restart.get('/api/state'), stateBeforeRestart)
  await restart.stop(); restart.saveEvidence(join(run, 'restart-evidence.json'))
  save(join(run, 'result.json'), { status: 'PASS', binary_sha256: sha(app.binary), checks, shutdown, browser: await browser.version(), data: fileSizeTree(join(app.directory, 'db')), evidence })
  console.log(JSON.stringify({ status: 'PASS', run, checks: checks.length, binary_sha256: sha(app.binary) }))
} catch (error) {
  save(join(run, 'failure.json'), { status: 'FAIL', error: String(error), stack: error.stack, checks, evidence }); app.saveEvidence(join(run, 'api-evidence.json')); await app.stop().catch(() => {})
  console.error(JSON.stringify({ status: 'FAIL', run, error: String(error) })); process.exitCode = 1
} finally {
  try { save(join(run, 'cleanup.json'), await Promise.allSettled([app.cleanup(), restart?.cleanup()])) }
  finally { await browser.close() }
}
