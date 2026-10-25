import { test, expect, type Page } from '@playwright/test'
import { spawn, type ChildProcess } from 'node:child_process'
import { once } from 'node:events'
import { randomUUID } from 'node:crypto'
import { cpSync, mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

const binaryName = process.platform === 'win32' ? 'sunchang-hansang.exe' : 'sunchang-hansang'
let directory: string, binary: string, server: ChildProcess, closed: Promise<[number | null, NodeJS.Signals | null]>, launchUrl: string
async function launch() {
  server = spawn(binary, ['--no-browser'], { cwd: directory, stdio: ['ignore', 'pipe', 'pipe'] })
  closed = once(server, 'close') as Promise<[number | null, NodeJS.Signals | null]>
  launchUrl = await new Promise<string>((resolveUrl, reject) => {
    let output = '', errors = ''
    const timer = setTimeout(() => reject(new Error(`reconciliation fixture start timed out: ${errors}`)), 15_000)
    server.stderr!.on('data', data => { errors += String(data) })
    server.once('exit', code => { clearTimeout(timer); reject(new Error(`reconciliation fixture exited ${code}: ${errors}`)) })
    server.stdout!.on('data', data => { output += String(data); const match = output.match(/http:\/\/127\.0\.0\.1:\d+\/#token=[a-f0-9]+/); if (match) { clearTimeout(timer); resolveUrl(match[0]) } })
  })
}
test.beforeEach(async () => { directory = mkdtempSync(join(tmpdir(), 'sunchang-reconciliations-')); binary = join(directory, binaryName); cpSync(resolve('..', 'target', 'release', binaryName), binary); await launch() })
test.afterEach(async () => { if (server && server.exitCode === null && server.signalCode === null) server.kill(); if (closed) await closed; if (directory) { expect(resolve(directory).startsWith(resolve(tmpdir()))).toBe(true); rmSync(directory, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 }) } })
test.setTimeout(120_000)
async function open(page: Page) { await page.goto(launchUrl); await expect(page.getByLabel('재료 이름', { exact: false })).toBeEnabled() }
async function visit(page: Page, path: string) { await page.goto(new URL(path, launchUrl).toString()) }
async function get(page: Page, path: string) { const response = await page.request.get(new URL(path, launchUrl).toString()); expect(response.ok(), await response.text()).toBe(true); return response.json() }
async function post(page: Page, path: string, data: unknown) { const response = await page.request.post(new URL(path, launchUrl).toString(), { headers: { Origin: new URL(launchUrl).origin }, data }); expect(response.status(), await response.text()).toBe(201); return response.json() }
async function meta(page: Page) { const state = await get(page, '/api/state'); return { request_id: randomUUID(), epoch: state.epoch, expected_revision: state.revision } }
async function material(page: Page, name: string, unit = 'g') { return post(page, '/api/materials', { ...await meta(page), name, unit, kind: 'raw', description: '' }) }
async function draft(page: Page, kind: string, data: unknown) { return post(page, kind === 'stocktake' ? '/api/stocktake-drafts' : kind === 'production' ? '/api/production-drafts' : '/api/business/drafts', { meta: await meta(page), payload: { kind, version: 1, data }, corrects_confirmation_id: null }) }
async function apiConfirm(page: Page, value: any) { return post(page, `/api/business/${value.result.item.id}/confirm`, { meta: await meta(page), expected_head_id: value.result.head.id }) }
async function observe(page: Page, item: any, quantity: string, initialValue?: string) {
  const saved = await apiConfirm(page, await draft(page, 'stocktake', { rows: [{ material_id: item.id, material_version_id: item.version_id, quantity, unit: item.unit, initial_value: initialValue === undefined ? null : { kind: 'manual_amount', amount: initialValue }, excess: null }], note: '' }))
  return get(page, `/api/stocktakes/${saved.result.confirmation.result.stocktake_id}`)
}
async function ordinaryReceipt(page: Page, item: any, quantity: string, amount: string) { return apiConfirm(page, await draft(page, 'receipt', { source: { kind: 'direct', material_id: item.id, material_version_id: item.version_id }, quantity, unit: item.unit, amount, receipt_date: '2024-02-29', note: '', stock_effect: { kind: 'unreflected' } })) }
async function waste(page: Page, item: any, quantity: string) { return apiConfirm(page, await draft(page, 'waste', { material_id: item.id, material_version_id: item.version_id, quantity, unit: item.unit, reason: 'discard', note: '' })) }
async function start(page: Page, item: any, orderId?: string) {
  await visit(page, `/business?kind=receipt&${orderId ? `order=${orderId}` : `material=${item.id}`}`)
  const reset = page.getByRole('button', { name: '새 입고 초안 작성', exact: true }); if (await reset.isVisible()) await reset.click()
  await expect(page.getByRole('heading', { name: '실제 입고 초안', exact: true })).toBeVisible()
}
async function fill(page: Page, quantity = '3', amount = '6000', unit = 'kg') { await page.getByLabel('실제 입고 수량', { exact: true }).fill(quantity); await page.getByLabel('입고 단위', { exact: true }).fill(unit); await page.getByLabel('실제 구입금액 (원)', { exact: true }).fill(amount); await page.getByLabel('실제 입고일', { exact: true }).fill('2024-02-29') }
async function choose(page: Page, observationId: string, quantity: string, index = 0) {
  await page.locator(`#receipt-inclusion-${index}-observation_version_id`).selectOption(observationId)
  await page.locator(`#receipt-inclusion-${index}-source`).selectOption('all')
  await page.locator(`#receipt-inclusion-${index}-included_quantity`).fill(quantity)
}
async function selectMode(page: Page) { await page.getByLabel('재고 반영 구분', { exact: true }).selectOption('reconciled') }
async function openSaved(page: Page) { const link = page.getByRole('link', { name: '저장한 초안 열기', exact: true }); await expect(link).toBeVisible(); const path = (await link.getAttribute('href'))!; await link.click(); await expect(page).toHaveURL(new URL(path, launchUrl).toString()); await expect(page.getByRole('button', { name: '확정 전 미리보기', exact: true })).toBeVisible(); return path }
async function save(page: Page) { await page.getByRole('button', { name: '초안 임시 저장', exact: true }).click(); return openSaved(page) }
async function revise(page: Page) { await page.getByRole('button', { name: '수정 내용 임시 저장', exact: true }).click(); await expect(page.getByText('수정 내용을 임시 저장했습니다.', { exact: true })).toBeVisible() }
async function preview(page: Page) { await page.getByRole('button', { name: '확정 전 미리보기', exact: true }).click(); await expect(page.getByRole('button', { name: '이 내용으로 확정', exact: true })).toBeEnabled() }
async function finish(page: Page) { await page.getByRole('button', { name: '이 내용으로 확정', exact: true }).click(); const link = page.getByRole('link', { name: '실제 입고 기록 보기', exact: true }); await expect(link).toBeVisible(); const path = (await link.getAttribute('href'))!; await link.click(); await expect(page).toHaveURL(new URL(path, launchUrl).toString()); await expect(page.getByRole('heading', { name: '실사 포함 입고의 보정 기록', exact: true })).toBeVisible(); return get(page, `/api${path}`) }
async function restart(page: Page) { await page.goto('about:blank'); let timer: ReturnType<typeof setTimeout> | undefined; const [code, signal] = await Promise.race([closed, new Promise<never>((_, reject) => { timer = setTimeout(() => reject(new Error('reconciliation normal shutdown timed out')), 180_000) })]).finally(() => clearTimeout(timer)); expect(code).toBe(0); expect(signal).toBeNull(); await launch(); await open(page) }

test('old reconciliation draft survives normal restart and explicit selections preserve actual receipt without double quantity', async ({ page }) => {
  test.setTimeout(260_000); await open(page)
  const raw = await material(page, '가상 늦게 확인한 입고 — 당시 실사와 현재 가치 보정을 나누어 보는 긴 이름')
  const stock = await observe(page, raw, '10000', '10000'), observation = stock.observations[0]
  await start(page, raw); await fill(page, '3.', '6000'); await page.getByLabel('재고 반영 구분', { exact: true }).selectOption('needs_reconciliation'); const path = await save(page)
  await restart(page); await visit(page, path); await expect(page.getByLabel('실제 입고 수량', { exact: true })).toHaveValue('3.'); await expect(page.getByLabel('재고 반영 구분', { exact: true })).toHaveValue('needs_reconciliation')
  await page.getByLabel('실제 입고 수량', { exact: true }).fill('3'); await selectMode(page); await choose(page, observation.version_id, '3000'); await revise(page); await preview(page)
  await expect(page.getByRole('region', { name: '실제 구입과 수량 반영' }).locator('dl > div').filter({ hasText: '현재 새로 더할 수량' }).locator('dd')).toHaveText('0g')
  const detail = await finish(page)
  expect(detail.item.quantity).toBe('3000'); expect(detail.item.amount).toBe('6000'); expect(detail.reconciliation.effect.unreflected_quantity).toBe('0'); expect(detail.reconciliation.effect.booked_value).toBe('3000'); expect(detail.reconciliation.effect.value_difference).toBe('3000')
  const holding = await get(page, `/api/holdings/${raw.id}`); expect(holding.settlement_basis.quantity).toBe('10000'); expect(holding.settlement_basis.value).toBe('13000'); expect(holding.observation.id).toBe(observation.version_id)
  for (const [width, height] of [[1440, 900], [768, 1024], [390, 844]] as const) { await page.setViewportSize({ width, height }); for (const theme of ['light', 'dark'] as const) { if (await page.locator('html').getAttribute('data-theme') !== theme) await page.getByRole('button', { name: theme === 'dark' ? '다크 모드로 전환' : '라이트 모드로 전환' }).click(); expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true); await page.screenshot({ path: test.info().outputPath(`reconciliation-${width}-${theme}.png`), fullPage: true }) } }
})

test('partial inclusion closes an order using actual receipt quantity and adds only the remainder', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 일부 포함 주문'); const stock = await observe(page, raw, '10000', '10000')
  const order = await apiConfirm(page, await draft(page, 'purchase_order', { action: 'place', material_id: raw.id, material_version_id: raw.version_id, quantity: '5000', unit: 'g', supplier: '', expected_date: '', estimated_amount: '', note: '' }))
  const orderId = order.result.confirmation.result.order_id
  await start(page, raw, orderId); await fill(page, '6', '9000'); await selectMode(page); await choose(page, stock.observations[0].version_id, '3000'); await save(page); await preview(page); const result = await finish(page)
  expect(result.item.quantity).toBe('6000'); expect(result.order_closure.received_quantity).toBe('6000'); expect(result.order_closure.difference).toBe('1000'); expect(result.reconciliation.effect.unreflected_quantity).toBe('3000'); expect(result.reconciliation.effect.included_amount).toBe('4500'); expect(result.reconciliation.effect.unreflected_amount).toBe('4500')
  const holding = await get(page, `/api/holdings/${raw.id}`); expect(holding.settlement_basis.quantity).toBe('13000'); expect(holding.settlement_basis.value).toBe('16000'); expect((await get(page, `/api/orders/${orderId}`)).item.status).toBe('received')
})

test('later measured consumption keeps its original values while a zero-price receipt records negative adjustments', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 음수 보정'); const first = await observe(page, raw, '10000', '10000'); const second = await observe(page, raw, '6000'); const before = JSON.stringify(second.observations)
  await start(page, raw); await fill(page, '3', '0'); await selectMode(page); await choose(page, first.observations[0].version_id, '3000'); await save(page); await preview(page)
  await expect(page.getByRole('region', { name: '이미 사용된 부분의 별도 보정' })).toContainText('-1,200원')
  const result = await finish(page); expect(result.reconciliation.effect.value_difference).toBe('-3000'); expect(result.reconciliation.effect.separate_adjustment).toBe('-1200'); expect(result.reconciliation.effect.materials[0].value_delta).toBe('-1800')
  expect(JSON.stringify((await get(page, `/api/stocktakes/${second.item.id}`)).observations)).toBe(before)
  const holding = await get(page, `/api/holdings/${raw.id}`); expect(holding.settlement_basis.quantity).toBe('6000'); expect(holding.settlement_basis.value).toBe('4200')
  const next = await observe(page, raw, '5000'); expect(next.observations[0].effect.calculation.consumption_quantity).toBe('1000'); expect(next.observations[0].effect.calculation.consumption_value).toBe('700')
})

