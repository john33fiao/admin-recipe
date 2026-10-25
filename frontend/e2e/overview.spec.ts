import { test, expect, type Page } from '@playwright/test'
import { spawn, type ChildProcess } from 'node:child_process'
import { once } from 'node:events'
import { randomUUID, createHash } from 'node:crypto'
import { cpSync, mkdtempSync, rmSync, readdirSync, readFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

const binaryName = process.platform === 'win32' ? 'sunchang-hansang.exe' : 'sunchang-hansang'
let directory: string, binary: string, server: ChildProcess, closed: Promise<[number | null, NodeJS.Signals | null]>, launchUrl: string
async function launch() {
  server = spawn(binary, ['--no-browser'], { cwd: directory, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] }); closed = once(server, 'close') as typeof closed
  launchUrl = await new Promise<string>((resolveUrl, reject) => {
    let output = '', errors = ''; const timer = setTimeout(() => reject(Error(`overview fixture start timed out: ${errors}`)), 15_000)
    server.stderr!.on('data', data => { errors += String(data) }); server.once('exit', code => { clearTimeout(timer); reject(Error(`overview fixture exited ${code}: ${errors}`)) })
    server.stdout!.on('data', data => { output += String(data); const match = output.match(/http:\/\/127\.0\.0\.1:\d+\/#token=[a-f0-9]+/); if (match) { clearTimeout(timer); resolveUrl(match[0]) } })
  })
}
test.beforeEach(async () => { directory = mkdtempSync(join(tmpdir(), 'sunchang-overview-')); binary = join(directory, binaryName); cpSync(resolve('..', 'target', 'release', binaryName), binary); await launch() })
test.afterEach(async () => { if (server && server.exitCode === null && server.signalCode === null) server.kill(); if (closed) await closed; if (directory) { expect(resolve(directory).startsWith(resolve(tmpdir()))).toBe(true); rmSync(directory, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 }) } })
test.setTimeout(120_000)
async function open(page: Page) { await page.goto(launchUrl); await expect(page.getByLabel('재료 이름', { exact: false })).toBeEnabled() }
async function visit(page: Page, path: string) { await page.goto(new URL(path, launchUrl).toString()) }
async function get(page: Page, path: string) { const response = await page.request.get(new URL(path, launchUrl).toString()); expect(response.ok(), await response.text()).toBe(true); return response.json() }
async function post(page: Page, path: string, data: unknown) { const response = await page.request.post(new URL(path, launchUrl).toString(), { headers: { Origin: new URL(launchUrl).origin }, data }); expect(response.status(), await response.text()).toBe(201); return response.json() }
async function meta(page: Page) { const state = await get(page, '/api/state'); return { request_id: randomUUID(), epoch: state.epoch, expected_revision: state.revision } }
async function material(page: Page, name: string) { return post(page, '/api/materials', { ...await meta(page), name, unit: 'g', kind: 'raw', description: '' }) }
async function draft(page: Page, kind: string, data: unknown, corrects: string | null = null) { return post(page, kind === 'correction' ? '/api/corrections/drafts' : kind === 'stocktake' ? '/api/stocktake-drafts' : kind === 'production' ? '/api/production-drafts' : '/api/business/drafts', { meta: await meta(page), payload: { kind, version: 1, data }, corrects_confirmation_id: corrects }) }
async function confirm(page: Page, kind: string, data: unknown, corrects: string | null = null) { const saved = await draft(page, kind, data, corrects); return (await post(page, `/api/business/${saved.result.item.id}/confirm`, { meta: await meta(page), expected_head_id: saved.result.head.id })).result.confirmation }
async function observe(page: Page, item: any, quantity: string, amount?: string) { const saved = await confirm(page, 'stocktake', { rows: [{ material_id: item.id, material_version_id: item.version_id, quantity, unit: item.unit, initial_value: amount === undefined ? null : { kind: 'manual_amount', amount }, excess: null }], note: '' }); return get(page, `/api/stocktakes/${saved.result.stocktake_id}`) }
async function receipt(page: Page, item: any, quantity: string, amount: string, stock_effect: unknown = { kind: 'unreflected' }) { return confirm(page, 'receipt', { source: { kind: 'direct', material_id: item.id, material_version_id: item.version_id }, quantity, unit: item.unit, amount, receipt_date: '2024-02-29', note: '', stock_effect }) }
async function correction(page: Page, confirmation: string, change: (draft: any, seed: any) => void) { const seed = await get(page, `/api/business/confirmations/${confirmation}/correction-seed`), data = seed.draft; data.reason = '가상 운영 개요 정정'; change(data, seed); return confirm(page, 'correction', data, seed.original.reference.original_confirmation_id) }
async function menu(page: Page, item: any, name = '가상 운영 메뉴') { return post(page, '/api/recipes', { meta: await meta(page), owner: { mode: 'new', name, unit: '개', kind: 'menu', description: '' }, data: { basis_count: '1', reference_output: null, selling_price: '50', cooking_description: '', process: '', reference_time: '', lines: [{ material_id: item.id, material_version_id: item.version_id, quantity: '0', unit: item.unit, note: '' }] } }) }
async function sale(page: Page, recipe: any, count: string) { return confirm(page, 'sale', { material_id: recipe.item.id, material_version_id: recipe.item.version_id, recipe_version_id: recipe.version.id, count, note: '' }) }
async function show(page: Page) { await visit(page, '/overview'); await expect(page.locator('.overview-count')).toBeVisible(); return get(page, '/api/overview') }
const section = (page: Page, name: string) => page.locator(`[data-overview-section="${name}"]`)
const itemRow = (page: Page, id: string) => section(page, 'materials').locator(`.overview-materials > [data-overview-row="${id}"]`)
async function expand(page: Page, name: string) { const details = section(page, name).locator(':scope > details'); if (await details.getAttribute('open') === null) await details.locator(':scope > summary').click() }
function hashes() { const root = join(directory, 'db', 'snapshots'); return readdirSync(root).sort().map(name => [name, createHash('sha256').update(readFileSync(join(root, name))).digest('hex')]) }

test('overview separates absent and initial zero measurements, archived count and actual entry links', async ({ page }) => {
  await open(page); await show(page); await expect(page.locator('.overview-count')).toHaveText('0개'); await expect(page.getByRole('link', { name: '첫 재료 등록하기' })).toBeVisible()
  const unchecked = await material(page, '가상 미확인'), zero = await material(page, '가상 처음 확인한 0'), raw = await material(page, '가상 확인 이후 입고')
  await observe(page, zero, '0', '0'); await observe(page, raw, '10', '100'); await receipt(page, raw, '2', '30')
  await post(page, `/api/materials/${unchecked.id}/archive`, { ...await meta(page), expected_version_id: unchecked.version_id })
  const summary = await show(page); expect(summary.materials.total_count).toBe(3); expect(summary.materials.rows.at(-1).material.id).toBe(unchecked.id)
  await expect(page.locator('.overview-count')).toHaveText('3개'); await expect(page.getByText('전체 품목(보관 포함)', { exact: true })).toBeVisible()
  await expect(itemRow(page, unchecked.id)).toContainText('아직 확인한 수량 없음'); await expect(itemRow(page, unchecked.id)).toContainText('보관')
  await expect(itemRow(page, zero.id)).toContainText('0g'); await expect(itemRow(page, zero.id)).toContainText('처음 확인한 기준'); expect(summary.materials.rows.find((row: any) => row.material.id === zero.id).last_observation.consumption).toBeNull()
  await expect(itemRow(page, raw.id)).toContainText('10g'); await itemRow(page, raw.id).locator('.overview-change-details > summary').click(); await expect(itemRow(page, raw.id)).toContainText('입고 2g · 30원')
  await expect(itemRow(page, raw.id).getByRole('link', { name: '이 품목 수량 확인', exact: true })).toHaveAttribute('href', `/business?kind=stocktake&material=${raw.id}`)
  await itemRow(page, raw.id).getByRole('link', { name: '이 품목 수량 확인', exact: true }).click(); await expect(page.getByRole('heading', { name: '실사 초안', exact: true })).toBeVisible(); await expect(page.getByLabel('실사 1 품목', { exact: true })).toHaveValue(raw.id)
})

test('last active measurement follows logical events through correction, void fallback and restoration', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 실측 원시각과 정정'), first = await observe(page, raw, '10', '100'), incoming = await receipt(page, raw, '15', '150')
  const changed = await correction(page, first.item.confirmation_id, data => { data.action.replacement.data.rows[0].action.quantity = '30' })
  let summary = await show(page), entry = summary.materials.rows.find((row: any) => row.material.id === raw.id)
  expect(entry.last_observation.quantity).toBe('30'); expect(entry.last_observation.observed_at).toBe(first.item.recorded_at); expect(entry.changes_after_observation.rows.map((row: any) => row.origin.id)).toEqual([incoming.result.receipt_id])
  await expect(itemRow(page, raw.id).locator(`a[href="/corrections/${changed.result.correction_id}"]`)).toBeVisible()
  const second = await observe(page, raw, '40'); const later = await receipt(page, raw, '2', '20')
  await correction(page, second.item.confirmation_id, data => { data.action = { kind: 'void' } }); summary = await show(page); entry = summary.materials.rows.find((row: any) => row.material.id === raw.id)
  expect(entry.last_observation.observation_id).toBe(first.observations[0].id); expect(entry.last_observation.quantity).toBe('30'); expect(entry.changes_after_observation.rows.map((row: any) => row.origin.id)).toEqual([later.result.receipt_id, incoming.result.receipt_id])
  await correction(page, second.item.confirmation_id, (data, seed) => { data.action = { kind: 'replace', replacement: seed.replacement_template } }); summary = await show(page); entry = summary.materials.rows.find((row: any) => row.material.id === raw.id)
  expect(entry.last_observation.observation_id).toBe(second.observations[0].id); expect(entry.last_observation.quantity).toBe('40'); expect(entry.changes_after_observation.rows.map((row: any) => row.origin.id)).toEqual([later.result.receipt_id]); await expect(itemRow(page, raw.id)).toContainText('40g')
})

test('included receipts, same-material production, returns and waste retain distinct effects and stable links', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 양방향 생산과 포함입고'), initial = await observe(page, raw, '10', '100')
  await receipt(page, raw, '2', '40', { kind: 'reconciled', selections: [{ observation_version_id: initial.observations[0].version_id, source: { kind: 'all_available' }, included_quantity: '2' }] })
  const production = await confirm(page, 'production', { inputs: [{ material_id: raw.id, material_version_id: raw.version_id, quantity: '2', unit: 'g' }], outputs: [{ material_id: raw.id, material_version_id: raw.version_id, quantity: '1', unit: 'g', allocation_percent: '100' }], note: '' })
  const extra = await receipt(page, raw, '3', '30'), source = await get(page, `/api/receipts/${extra.result.receipt_id}/return-source`)
  const returned = await confirm(page, 'purchase_return', { receipt_id: source.original.id, expected_receipt_version_id: source.current.reference.version_id, quantity: '1', unit: 'g', note: '' })
  const wasted = await confirm(page, 'waste', { material_id: raw.id, material_version_id: raw.version_id, quantity: '1', unit: 'g', reason: 'discard', note: '' })
  const summary = await show(page), entry = summary.materials.rows.find((row: any) => row.material.id === raw.id), effects = entry.changes_after_observation.rows
  expect(effects).toHaveLength(5); expect(entry.last_observation.quantity).toBe('10')
  const productionRow = effects.find((row: any) => row.effect.kind === 'production'); expect([productionRow.effect.input_quantity, productionRow.effect.output_quantity]).toEqual(['2', '1'])
  const included = effects.find((row: any) => row.effect.kind === 'reconciled_receipt'); expect([included.effect.receipt.quantity, included.effect.included_quantity, included.effect.inflow_quantity, included.effect.value_delta]).toEqual(['2', '2', '0', '20'])
  await itemRow(page, raw.id).locator('.overview-change-details > summary').click(); await expect(itemRow(page, raw.id)).toContainText('이 품목 새 유입 0g'); await expect(itemRow(page, raw.id)).toContainText('투입 2g'); await expect(itemRow(page, raw.id)).toContainText('결과 1g')
  for (const [path, id] of [['productions', production.result.production_id], ['purchase-returns', returned.result.purchase_return_id], ['wastes', wasted.result.waste_id]]) await expect(itemRow(page, raw.id).locator(`a[href="/${path}/${id}"]`)).toBeVisible()
})

