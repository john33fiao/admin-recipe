import { test, expect, type Page } from '@playwright/test'
import { spawn, type ChildProcess } from 'node:child_process'
import { once } from 'node:events'
import { randomUUID } from 'node:crypto'
import { cpSync, mkdtempSync, rmSync, readdirSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

const binaryName = process.platform === 'win32' ? 'sunchang-hansang.exe' : 'sunchang-hansang'
let directory: string, binary: string, server: ChildProcess, closed: Promise<[number | null, NodeJS.Signals | null]>, launchUrl: string
async function launch() {
  server = spawn(binary, ['--no-browser'], { cwd: directory, stdio: ['ignore', 'pipe', 'pipe'] }); closed = once(server, 'close') as typeof closed
  launchUrl = await new Promise<string>((resolveUrl, reject) => {
    let output = '', errors = ''; const timer = setTimeout(() => reject(Error(`statistics fixture start timed out: ${errors}`)), 15_000)
    server.stderr!.on('data', data => { errors += String(data) }); server.once('exit', code => { clearTimeout(timer); reject(Error(`statistics fixture exited ${code}: ${errors}`)) })
    server.stdout!.on('data', data => { output += String(data); const match = output.match(/http:\/\/127\.0\.0\.1:\d+\/#token=[a-f0-9]+/); if (match) { clearTimeout(timer); resolveUrl(match[0]) } })
  })
}
test.beforeEach(async () => { directory = mkdtempSync(join(tmpdir(), 'sunchang-statistics-')); binary = join(directory, binaryName); cpSync(resolve('..', 'target', 'release', binaryName), binary); await launch() })
test.afterEach(async () => { if (server && server.exitCode === null && server.signalCode === null) server.kill(); if (closed) await closed; if (directory) { expect(resolve(directory).startsWith(resolve(tmpdir()))).toBe(true); rmSync(directory, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 }) } })
test.setTimeout(120_000)
async function open(page: Page) { await page.goto(launchUrl); await expect(page.getByLabel('재료 이름', { exact: false })).toBeEnabled() }
async function visit(page: Page, path: string) { await page.goto(new URL(path, launchUrl).toString()) }
async function get(page: Page, path: string) { const response = await page.request.get(new URL(path, launchUrl).toString()); expect(response.ok(), await response.text()).toBe(true); return response.json() }
async function post(page: Page, path: string, data: unknown) { const response = await page.request.post(new URL(path, launchUrl).toString(), { headers: { Origin: new URL(launchUrl).origin }, data }); expect(response.status(), await response.text()).toBe(201); return response.json() }
async function meta(page: Page) { const state = await get(page, '/api/state'); return { request_id: randomUUID(), epoch: state.epoch, expected_revision: state.revision } }
async function material(page: Page, name: string) { return post(page, '/api/materials', { ...await meta(page), name, unit: 'g', kind: 'raw', description: '' }) }
async function confirm(page: Page, kind: string, data: unknown, corrects: string | null = null) {
  const alias = kind === 'correction' ? '/api/corrections/drafts' : kind === 'stocktake' ? '/api/stocktake-drafts' : kind === 'production' ? '/api/production-drafts' : '/api/business/drafts'
  const saved = await post(page, alias, { meta: await meta(page), payload: { kind, version: 1, data }, corrects_confirmation_id: corrects })
  return (await post(page, `/api/business/${saved.result.item.id}/confirm`, { meta: await meta(page), expected_head_id: saved.result.head.id })).result.confirmation
}
async function observe(page: Page, item: any, quantity: string, amount?: string) {
  const saved = await confirm(page, 'stocktake', { rows: [{ material_id: item.id, material_version_id: item.version_id, quantity, unit: item.unit, initial_value: amount === undefined ? null : { kind: 'manual_amount', amount }, excess: null }], note: '' })
  return get(page, `/api/stocktakes/${saved.result.stocktake_id}`)
}
async function receipt(page: Page, item: any, quantity: string, amount: string, stock_effect: unknown = { kind: 'unreflected' }) { return confirm(page, 'receipt', { source: { kind: 'direct', material_id: item.id, material_version_id: item.version_id }, quantity, unit: item.unit, amount, receipt_date: '2024-02-29', note: '', stock_effect }) }
async function recipe(page: Page, name: string, lines: Array<{ item: any; quantity: string }>, price: string | null = '50', basis = '1', owner?: any) {
  return post(page, '/api/recipes', { meta: await meta(page), owner: owner ? { mode: 'existing', material_id: owner.id, expected_material_version_id: owner.version_id } : { mode: 'new', name, unit: '개', kind: 'menu', description: '' }, data: { basis_count: basis, reference_output: null, selling_price: price, cooking_description: '', process: '', reference_time: '', lines: lines.map(({ item, quantity }) => ({ material_id: item.id, material_version_id: item.version_id, quantity, unit: item.unit, note: '' })) } })
}
async function sale(page: Page, menu: any, count: string) { return confirm(page, 'sale', { material_id: menu.item.id, material_version_id: menu.item.version_id, recipe_version_id: menu.version.id, count, note: '' }) }
async function correction(page: Page, confirmation: string, change: (draft: any, seed: any) => void) { const seed = await get(page, `/api/business/confirmations/${confirmation}/correction-seed`), data = seed.draft; data.reason = '가상 통계 정정'; change(data, seed); return confirm(page, 'correction', data, seed.original.reference.original_confirmation_id) }
const broad = { from: '2020-01-01T00:00:00Z', until: '2100-01-01T00:00:00Z', zone: 'UTC' }
function reportPath(range = broad, api = false) { return `${api ? '/api' : ''}/statistics?${new URLSearchParams(api ? { from: range.from, until: range.until } : range)}` }
async function show(page: Page, range = broad) { await visit(page, reportPath(range)); await expect(page.getByRole('heading', { name: '적용된 조회 기간', exact: true })).toBeVisible(); return get(page, reportPath(range, true)) }
async function archive(page: Page, id: string) { const item = await get(page, `/api/materials/${id}`); return post(page, `/api/materials/${id}/archive`, { ...await meta(page), expected_version_id: item.version_id }) }
async function mixed(page: Page) {
  const raw = await material(page, '가상 원물 F — 기간과 정정의 근거를 함께 확인하는 긴 이름'), stock = await material(page, '가상 비축품 S')
  const received = await receipt(page, raw, '10', '1000'); await observe(page, raw, '10')
  await confirm(page, 'production', { inputs: [{ material_id: raw.id, material_version_id: raw.version_id, quantity: '2', unit: 'g' }], outputs: [{ material_id: stock.id, material_version_id: stock.version_id, quantity: '1', unit: 'g', allocation_percent: '100' }], note: '' })
  const source = await get(page, `/api/receipts/${received.result.receipt_id}/return-source`)
  await confirm(page, 'purchase_return', { receipt_id: source.original.id, expected_receipt_version_id: source.current.reference.version_id, quantity: '2', unit: 'g', note: '' })
  await confirm(page, 'waste', { material_id: raw.id, material_version_id: raw.version_id, quantity: '1', unit: 'g', reason: 'discard', note: '' })
  await recipe(page, '가상 비축 배합', [{ item: raw, quantity: '2' }], null, '1', stock)
  const menu = await recipe(page, '가상 두 개 기준 메뉴', [{ item: raw, quantity: '1' }, { item: stock, quantity: '0.5' }], '50', '2')
  const first = await sale(page, menu, '3'), second = await sale(page, menu, '1'), ending = await observe(page, raw, '4'); await observe(page, stock, '1')
  return { raw, stock, menu, first, second, ending, received }
}

test('mixed normal records keep exact sales, consumption and separate operation values before and after correction', async ({ page }) => {
  await open(page); const data = await mixed(page), state = await get(page, '/api/state'), files = readdirSync(join(directory, 'db', 'snapshots'))
  let report = await show(page); expect(report.state).toEqual(state); expect(await get(page, '/api/state')).toEqual(state); expect(readdirSync(join(directory, 'db', 'snapshots'))).toEqual(files)
  expect(report.sales.summary.count).toBe('4'); expect(report.sales.summary.known_revenue).toBe('200')
  let total = report.observations.summary_by_material.find((row: any) => row.material_id === data.raw.id)
  expect([total.consumption_quantity, total.consumption_value, total.allocated_quantity, total.unallocated_quantity]).toEqual(['1', '120', '1', '0'])
  expect(report.sales.rows[0].original.recipe_cost.total_cost).toBe('110')
  expect(report.sales.rows.map((row: any) => row.consumption.find((entry: any) => entry.material_id === data.raw.id).value)).toEqual(['90', '30'])
  expect(report.operations.rows.find((row: any) => row.origin.kind === 'purchase_return').value.result.deducted_value).toBe('200')
  expect(report.operations.rows.find((row: any) => row.origin.kind === 'production').value.result.total_input_value).toBe('200')
  expect(report.operations.rows.find((row: any) => row.origin.kind === 'waste').value.result.loss_value).toBe('0')
  await correction(page, data.first.id, draft => { draft.action.replacement.data.count = '1' }); report = await show(page)
  expect(report.sales.summary.count).toBe('2'); expect(report.sales.summary.known_revenue).toBe('100'); total = report.observations.summary_by_material.find((row: any) => row.material_id === data.raw.id)
  expect([total.consumption_quantity, total.consumption_value]).toEqual(['1', '120'])
  expect(report.sales.rows.map((row: any) => row.consumption.find((entry: any) => entry.material_id === data.raw.id).value)).toEqual(['60', '60'])
  await page.getByRole('button', { name: data.menu.item.name, exact: true }).first().click(); await expect(page.locator('#statistics-sale-detail')).toContainText('0.5g · 60원'); await page.getByText('당시 레시피 기준 원가 · 1개 기준', { exact: true }).click(); await expect(page.locator('.sale-cost-snapshot')).toContainText('110원')
  await page.getByLabel('이력 종류').selectOption('receipt'); await page.locator('.statistics-records summary').filter({ hasText: /^입고/ }).click(); await expect(page.getByRole('heading', { name: '원입고', exact: true })).toBeVisible(); await expect(page.locator('.statistics-report')).toContainText('입고일 2024-02-29')
})

test('initial, zero weight, missing revenue, void and empty observed rows never become invented zeros', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 시작과 미배분'), initial = await observe(page, raw, '10', '100'), menu = await recipe(page, '가상 가격 없는 0개', [{ item: raw, quantity: '0' }], null)
  const first = await sale(page, menu, '0'); await observe(page, raw, '8')
  const other = await material(page, '가상 명시 확인한 0 수량'), beforeUnchecked = await get(page, `/api/holdings/${raw.id}`)
  const partial = await confirm(page, 'stocktake', { rows: [{ material_id: other.id, material_version_id: other.version_id, quantity: '0', unit: 'g', initial_value: { kind: 'manual_amount', amount: '0' }, excess: null }, { material_id: raw.id, material_version_id: raw.version_id, quantity: '', unit: 'g', initial_value: null, excess: null }], note: '가상 확인한 행과 미확인 행' })
  const partialDetail = await get(page, `/api/stocktakes/${partial.result.stocktake_id}`); expect(partialDetail.observations).toHaveLength(1); expect(partialDetail.observations[0].effect.material_id).toBe(other.id); expect((await get(page, `/api/holdings/${raw.id}`)).observation).toEqual(beforeUnchecked.observation)
  let report = await show(page); expect(report.sales.summary.missing_revenue_records).toBe(1); expect(report.sales.rows[0].value).toEqual({ count: '0', amount: null }); expect(report.sales.rows[0].consumption[0].quantity).toBeNull()
  expect(report.observations.rows.find((row: any) => row.observation_id === initial.observations[0].id).status).toBe('initial')
  const totals = report.observations.summary_by_material.find((row: any) => row.material_id === raw.id); expect(totals.unallocated_quantity).toBe('2'); expect(totals.unallocated_value).toBe('20'); expect(report.observations.rows.filter((row: any) => row.material_id === raw.id)).toHaveLength(2)
  await expect(page.locator('.statistics-chart circle')).toHaveCount(0); await expect(page.locator('#statistics-sale-detail')).toContainText('총개수 0 · 미배분')
  await page.getByLabel('시계열 값').selectOption('count'); await expect(page.locator('.statistics-chart circle')).toHaveCount(1); await expect(page.locator('.chart-selection')).toContainText('0개')
  await correction(page, first.id, draft => { draft.action = { kind: 'void' } }); report = await show(page); expect(report.sales.summary.void_records).toBe(1); expect(report.sales.rows[0].value).toBeNull(); await expect(page.locator('.statistics-table').first()).toContainText('취소됨')
  await correction(page, first.id, (draft, seed) => { draft.action = { kind: 'replace', replacement: seed.replacement_template } }); report = await show(page); expect(report.sales.rows[0].value.count).toBe('0'); expect(report.sales.summary.missing_revenue_records).toBe(1)
})

test('exact original boundaries keep outside-period allocations and outside-period sales candidates', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 기간 밖 근거'), menu = await recipe(page, '가상 기간별 메뉴', [{ item: raw, quantity: '0' }]); await observe(page, raw, '10', '100'); const selling = await sale(page, menu, '2'), original = await get(page, `/api/sales/${selling.result.sale_id}`), ending = await observe(page, raw, '8')
  const onlySale = { from: original.item.recorded_at, until: ending.item.recorded_at, zone: 'UTC' }; const report = await show(page, onlySale)
  expect(report.sales.rows).toHaveLength(1); expect(report.observations.rows).toHaveLength(0); expect(report.sales.rows[0].consumption[0].observation_in_period).toBe(false)
  await expect(page.locator('#statistics-sale-detail')).toContainText('조회 기간 밖 실사'); await expect(page.locator('.statistics-report')).toContainText('다른 기간에')
  const onlyStock = { from: ending.item.recorded_at, until: broad.until, zone: 'UTC' }, reverse = await show(page, onlyStock)
  expect(reverse.sales.rows).toHaveLength(0); expect(reverse.observations.rows[0].candidate_links[0].sale_in_period).toBe(false)
  await page.locator(`[data-observation-id="${ending.observations[0].id}"] summary`).click(); await expect(page.locator('.statistics-report')).toContainText('조회 기간 밖 판매')
  const encoded = new URLSearchParams({ from: onlySale.from.replace('Z', '+00:00'), until: onlySale.until.replace('Z', '+00:00') }); expect((await get(page, `/api/statistics?${encoded}`)).sales.rows).toEqual(report.sales.rows)
})

test('current recipe connections preserve owner basis, zero rows, multiple paths, archived references and stable links', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 연결 원물'), left = await material(page, '가상 왼쪽 비축'), right = await material(page, '가상 오른쪽 비축')
  await recipe(page, '왼쪽', [{ item: raw, quantity: '4' }], null, '2', left); await recipe(page, '오른쪽', [{ item: raw, quantity: '9' }], null, '3', right)
  const menu = await recipe(page, '가상 여러 경로 메뉴', [{ item: raw, quantity: '0' }, { item: left, quantity: '2' }, { item: right, quantity: '3' }], '50', '30'); await archive(page, left.id)
  await show(page); await page.getByLabel('연결을 볼 품목').selectOption(menu.item.id); await expect(page).toHaveURL(new RegExp(`material=${menu.item.id}`)); await expect(page.locator('.recipe-connections')).toContainText('직접 구성 3행'); await expect(page.locator('.recipe-connections')).toContainText('간접 연결 2경로'); await expect(page.locator('.recipe-connections')).toContainText('30개 기준 0g')
  await page.locator('.recipe-connections .statistics-records summary').first().click(); await expect(page.locator('.recipe-connections')).toContainText('2개 기준 구성 원입력 4g')
  const graph = await get(page, `/api/recipes/${menu.item.id}/connections`); expect(graph.paths[0].steps.map((step: any) => step.basis_count)).toEqual(['30', '2']); expect(graph.direct[0].line.quantity).toBe('0')
  await page.reload(); await expect(page.getByLabel('연결을 볼 품목')).toHaveValue(menu.item.id); await page.getByLabel('연결을 볼 품목').selectOption(raw.id); await expect(page.locator('.recipe-connections')).toContainText('현재 레시피가 없습니다'); await expect(page.locator('.recipe-connections')).toContainText('가상 여러 경로 메뉴')
  await visit(page, `/recipes/${menu.item.id}`); await expect(page.getByRole('heading', { name: '현재 품목 연결', exact: true })).toBeVisible()
})