test('production result values change without changing their units quantities or zero-percent values', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 원물'), liquid = await material(page, '가상 액체 결과', 'mL'), piece = await material(page, '가상 0원 부산물', '개'); const stock = await observe(page, raw, '10', '10')
  const production = await apiConfirm(page, await draft(page, 'production', { inputs: [{ material_id: raw.id, material_version_id: raw.version_id, quantity: '2', unit: 'g' }], outputs: [{ material_id: liquid.id, material_version_id: liquid.version_id, quantity: '1', unit: 'mL', allocation_percent: '100' }, { material_id: piece.id, material_version_id: piece.version_id, quantity: '1', unit: '개', allocation_percent: '0' }], note: '' }))
  const original = await get(page, `/api/productions/${production.result.confirmation.result.production_id}`)
  await start(page, raw); await fill(page, '3', '6', 'g'); await selectMode(page); await choose(page, stock.observations[0].version_id, '3'); await save(page); await preview(page); const result = await finish(page)
  expect(result.reconciliation.effect.separate_adjustment).toBe('0'); expect((await get(page, `/api/holdings/${raw.id}`)).settlement_basis.value).toBe('10.4')
  const a = await get(page, `/api/holdings/${liquid.id}`), b = await get(page, `/api/holdings/${piece.id}`); expect(a.settlement_basis.quantity).toBe('1'); expect(a.settlement_basis.value).toBe('2.6'); expect(b.settlement_basis.quantity).toBe('1'); expect(b.settlement_basis.value).toBe('0')
  const current = await get(page, `/api/productions/${production.result.confirmation.result.production_id}`); expect(current.item).toEqual(original.item); expect(current.materials).toEqual(original.materials)
  await expect(page.locator('.receipt-reconciliation-result')).toContainText('가상 액체 결과'); await expect(page.locator('.receipt-reconciliation-result')).toContainText('1mL')
})