test('open and received orders, no-sales, zero-weight and explicit zero allocations remain different', async ({ page }) => {
  await open(page); const ordered = await material(page, '가상 주문 재개'), raw = await material(page, '가상 소모 상태')
  const placed = await confirm(page, 'purchase_order', { action: 'place', material_id: ordered.id, material_version_id: ordered.version_id, quantity: '10', unit: 'g', supplier: '가상 구입처', expected_date: '', estimated_amount: '', note: '' })
  async function orderReceipt(quantity: string) { const order = await get(page, `/api/orders/${placed.result.order_id}`); return confirm(page, 'receipt', { source: { kind: 'order', order_id: order.item.id, expected_order_version_id: order.head.id, material_version_id: order.head.material_version_id }, quantity, unit: 'g', amount: '100', receipt_date: '2024-02-29', note: '', stock_effect: { kind: 'unreflected' } }) }
  let summary = await show(page); expect(summary.order_counts.open).toBe(1)
  const received = await orderReceipt('8'); summary = await show(page); expect(summary.received_orders.rows[0].closure.difference).toBe('-2'); await expand(page, 'received_orders'); await expect(section(page, 'received_orders')).toContainText('차이 -2g')
  await correction(page, received.id, data => { data.action = { kind: 'void' } }); summary = await show(page); expect(summary.order_counts.open).toBe(1); expect(summary.received_orders.total_count).toBe(0)
  const replacement = await orderReceipt('10'); summary = await show(page); expect(summary.received_orders.rows[0].closure.receipt_id).toBe(replacement.result.receipt_id); expect(summary.received_orders.rows[0].closure.difference).toBe('0')
  await observe(page, raw, '10', '100'); await observe(page, raw, '8'); const recipe = await menu(page, raw), zeroSale = await sale(page, recipe, '0'); await observe(page, raw, '6')
  await sale(page, recipe, '2'); await observe(page, raw, '6')
  const missing = await material(page, '가상 아직 연결하지 않은 품목'), missingMenu = await menu(page, missing, '가상 미확정 메뉴'); await sale(page, missingMenu, '0'); await observe(page, missing, '0', '0')
  summary = await show(page); expect(summary.unallocated.rows.map((row: any) => row.observation.consumption.reason).sort()).toEqual(['no_sales', 'zero_weight']); expect(summary.unconfirmed.total_count).toBe(1); expect(summary.unconfirmed.rows[0].reason).toBe('initial_basis_missing')
  const zero = summary.sales.rows.find((row: any) => row.origin.id === zeroSale.result.sale_id); expect(zero.count).toBe('0'); expect(zero.amount).toBe('0')
  const allocated = summary.sales.rows.find((row: any) => row.count === '2'); expect(allocated.consumption_counts.allocated).toBe(1); expect(allocated.consumption_counts.unconfirmed).toBe(0)
  await expand(page, 'unallocated'); await expect(section(page, 'unallocated')).toContainText('판매 없음'); await expect(section(page, 'unallocated')).toContainText('총 판매 개수 0'); await expand(page, 'unconfirmed'); await expect(section(page, 'unconfirmed')).toContainText('처음 확인한 기준만')
})

