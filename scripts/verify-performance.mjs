// Run after `node scripts/build.mjs` and Chromium setup, with other checks idle.
// Creates a synthetic release app under ignored tmp/verification/ and measures
// unmodified save/start/read paths at 0/1/10/50/100 sales and, in a separate DB,
// 0/1/10/50/100 materials. Retains every JSON in both synthetic datasets.
// Single-run observations are not production guarantees. Verified on macOS.
import assert from 'node:assert/strict'
import { mkdirSync } from 'node:fs'
import { join } from 'node:path'
import { randomUUID } from 'node:crypto'
import { cpus, arch, platform, release } from 'node:os'
import { chromium, expect, App, root, sha, save, fileSizeTree } from './verification/app.mjs'

const run = join(root, 'tmp/verification', `t021-performance-${new Date().toISOString().replace(/[:.]/g, '-')}-${randomUUID().slice(0, 8)}`)
mkdirSync(run, { recursive: true })
const browser = await chromium.launch()
let app = new App(join(run, 'app'), browser, { transcript: false })
const result = { status: 'RUNNING', environment: { platform: platform(), os_release: release(), arch: arch(), cpu: cpus()[0].model, node: process.version, browser: await browser.version() }, measurements: [], writes: [], catalog_measurements: [], catalog_writes: [], caveat: 'Single sequential run per size on a developer machine. Concurrent external build/test load, if any, must be noted separately. No latency multiplier, release SLA, cold-cache guarantee or production-size claim.' }
const today = new Date(), day = [today.getFullYear(), String(today.getMonth() + 1).padStart(2, '0'), String(today.getDate()).padStart(2, '0')].join('-')
const range = { from: new Date(today.valueOf() - 86400000).toISOString(), until: new Date(today.valueOf() + 86400000).toISOString() }
const statsPath = '/api/statistics?' + new URLSearchParams(range)
let menuId
const catalogRecords = [], catalogKeyword = '가상 성능 짝수'
const summary = values => ({ samples: values.length, min_ms: Math.min(...values), max_ms: Math.max(...values), mean_ms: values.reduce((a, b) => a + b, 0) / values.length })
async function measure(count, settled = false) {
  const beforeStop = await app.get('/api/state')
  const shutdown = await app.stop()
  app.saveEvidence(join(run, `api-${count}-${settled ? 'settled' : 'pending'}.json`))
  const snapshots = fileSizeTree(join(app.directory, 'db/snapshots'))
  const totalData = fileSizeTree(join(app.directory, 'db'))
  app = new App(app.directory, browser, { copy: false, transcript: false }); await app.launch()
  assert.deepEqual(await app.get('/api/state'), beforeStop)
  const paths = ['/api/materials', '/api/materials?query=' + encodeURIComponent('가상 성능'), '/api/sales', '/api/sales?material_id=' + menuId, statsPath]
  const queries = {}
  for (const path of paths) {
    const times = []
    for (let i = 0; i < 3; i++) {
      const start = performance.now(), value = await app.get(path); times.push(performance.now() - start)
      if (path.startsWith('/api/sales')) assert.equal(value.items.length, count)
      if (path === statsPath) assert.equal(value.sales.summary.count, String(count))
    }
    queries[path] = summary(times)
  }
  const renderStart = performance.now()
  await app.page.goto(new URL('/statistics?' + new URLSearchParams({ ...range, zone: 'UTC' }), app.launchUrl).toString())
  await expect(app.page.locator('#statistics-sales-title')).toBeVisible({ timeout: 120000 })
  await expect(app.page.locator('.statistics-report')).toHaveAttribute('aria-busy', 'false')
  await expect(app.page.locator('.statistics-applied details .material-id')).toContainText(range.from)
  await expect(app.page.locator('.statistics-applied details .material-id')).toContainText(range.until)
  await expect(app.page.locator('tr[data-sale-id]')).toHaveCount(count)
  await expect(app.page.locator('.statistics-chart circle')).toHaveCount(count)
  await app.page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))))
  const graphRenderMs = performance.now() - renderStart
  result.measurements.push({ sales: count, settled, mutation_revision: beforeStop.revision, startup_ms: app.startupMs, snapshots, total_data: totalData, queries, statistics_ui_ms: graphRenderMs, shutdown })
  save(join(run, 'result.json'), result)
  console.log(JSON.stringify({ event: 'measured', sales: count, settled, startup_ms: app.startupMs, json_bytes: snapshots.bytes, statistics_ui_ms: graphRenderMs }))
}
async function measureCatalog(count) {
  assert.equal(catalogRecords.length, count)
  const beforeStop = await app.get('/api/state')
  const shutdown = await app.stop()
  app.saveEvidence(join(run, `catalog-api-${count}.json`))
  const snapshots = fileSizeTree(join(app.directory, 'db/snapshots'))
  const totalData = fileSizeTree(join(app.directory, 'db'))
  app = new App(app.directory, browser, { copy: false, transcript: false }); await app.launch()
  assert.deepEqual(await app.get('/api/state'), beforeStop)
  const matches = catalogRecords.filter(row => row.name.includes(catalogKeyword))
  assert.equal(matches.length, Math.floor(count / 2))
  const expected = rows => rows.map(row => [row.id, row.name]).sort((a, b) => a[0].localeCompare(b[0]))
  const paths = ['/api/materials', '/api/materials?query=' + encodeURIComponent(catalogKeyword)]
  const queries = {}
  for (const path of paths) {
    const times = [], rows = path === paths[0] ? catalogRecords : matches
    for (let i = 0; i < 3; i++) {
      const start = performance.now(), value = await app.get(path); times.push(performance.now() - start)
      assert.deepEqual(expected(value), expected(rows), 'catalog API exact name and ID subset')
    }
    queries[path] = { expected_rows: rows.length, ...summary(times) }
  }
  const panel = app.page.locator('.material-panel')
  async function checkRows(rows, searched = false) {
    await expect(panel).toHaveAttribute('aria-busy', 'false')
    await expect(app.page.locator('.count-label strong')).toHaveText(String(rows.length))
    await expect(app.page.getByLabel('이름으로 찾기', { exact: true })).toBeEnabled()
    await expect(app.page.getByRole('button', { name: '검색', exact: true })).toBeEnabled()
    await expect(panel.locator('.material-list > li')).toHaveCount(rows.length)
    await expect(panel.locator('[role="alert"]')).toHaveCount(0)
    if (rows.length === 0) await expect(panel.locator('.empty-state h3')).toHaveText(searched ? '조건에 맞는 재료가 없습니다' : '첫 재료를 기록해 보세요')
    const names = await panel.locator('.material-copy > strong').allTextContents()
    assert.deepEqual(names.map(value => value.trim()).sort(), rows.map(row => row.name).sort(), 'catalog UI exact names')
    const links = await panel.locator('a.material-row').evaluateAll(elements => elements.map(element => element.getAttribute('href')))
    assert.deepEqual(links.sort(), rows.map(row => `/materials/${row.id}`).sort(), 'catalog UI exact material links')
    await app.page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))))
  }
  const renderStart = performance.now()
  await app.page.goto(new URL('/', app.launchUrl).toString())
  await checkRows(catalogRecords)
  const listRenderMs = performance.now() - renderStart
  await app.page.getByLabel('이름으로 찾기', { exact: true }).fill(catalogKeyword)
  const searchStart = performance.now()
  const [response] = await Promise.all([
    app.page.waitForResponse(response => {
      const url = new URL(response.url())
      return response.request().method() === 'GET' && url.pathname === '/api/materials' && url.searchParams.get('query') === catalogKeyword
    }),
    app.page.getByRole('button', { name: '검색', exact: true }).click(),
  ])
  assert.equal(response.status(), 200)
  await response.finished()
  await checkRows(matches, true)
  const searchRenderMs = performance.now() - searchStart
  assert.deepEqual(await app.get('/api/state'), beforeStop, 'catalog reads and UI leave state unchanged')
  result.catalog_measurements.push({ materials: count, matched_materials: matches.length, mutation_revision: beforeStop.revision, startup_ms: app.startupMs, snapshots, total_data: totalData, queries, list_ui_ms: listRenderMs, search_ui_ms: searchRenderMs, shutdown })
  save(join(run, 'result.json'), result)
  console.log(JSON.stringify({ event: 'catalog-measured', materials: count, matched_materials: matches.length, startup_ms: app.startupMs, json_bytes: snapshots.bytes, list_ui_ms: listRenderMs, search_ui_ms: searchRenderMs }))
}
try {
  await app.launch()
  result.binary_sha256 = sha(app.binary)
  const item = await app.material('가상 성능 원재료')
  await app.receipt(item, '1000', '1000', day)
  await app.stocktake([app.row(item, '1000')])
  const menu = await app.recipe('가상 성능 메뉴', [{ material_id: item.id, material_version_id: item.version_id, quantity: '1', unit: 'g', note: '' }])
  menuId = menu.item.id
  result.dataset = { material_count: 2, description: 'One raw material and one menu fixed; cumulative sales and full-state JSON grow with n. Catalog keyword search stays a two-item lookup; growing-record filter cost is measured separately.' }
  await measure(0)
  for (let n = 1; n <= 100; n++) {
    const callStart = app.calls.length, start = performance.now()
    await app.sale(menu, '1')
    const calls = app.calls.slice(callStart)
    result.writes.push({ sale: n, total_ms: performance.now() - start, draft_ms: calls.find(call => call.method === 'POST' && call.path.endsWith('/drafts'))?.ms, confirm_ms: calls.find(call => call.path.endsWith('/confirm'))?.ms, preview_ms: calls.find(call => call.path.endsWith('/preview'))?.ms })
    if ([1, 10, 50, 100].includes(n)) await measure(n)
    else if (n % 10 === 0) { save(join(run, 'result.json'), result); console.log(JSON.stringify({ event: 'written', sales: n, last_total_ms: result.writes.at(-1).total_ms })) }
  }
  const start = performance.now()
  await app.stocktake([app.row(item, '900')])
  result.settlement_ms = performance.now() - start
  const current = await app.get(`/api/holdings/${item.id}`)
  assert.equal(current.settlement_basis.quantity, '900')
  assert.equal(current.settlement_basis.value, '900')
  const report = await app.get(statsPath)
  assert.equal(report.observations.summary_by_material[0].consumption_quantity, '100')
  assert.equal(report.observations.summary_by_material[0].allocated_quantity, '100')
  await measure(100, true)
  await app.stop(); app.saveEvidence(join(run, 'api-final.json'))
  app = new App(join(run, 'catalog'), browser, { transcript: false }); await app.launch()
  assert.equal(sha(app.binary), result.binary_sha256, 'both datasets use the same release candidate')
  result.catalog_dataset = { description: 'Separate fresh DB, 0/1/10/50/100 raw materials created through the normal API. Even-numbered names contain the search keyword; expected subset is floor(n/2). Every JSON and DB is retained.', keyword: catalogKeyword }
  await measureCatalog(0)
  for (let n = 1; n <= 100; n++) {
    const start = performance.now()
    const item = await app.material(`가상 성능 ${n % 2 === 0 ? '짝수' : '홀수'} 품목 ${String(n).padStart(3, '0')}`)
    result.catalog_writes.push({ material: n, total_ms: performance.now() - start, save_ms: app.calls.at(-1).ms })
    catalogRecords.push({ id: item.id, name: item.name })
    if ([1, 10, 50, 100].includes(n)) await measureCatalog(n)
    else if (n % 10 === 0) { save(join(run, 'result.json'), result); console.log(JSON.stringify({ event: 'catalog-written', materials: n, last_total_ms: result.catalog_writes.at(-1).total_ms })) }
  }
  await app.stop(); app.saveEvidence(join(run, 'catalog-api-final.json'))
  result.status = 'PASS'; save(join(run, 'result.json'), result)
  console.log(JSON.stringify({ status: 'PASS', run, binary_sha256: result.binary_sha256 }))
} catch (error) {
  result.status = 'FAIL'; result.error = String(error); result.stack = error.stack
  save(join(run, 'result.json'), result); app.saveEvidence(join(run, 'api-failure.json')); await app.stop().catch(() => {})
  console.error(JSON.stringify({ status: 'FAIL', run, error: String(error) })); process.exitCode = 1
} finally {
  try { save(join(run, 'cleanup.json'), await Promise.allSettled([app.cleanup()])) }
  finally { await browser.close() }
}