test('fully consumed or unchanged receipts have usable details without fake holding records', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 전량 폐기 후 입고'); const stock = await observe(page, raw, '10', '10'); await waste(page, raw, '10')
  await start(page, raw); await fill(page, '3', '6', 'g'); await selectMode(page); await choose(page, stock.observations[0].version_id, '3'); await save(page); await preview(page); const result = await finish(page)
  expect(result.holding_version_id).toBeNull(); expect(result.reconciliation.material_results).toHaveLength(0); expect(result.reconciliation.effect.separate_adjustment).toBe('3'); await expect(page.locator('.receipt-detail')).toContainText('새 정산 근거를 만들지 않음')
  const holding = await get(page, `/api/holdings/${raw.id}`); expect(holding.settlement_basis.quantity).toBe('0'); expect(holding.settlement_basis.value).toBe('0')
  const unchanged = await material(page, '가상 동일 가치 보정'); const sameStock = await observe(page, unchanged, '10', '10')
  await start(page, unchanged); await fill(page, '3', '3', 'g'); await selectMode(page); await choose(page, sameStock.observations[0].version_id, '3'); await save(page); await preview(page); const same = await finish(page)
  expect(same.holding_version_id).toBeNull(); expect(same.reconciliation.material_results).toHaveLength(0); expect(same.reconciliation.effect.value_difference).toBe('0'); expect(same.reconciliation.effect.separate_adjustment).toBe('0')
})