test('exact material and draft pages preserve old data on 409 and restart only through a fresh summary', async ({ page }) => {
  await open(page); const items = []; for (let n = 0; n < 26; n += 1) items.push(await material(page, `가상 전체 목록 ${String(n).padStart(2, '0')}`))
  for (let n = 0; n < 21; n += 1) await draft(page, 'catalog_price', { material_id: items[0].id, material_version_id: items[0].version_id, quantity: '', amount: '', unit: 'g', note: `가상 미완성 ${n}` })
  const initial = await show(page); expect(initial.materials.total_count).toBe(26); expect(initial.materials.returned_count).toBe(25); expect(initial.drafts.total_count).toBe(21)
  await page.getByRole('link', { name: '마지막 확인 수량 전체 보기', exact: true }).click(); await expect(page.locator('.overview-page-count')).toHaveText('전체 26건 중 1–25건'); await expect(page.locator('.overview-materials > li')).toHaveCount(25)
  await material(page, '가상 전체 목록 26'); await page.getByRole('link', { name: '다음 페이지', exact: true }).click(); await expect(page.getByRole('alert')).toContainText('기록이 바뀌어'); await expect(page.locator('.overview-page-count')).toHaveText('전체 26건 중 1–25건'); await expect(page.locator('.overview-materials > li')).toHaveCount(25)
  await page.getByRole('button', { name: '전체 다시 조회하고 첫 페이지로' }).click(); await expect(page).toHaveURL(/offset=0/); await expect(page.locator('.overview-page-count')).toHaveText('전체 27건 중 1–25건')
  const first = await page.locator('.overview-materials > li').evaluateAll(rows => rows.map(row => row.getAttribute('data-overview-row'))); await page.getByRole('link', { name: '다음 페이지', exact: true }).click(); await expect(page.locator('.overview-page-count')).toHaveText('전체 27건 중 26–27건'); const last = await page.locator('.overview-materials > li').evaluateAll(rows => rows.map(row => row.getAttribute('data-overview-row'))); expect(new Set([...first, ...last]).size).toBe(27)
  await visit(page, '/overview/list?section=drafts&offset=0'); await expect(page.locator('.overview-page-count')).toHaveText('전체 21건 중 1–20건'); await page.getByRole('link', { name: '다음 페이지', exact: true }).click(); await expect(page.locator('.overview-page-count')).toHaveText('전체 21건 중 21–21건')
  const state = await get(page, '/api/state'), before = hashes()
  for (const name of ['materials', 'material_changes', 'open_orders', 'received_orders', 'receipts', 'sales', 'observations', 'unallocated', 'unconfirmed', 'drafts']) { const query = new URLSearchParams({ section: name, offset: '1000', epoch: state.epoch, revision: state.revision }); if (name === 'material_changes') query.set('material_id', items[0].id); const list = await get(page, `/api/overview/list?${query}`); expect(list.list.page.rows).toEqual([]); expect(list.list.page.next_offset).toBeNull() }
  expect(await get(page, '/api/state')).toEqual(state); expect(hashes()).toEqual(before)
})