test('today default, failed new periods and browser navigation preserve the applied range and existing draft', async ({ page }) => {
  await open(page); await page.getByLabel('재료 이름', { exact: false }).fill('가상 저장 전 이름'); await page.getByRole('link', { name: '통계·이력', exact: true }).click(); await expect(page.getByRole('heading', { name: '적용된 조회 기간', exact: true })).toBeVisible()
  const savedUrl = page.url(), query = new URL(savedUrl).searchParams; expect(query.get('from')).toBeTruthy(); expect(query.get('until')).toBeTruthy(); expect(query.get('zone')).toBeTruthy()
  const before = await get(page, '/api/state'); await page.route('**/api/statistics?*', route => route.fulfill({ status: 400, contentType: 'application/json', body: JSON.stringify({ code: 'report_too_large', error: '가상 상한 거절' }) }))
  await page.getByLabel('시작 날짜', { exact: true }).fill('2020-01-01'); await page.getByRole('button', { name: '기간 조회', exact: true }).click(); await expect(page.getByRole('alert')).toContainText('기간을 좁혀'); await expect(page.getByRole('heading', { name: '이전에 조회한 결과', exact: true })).toBeVisible(); await expect(page.locator('.statistics-applied')).toContainText(query.get('from')!)
  await page.unrouteAll({ behavior: 'wait' }); await page.goBack(); await expect(page).toHaveURL(savedUrl); await expect(page.getByRole('heading', { name: '적용된 조회 기간', exact: true })).toBeVisible(); expect(await get(page, '/api/state')).toEqual(before)
  await page.getByRole('link', { name: '재료 관리', exact: true }).click(); await expect(page.getByLabel('재료 이름', { exact: false })).toHaveValue('가상 저장 전 이름')
  for (const suffix of ['', '?from=bad&until=bad', '?from=2026-01-01T00%3A00%3A00Z&until=2027-01-01T00%3A00%3A00Z&extra=1']) { const response = await page.request.get(new URL(`/api/statistics${suffix}`, launchUrl).toString()); expect(response.status()).toBe(400) }
})