test('uncertain nested requests retry unchanged and stale confirmation retains selected input for review', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 중첩 재시도'); const stock = await observe(page, raw, '10000', '10000'); await start(page, raw); await fill(page); await selectMode(page); await choose(page, stock.observations[0].version_id, '3000')
  const creates: unknown[] = []; await page.route('**/api/business/drafts', async route => { creates.push(route.request().postDataJSON()); if (creates.length === 1) { await route.fetch(); await route.abort('failed') } else await route.continue() })
  await page.getByRole('button', { name: '초안 임시 저장', exact: true }).click(); await expect(page.getByRole('button', { name: '같은 업무 요청 재시도' })).toBeVisible(); await page.route('**/receipt-inclusion-bases', route => route.abort('failed')); await page.reload(); await page.getByRole('button', { name: '같은 업무 요청 재시도' }).click(); const path = await openSaved(page); expect(creates[1]).toEqual(creates[0]); await page.unroute('**/receipt-inclusion-bases'); await page.getByRole('button', { name: '실사 근거 새로고침' }).click()
  const revisions: unknown[] = []; await page.route('**/api/business/*/draft-revisions', async route => { revisions.push(route.request().postDataJSON()); if (revisions.length === 1) { await route.fetch(); await route.abort('failed') } else await route.continue() }); await page.getByLabel('메모 (선택)', { exact: true }).fill('가상 입력 보존'); await page.getByRole('button', { name: '수정 내용 임시 저장' }).click(); await expect(page.getByRole('button', { name: '같은 업무 요청 재시도' })).toBeVisible(); await page.reload(); await page.getByRole('button', { name: '같은 업무 요청 재시도' }).click(); await expect(page.getByText('수정 내용을 임시 저장했습니다.', { exact: true })).toBeVisible(); expect(revisions[1]).toEqual(revisions[0]); await preview(page)
  await material(page, '가상 다른 화면 변경'); await page.getByRole('button', { name: '이 내용으로 확정' }).click(); await page.getByRole('button', { name: '최신 기록 확인 후 입력 검토' }).click(); await expect(page.locator('#receipt-inclusion-0-included_quantity')).toHaveValue('3000'); await preview(page)
  const confirmations: unknown[] = []; await page.route('**/api/business/*/confirm', async route => { confirmations.push(route.request().postDataJSON()); if (confirmations.length === 1) { await route.fetch(); await route.abort('failed') } else await route.continue() }); await page.getByRole('button', { name: '이 내용으로 확정' }).click(); await expect(page.getByRole('button', { name: '같은 업무 요청 재시도' })).toBeVisible(); await page.reload(); await page.getByRole('button', { name: '같은 업무 요청 재시도' }).click(); await expect(page.getByRole('link', { name: '실제 입고 기록 보기', exact: true })).toBeVisible(); expect(confirmations[1]).toEqual(confirmations[0]); expect((await get(page, `/api${path}`)).confirmations).toHaveLength(1); expect((await get(page, '/api/receipts')).items).toHaveLength(1)
})