test('DB drafts, local fields and an uncertain original request survive overview reads and failures', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 DB 초안 품목'); await draft(page, 'catalog_price', { material_id: raw.id, material_version_id: raw.version_id, quantity: '', amount: '', unit: 'g', note: '' })
  await page.getByLabel('재료 이름', { exact: false }).fill('가상 응답 확인 전 등록'); await page.getByLabel('관리 단위', { exact: false }).fill('g')
  const requests: any[] = []; page.on('request', request => { if (request.method() === 'POST' && new URL(request.url()).pathname === '/api/materials') requests.push(request.postDataJSON()) })
  await page.route('**/api/materials', async route => { if (route.request().method() !== 'POST') return route.continue(); await route.fetch(); await route.abort('failed') })
  await page.getByRole('button', { name: '재료 등록', exact: true }).click(); await expect(page.getByRole('button', { name: '같은 요청 재시도', exact: true })).toBeEnabled(); await page.unrouteAll({ behavior: 'wait' })
  const pending = await page.evaluate(() => Object.fromEntries(Object.entries(sessionStorage).filter(([key]) => key.includes('pending')))); expect(Object.keys(pending).length).toBeGreaterThan(0)
  await page.getByRole('navigation', { name: '주요 메뉴' }).getByRole('link', { name: '작업 시작', exact: true }).click(); await expect(page.locator('.overview-count')).toHaveText('2개'); await expand(page, 'drafts'); await expect(section(page, 'drafts')).toContainText('전체 1건'); await expect(section(page, 'drafts')).toContainText('확정 가능 여부 미평가')
  const state = await get(page, '/api/state'); await page.route('**/api/overview', route => route.fulfill({ status: 503, contentType: 'application/json', body: JSON.stringify({ error: '가상 조회 실패', code: 'unavailable' }) })); await page.getByRole('button', { name: '전체 새로고침', exact: true }).click(); await expect(page.getByRole('alert')).toContainText('가상 조회 실패'); await expect(page.getByText('이전에 조회한 자료입니다.', { exact: true })).toBeVisible(); await expect(page.locator('.overview-count')).toHaveText('2개')
  expect(await page.evaluate(() => Object.fromEntries(Object.entries(sessionStorage).filter(([key]) => key.includes('pending'))))).toEqual(pending); expect(await get(page, '/api/state')).toEqual(state); await page.unrouteAll({ behavior: 'wait' })
  await page.getByRole('link', { name: '재료 관리', exact: true }).click(); await expect(page.getByLabel('재료 이름', { exact: false })).toHaveValue('가상 응답 확인 전 등록'); await page.getByRole('button', { name: '같은 요청 재시도', exact: true }).click(); await expect(page.getByText('재료를 등록했습니다.', { exact: false })).toBeVisible(); expect(requests[1]).toEqual(requests[0]); expect(await get(page, '/api/materials')).toHaveLength(2)
})