test('current corrections stay at original time and included receipt effects remain separate from sales consumption', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 포함과 원시각'), initial = await observe(page, raw, '10', '100'), received = await receipt(page, raw, '2', '40', { kind: 'reconciled', selections: [{ observation_version_id: initial.observations[0].version_id, source: { kind: 'all_available' }, included_quantity: '2' }] })
  const menu = await recipe(page, '가상 원가 고정 메뉴', [{ item: raw, quantity: '1' }]), selling = await sale(page, menu, '1'), original = await get(page, `/api/sales/${selling.result.sale_id}`)
  const changed = await correction(page, selling.id, draft => { draft.action.replacement.data.count = '2' })
  await archive(page, menu.item.id); const range = { from: original.item.recorded_at, until: changed.recorded_at, zone: 'UTC' }; const report = await show(page, range)
  expect(report.sales.rows).toHaveLength(1); expect(report.sales.rows[0].value.count).toBe('2'); expect(report.sales.rows[0].origin.recorded_at).toBe(original.item.recorded_at); expect(report.sales.rows[0].original.recipe_cost).toEqual(original.item.recipe_cost)
  await expect(page.locator('#statistics-sale-detail a[href^="/corrections/"]')).toHaveAttribute('href', `/corrections/${changed.result.correction_id}`)
  const whole = await show(page), entry = whole.operations.rows.find((row: any) => row.origin.id === received.result.receipt_id)
  expect(entry.receipt_basis.original.quantity).toBe('2'); expect(entry.receipt_basis.effective.amount).toBe('40'); expect(entry.value.reconciliation.unreflected_quantity).toBe('0'); expect(entry.value.reconciliation.value_difference).toBe('20')
  await page.getByLabel('이력 종류').selectOption('receipt'); await page.locator('.statistics-records summary').filter({ hasText: /^입고/ }).click(); await expect(page.locator('.receipt-reconciliation-result')).toContainText('현재 새로 더할 수량'); await expect(page.locator('.receipt-reconciliation-result')).toContainText('0g'); await expect(page.locator('.receipt-reconciliation-result')).toContainText('20원')
})