test('all-available selection combines sub-minimum portions and duplicate inclusion cannot reuse the reservation', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 최소 수량 근거'); await ordinaryReceipt(page, raw, '0.000001', '0'); await ordinaryReceipt(page, raw, '0.000001', '0'); const stock = await observe(page, raw, '0.000001')
  const candidates = await get(page, `/api/materials/${raw.id}/receipt-inclusion-bases`), observed = candidates.items.find((item: any) => item.observation_version_id === stock.observations[0].version_id)
  expect(observed.available_quantity).toBe('0.000001'); expect(observed.sources).toHaveLength(2); expect(observed.sources.map((item: any) => item.available_quantity)).toEqual(['0', '0'])
  await start(page, raw); await fill(page, '0.000001', '0', 'g'); await selectMode(page); await choose(page, observed.observation_version_id, '0.000001'); await save(page); await preview(page); const result = await finish(page)
  expect(result.reconciliation.effect.selections[0].portions).toHaveLength(2); expect(result.reconciliation.effect.included_quantity).toBe('0.000001')
  const duplicate = await draft(page, 'receipt', { source: { kind: 'direct', material_id: raw.id, material_version_id: raw.version_id }, quantity: '0.000001', unit: 'g', amount: '0', receipt_date: '2024-02-29', note: '', stock_effect: { kind: 'reconciled', selections: [{ observation_version_id: observed.observation_version_id, source: { kind: 'all_available' }, included_quantity: '0.000001' }] } })
  const path = `/business/${duplicate.result.item.id}`; await visit(page, path); await page.getByRole('button', { name: '확정 전 미리보기' }).click(); await expect(page.getByRole('button', { name: '이 내용으로 확정' })).toBeDisabled(); expect((await get(page, `/api${path}`)).head.payload.data.stock_effect.selections[0].included_quantity).toBe('0.000001'); expect((await get(page, '/api/receipts')).items).toHaveLength(3)
})