test('overview lists survive normal restart and support keyboard navigation in six viewport themes', async ({ page, browserName }) => {
  test.setTimeout(270_000); await open(page); const raw = await material(page, '가상 마지막 수량과 여러 확정 변동의 근거를 함께 확인하는 긴 품목 이름'), zero = await material(page, '가상 처음 확인한 0 수량'), unchecked = await material(page, '가상 아직 확인하지 않은 품목')
  await observe(page, raw, '10', '100'); await receipt(page, raw, '2', '40'); await observe(page, zero, '0', '0'); await draft(page, 'catalog_price', { material_id: unchecked.id, material_version_id: unchecked.version_id, quantity: '', amount: '', unit: 'g', note: '가상 미완성' }); await show(page)
  const countLink = page.getByRole('link', { name: '마지막 확인 수량 전체 보기', exact: true }); await countLink.focus(); await page.keyboard.press('Enter'); await expect(page).toHaveURL(/\/overview\/list\?section=materials&offset=0/); await expect(page.getByRole('heading', { name: '마지막 확인 수량', exact: true })).toBeFocused()
  const path = new URL(page.url()).pathname + new URL(page.url()).search; await page.goto('about:blank'); let timer: ReturnType<typeof setTimeout> | undefined
  const [code, signal] = await Promise.race([closed, new Promise<never>((_, reject) => { timer = setTimeout(() => reject(Error('overview normal shutdown timed out')), 180_000) })]).finally(() => clearTimeout(timer)); expect(code).toBe(0); expect(signal).toBeNull(); await launch(); await open(page); await visit(page, path)
  await expect(page.locator('.overview-page-count')).toHaveText('전체 3건 중 1–3건'); await expect(itemRow(page, raw.id)).toContainText('10g'); await page.getByRole('link', { name: '작업 시작으로', exact: true }).click(); await expect(page.locator('.overview-count')).toHaveText('3개')
  await page.getByRole('button', { name: '전체 새로고침', exact: true }).focus(); await page.keyboard.press('Tab'); await expect(page.getByRole('link', { name: '모아 입력', exact: true }).last()).toBeFocused()
  const change = itemRow(page, raw.id).locator('.overview-change-details > summary'); await change.focus(); await page.keyboard.press('Space'); await expect(itemRow(page, raw.id).locator('.overview-changes')).toBeVisible(); await expand(page, 'drafts')
  for (const [width, height] of [[1440, 900], [768, 1024], [390, 844]] as const) { await page.setViewportSize({ width, height }); for (const theme of ['light', 'dark'] as const) { if (await page.locator('html').getAttribute('data-theme') !== theme) await page.getByRole('button', { name: theme === 'dark' ? '다크 모드로 전환' : '라이트 모드로 전환' }).click(); await expect(page.locator('html')).toHaveAttribute('data-theme', theme); expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true); await page.screenshot({ path: test.info().outputPath(`overview-${width}-${theme}.png`), fullPage: true }) } }
  expect(await page.evaluate(() => { const ids = [...document.querySelectorAll('[id]')].map(item => item.id); return ids.filter((id, index) => ids.indexOf(id) !== index) })).toEqual([])
  console.log(`overview keyboard and six layouts: ${browserName}; ${await page.evaluate(() => navigator.userAgent)}`)
})