test('report survives normal restart and is usable with keyboard and six viewport themes', async ({ page, browserName }) => {
  test.setTimeout(270_000); await open(page); const data = await mixed(page); await show(page); await page.getByLabel('연결을 볼 품목').selectOption(data.menu.item.id)
  await expect(page).toHaveURL(new RegExp(`material=${data.menu.item.id}`))
  const path = new URL(page.url()).pathname + new URL(page.url()).search; await page.goto('about:blank')
  let timer: ReturnType<typeof setTimeout> | undefined; const [code, signal] = await Promise.race([closed, new Promise<never>((_, reject) => { timer = setTimeout(() => reject(Error('statistics normal shutdown timed out')), 180_000) })]).finally(() => clearTimeout(timer)); expect(code).toBe(0); expect(signal).toBeNull(); await launch(); await open(page); await visit(page, path)
  await expect(page.getByRole('heading', { name: '적용된 조회 기간', exact: true })).toBeVisible(); await expect(page.getByLabel('연결을 볼 품목')).toHaveValue(data.menu.item.id)
  await page.getByRole('button', { name: '기간 조회', exact: true }).focus(); await page.keyboard.press('Tab'); await expect(page.getByRole('link', { name: '판매·원가', exact: true })).toBeFocused(); await page.keyboard.press('Enter'); await expect(page).toHaveURL(/#statistics-sales-title$/)
  await page.getByLabel('시계열 값').focus(); await page.keyboard.press('ArrowDown'); await page.keyboard.press('Enter'); await expect(page.getByLabel('시계열 값')).toHaveValue('count')
  const selected = page.getByRole('button', { name: data.menu.item.name, exact: true }).last(); await selected.focus(); await page.keyboard.press('Space'); await expect(selected).toHaveAttribute('aria-pressed', 'true'); await expect(page.locator('.chart-selection')).toContainText('1개')
  const details = page.getByText('당시 레시피 기준 원가 · 1개 기준', { exact: true }); await details.focus(); await page.keyboard.press('Enter'); await expect(page.locator('.sale-cost-snapshot')).toBeVisible()
  for (const [width, height] of [[1440, 900], [768, 1024], [390, 844]] as const) { await page.setViewportSize({ width, height }); for (const theme of ['light', 'dark'] as const) { if (await page.locator('html').getAttribute('data-theme') !== theme) await page.getByRole('button', { name: theme === 'dark' ? '다크 모드로 전환' : '라이트 모드로 전환' }).click(); await expect(page.locator('html')).toHaveAttribute('data-theme', theme); expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true); await page.screenshot({ path: test.info().outputPath(`statistics-${width}-${theme}.png`), fullPage: true }) } }
  expect(await page.evaluate(() => { const ids = [...document.querySelectorAll('[id]')].map(item => item.id); return ids.filter((id, index) => ids.indexOf(id) !== index) })).toEqual([])
  console.log(`statistics keyboard and six layouts: ${browserName}; ${await page.evaluate(() => navigator.userAgent)}`)
})