test('candidate responses cannot replace a new material selection and keyboard row actions retain exact input', async ({ page }) => {
  await open(page); const first = await material(page, '가상 이전 근거'), next = await material(page, '가상 다음 근거'); const firstStock = await observe(page, first, '10', '10'), nextStock = await observe(page, next, '10', '20')
  await start(page, first); await fill(page, '2', '4', 'g'); await selectMode(page); await choose(page, firstStock.observations[0].version_id, '1')
  let release = () => {}; const gate = new Promise<void>(resolve => { release = resolve }); await page.route(`**/api/materials/${first.id}/receipt-inclusion-bases`, async route => { await gate; await route.continue() }); await page.getByRole('button', { name: '실사 근거 새로고침' }).click(); await page.getByLabel('입고 재료', { exact: true }).selectOption(next.id); await expect(page.locator('#receipt-inclusion-0-included_quantity')).toHaveValue('1'); const late = page.waitForResponse(response => new URL(response.url()).pathname === `/api/materials/${first.id}/receipt-inclusion-bases`); release(); await late
  await page.locator('#receipt-inclusion-0-observation_version_id').selectOption(nextStock.observations[0].version_id)
  const sourceSelect = page.locator('#receipt-inclusion-0-source')
  await sourceSelect.focus()
  // macOS headless native selects do not commit ArrowDown/Home (isolated
  // native-select probe). Keep the same selected value and keyboard row flow.
  if (process.platform === 'darwin') await sourceSelect.selectOption('all')
  else { await page.keyboard.press('Home'); await page.keyboard.press('ArrowDown') }
  await expect(sourceSelect).toHaveValue('all')
  await page.keyboard.press('Tab'); await expect(page.locator('#receipt-inclusion-0-included_quantity')).toBeFocused()
  await page.getByRole('button', { name: '포함 근거 추가', exact: true }).focus(); await page.keyboard.press('Space'); await expect(page.locator('#receipt-inclusion-1-observation_version_id')).toBeFocused(); await page.getByRole('button', { name: '포함 근거 2 삭제', exact: true }).focus(); await page.keyboard.press('Enter'); await expect(page.locator('#receipt-inclusion-0-observation_version_id')).toBeFocused()
  const path = await save(page); await preview(page); const saved = await get(page, `/api${path}`); expect(saved.head.payload.data.source.material_id).toBe(next.id); expect(saved.head.payload.data.stock_effect.selections[0].observation_version_id).toBe(nextStock.observations[0].version_id); expect(saved.head.payload.data.stock_effect.selections[0].included_quantity).toBe('1'); await finish(page)
})
