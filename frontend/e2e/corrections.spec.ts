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
    const timer = setTimeout(() => reject(new Error(`correction fixture start timed out: ${errors}`)), 15_000)
    server.stderr!.on('data', data => { errors += String(data) })
    server.once('exit', code => { clearTimeout(timer); reject(new Error(`correction fixture exited ${code}: ${errors}`)) })
    server.stdout!.on('data', data => { output += String(data); const match = output.match(/http:\/\/127\.0\.0\.1:\d+\/#token=[a-f0-9]+/); if (match) { clearTimeout(timer); resolveUrl(match[0]) } })
  })
}
test.beforeEach(async () => { directory = mkdtempSync(join(tmpdir(), 'sunchang-corrections-')); binary = join(directory, binaryName); cpSync(resolve('..', 'target', 'release', binaryName), binary); await launch() })
test.afterEach(async () => { if (server && server.exitCode === null && server.signalCode === null) server.kill(); if (closed) await closed; if (directory) { expect(resolve(directory).startsWith(resolve(tmpdir()))).toBe(true); rmSync(directory, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 }) } })
test.setTimeout(120_000)
async function open(page: Page) { await page.goto(launchUrl); await expect(page.getByLabel('재료 이름', { exact: false })).toBeEnabled() }
async function visit(page: Page, path: string) { await page.goto(new URL(path, launchUrl).toString()) }
async function get(page: Page, path: string) { const response = await page.request.get(new URL(path, launchUrl).toString()); expect(response.ok(), await response.text()).toBe(true); return response.json() }
async function post(page: Page, path: string, data: unknown) { const response = await page.request.post(new URL(path, launchUrl).toString(), { headers: { Origin: new URL(launchUrl).origin }, data }); expect(response.status(), await response.text()).toBe(201); return response.json() }
async function meta(page: Page) { const state = await get(page, '/api/state'); return { request_id: randomUUID(), epoch: state.epoch, expected_revision: state.revision } }
async function material(page: Page, name: string, unit = 'g') { return post(page, '/api/materials', { ...await meta(page), name, unit, kind: 'raw', description: '' }) }
async function draft(page: Page, kind: string, data: unknown) { return post(page, kind === 'correction' ? '/api/corrections/drafts' : kind === 'stocktake' ? '/api/stocktake-drafts' : kind === 'production' ? '/api/production-drafts' : '/api/business/drafts', { meta: await meta(page), payload: { kind, version: 1, data }, corrects_confirmation_id: null }) }
async function apiConfirm(page: Page, value: any) { return post(page, `/api/business/${value.result.item.id}/confirm`, { meta: await meta(page), expected_head_id: value.result.head.id }) }
async function observe(page: Page, item: any, quantity: string, initialValue?: string) {
  const saved = await apiConfirm(page, await draft(page, 'stocktake', { rows: [{ material_id: item.id, material_version_id: item.version_id, quantity, unit: item.unit, initial_value: initialValue === undefined ? null : { kind: 'manual_amount', amount: initialValue }, excess: null }], note: '' }))
  return get(page, `/api/stocktakes/${saved.result.confirmation.result.stocktake_id}`)
}
async function ordinaryReceipt(page: Page, item: any, quantity: string, amount: string) { return apiConfirm(page, await draft(page, 'receipt', { source: { kind: 'direct', material_id: item.id, material_version_id: item.version_id }, quantity, unit: item.unit, amount, receipt_date: '2024-02-29', note: '', stock_effect: { kind: 'unreflected' } })) }
async function waste(page: Page, item: any, quantity: string) { return apiConfirm(page, await draft(page, 'waste', { material_id: item.id, material_version_id: item.version_id, quantity, unit: item.unit, reason: 'discard', note: '' })) }
async function restart(page: Page) { await page.goto('about:blank'); let timer: ReturnType<typeof setTimeout> | undefined; const [code, signal] = await Promise.race([closed, new Promise<never>((_, reject) => { timer = setTimeout(() => reject(new Error('reconciliation normal shutdown timed out')), 180_000) })]).finally(() => clearTimeout(timer)); expect(code).toBe(0); expect(signal).toBeNull(); await launch(); await open(page) }

async function start(page: Page, confirmation: string) { await visit(page, `/business?kind=correction&corrects=${confirmation}`); await expect(page.getByRole('heading', { name: '정정 초안', exact: true })).toBeVisible() }
async function openSaved(page: Page) { const link = page.getByRole('link', { name: '저장한 초안 열기', exact: true }); await expect(link).toBeVisible(); const path = (await link.getAttribute('href'))!; await link.click(); await expect(page).toHaveURL(new URL(path, launchUrl).toString()); await expect(page.getByRole('button', { name: '확정 전 미리보기', exact: true })).toBeVisible(); return path }
async function save(page: Page) { await page.getByRole('button', { name: '초안 임시 저장', exact: true }).click(); return openSaved(page) }
async function revise(page: Page) { await page.getByRole('button', { name: '수정 내용 임시 저장', exact: true }).click(); await expect(page.getByText('수정 내용을 임시 저장했습니다.', { exact: true })).toBeVisible(); await expect(page.getByRole('button', { name: '확정 전 미리보기', exact: true })).toBeEnabled() }
async function preview(page: Page) { await page.getByRole('button', { name: '확정 전 미리보기', exact: true }).click(); await expect(page.getByRole('heading', { name: '정정 영향 미리보기', exact: true })).toBeVisible(); await expect(page.getByRole('button', { name: '이 내용으로 확정', exact: true })).toBeEnabled() }
async function finish(page: Page) { await page.getByRole('button', { name: '이 내용으로 확정', exact: true }).click(); const link = page.getByRole('link', { name: '정정 영향과 확정 이력 보기', exact: true }); await expect(link).toBeVisible(); const path = (await link.getAttribute('href'))!; await link.click(); await expect(page).toHaveURL(new URL(path, launchUrl).toString()); await expect(page.getByRole('heading', { name: '정정 확정 기록', exact: true })).toBeVisible(); return get(page, `/api${path}`) }
async function recipe(page: Page, item: any) { return post(page, '/api/recipes', { meta: await meta(page), owner: { mode: 'new', name: '가상 판매 배분 메뉴', unit: '개', kind: 'menu', description: '' }, data: { basis_count: '1', reference_output: null, selling_price: '1000', cooking_description: '', process: '', reference_time: '', lines: [{ material_id: item.id, material_version_id: item.version_id, quantity: '0', unit: item.unit, note: '' }] } }) }
async function sale(page: Page, menu: any, count: string) { return apiConfirm(page, await draft(page, 'sale', { material_id: menu.item.id, material_version_id: menu.item.version_id, recipe_version_id: menu.version.id, count, note: '' })) }
async function correctApi(page: Page, confirmation: string, change: (data: any) => void) { const seed = await get(page, `/api/business/confirmations/${confirmation}/correction-seed`); const data = seed.draft; data.reason = '가상 다른 화면 정정'; change(data); const saved = await post(page, '/api/corrections/drafts', { meta: await meta(page), payload: { kind: 'correction', version: 1, data }, corrects_confirmation_id: seed.original.reference.original_confirmation_id }); return apiConfirm(page, saved) }

test('settled sale correction survives normal restart and reallocates counts without changing consumption or stock', async ({ page }) => {
  test.setTimeout(260_000); await open(page)
  const raw = await material(page, '가상 정정 배분 재료'); await observe(page, raw, '10', '10')
  const menu = await recipe(page, raw), first = await sale(page, menu, '3'), second = await sale(page, menu, '1'); const ending = await observe(page, raw, '6')
  const before = await get(page, `/api/holdings/${raw.id}`), original = await get(page, `/api/sales/${first.result.confirmation.result.sale_id}`)
  await visit(page, `/sales/${original.item.id}`); await page.getByRole('link', { name: '이 기록 정정하기', exact: true }).click(); await expect(page.getByRole('heading', { name: '정정 초안', exact: true })).toBeVisible()
  await page.getByLabel('정정할 판매 개수', { exact: true }).fill('1.'); const path = await save(page)
  await restart(page); await visit(page, path); await expect(page.getByLabel('정정할 판매 개수', { exact: true })).toHaveValue('1.')
  await page.getByLabel('정정할 판매 개수', { exact: true }).fill('1'); await page.getByLabel('정정 이유', { exact: true }).fill('가상 개수 오입력 정정'); await revise(page); await preview(page)
  await expect(page.locator('.correction-preview')).toContainText('판매 개수에 따른 실사 소모 배분'); const result = await finish(page)
  const allocation = result.item.plan.allocations.find((entry: any) => entry.after?.candidates.some((row: any) => row.sale_id === original.item.id)).after
  expect(allocation.total_quantity).toBe('4'); expect(allocation.total_value).toBe('4'); expect(allocation.candidates.map((row: any) => row.quantity)).toEqual(['2', '2'])
  const after = await get(page, `/api/holdings/${raw.id}`); expect(after.settlement_basis.quantity).toBe(before.settlement_basis.quantity); expect(after.settlement_basis.value).toBe(before.settlement_basis.value)
  expect((await get(page, `/api/sales/${original.item.id}`)).item).toEqual(original.item)
  await correctApi(page, first.result.confirmation.id, data => { data.action.replacement.data.count = '0' }); await correctApi(page, second.result.confirmation.id, data => { data.action.replacement.data.count = '0' })
  const effective = await get(page, `/api/sales/${original.item.id}`); expect(effective.consumption[0].quantity).toBeNull(); expect(effective.consumption[0].reason).toBe('zero_weight'); expect(ending.observations[0].effect.measured_quantity).toBe('6')
  await start(page, first.result.confirmation.id); await page.getByLabel('정정 방법', { exact: true }).selectOption('void'); await page.getByLabel('정정 이유', { exact: true }).fill('가상 중복 판매 취소'); await save(page); await preview(page); await finish(page)
  await visit(page, `/sales/${original.item.id}`); await expect(page.getByRole('heading', { name: '보존된 판매 기록', exact: true })).toBeVisible(); await expect(page.locator('.sale-detail')).toContainText('현재 판매로 합산하지 않습니다')
  await visit(page, '/sales'); await expect(page.locator('.sale-list > li').filter({ has: page.locator(`a[href="/sales/${original.item.id}"]`) })).toContainText('취소됨 · 원기록 보존')
})

test('earlier observation correction preserves later measured quantity and the next interval boundary', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 앞선 실사 정정 — 원본 수량과 현재 유효한 소모를 구분하는 긴 이름'); await observe(page, raw, '10', '10'); const middle = await observe(page, raw, '6'); const ending = await observe(page, raw, '4')
  await start(page, middle.item.confirmation_id); await page.getByLabel('정정할 확인 수량 1', { exact: true }).fill('7'); await page.getByLabel('정정 이유', { exact: true }).fill('가상 앞선 실측 오입력'); await save(page); await preview(page)
  for (const [width, height] of [[1440, 900], [768, 1024], [390, 844]] as const) { await page.setViewportSize({ width, height }); for (const theme of ['light', 'dark'] as const) { if (await page.locator('html').getAttribute('data-theme') !== theme) await page.getByRole('button', { name: theme === 'dark' ? '다크 모드로 전환' : '라이트 모드로 전환' }).click(); expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true); await page.screenshot({ path: test.info().outputPath(`correction-${width}-${theme}.png`), fullPage: true }) } }
  const result = await finish(page), rows = result.item.plan.observations.map((item: any) => item.after).filter(Boolean)
  expect(rows.find((row: any) => row.measured_quantity === '7').calculation.consumption_quantity).toBe('3'); expect(rows.find((row: any) => row.measured_quantity === '4').calculation.consumption_quantity).toBe('3')
  expect((await get(page, `/api/holdings/${raw.id}`)).settlement_basis.quantity).toBe('4'); expect((await get(page, `/api/stocktakes/${ending.item.id}`)).observations[0].effect.measured_quantity).toBe('4')
  const next = await observe(page, raw, '3'); expect(next.observations[0].effect.calculation.consumption_quantity).toBe('1')
  const restoreMaterial = await material(page, '가상 전체 실사 취소 후 명시 복원'), restoreStock = await observe(page, restoreMaterial, '10', '10')
  await start(page, restoreStock.item.confirmation_id); await page.getByLabel('정정 방법', { exact: true }).selectOption('void'); await page.getByLabel('정정 이유', { exact: true }).fill('가상 전체 실사 취소'); await save(page); await preview(page); await finish(page)
  expect((await get(page, `/api/holdings/${restoreMaterial.id}`)).settlement_basis).toBeNull()
  const voidSeed = await get(page, `/api/business/confirmations/${restoreStock.item.confirmation_id}/correction-seed`)
  expect(voidSeed.current.input).toBeNull(); expect(voidSeed.current.value).toBeNull()
  await start(page, restoreStock.item.confirmation_id); await expect(page.getByLabel('정정 방법', { exact: true })).toHaveValue('void'); await page.getByLabel('정정 방법', { exact: true }).selectOption('replace')
  await expect(page.getByLabel('정정할 확인 수량 1', { exact: true })).toHaveValue('10'); await page.getByRole('button', { name: '이 행은 정정하지 않고 유지', exact: true }).click(); await page.getByRole('button', { name: '제외한 원래 실사 행 다시 선택', exact: true }).click()
  await page.getByLabel('정정할 확인 수량 1', { exact: true }).fill('5'); await page.getByLabel('시작 가치 총금액 (원)', { exact: true }).fill('5'); await page.getByLabel('정정 이유', { exact: true }).fill('가상 전체 취소 후 실제 확인 수량 복원'); await save(page); await preview(page); const restored = await finish(page)
  expect(restored.item.input.action.replacement.data.rows[0].expected_version_id).toBe(voidSeed.replacement_template.data.rows[0].expected_version_id)
  const restoredHolding = await get(page, `/api/holdings/${restoreMaterial.id}`); expect(restoredHolding.settlement_basis.quantity).toBe('5'); expect(restoredHolding.settlement_basis.value).toBe('5')
  expect((await get(page, `/api/business/confirmations/${restoreStock.item.confirmation_id}/correction-seed`)).original).toEqual(voidSeed.original)
})

test('receipt replacement and void retain originals and distinguish absent basis from zero', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 시작 입고 취소'); const receipt = await ordinaryReceipt(page, raw, '10', '20'); const original = await get(page, `/api/receipts/${receipt.result.confirmation.result.receipt_id}`)
  await start(page, receipt.result.confirmation.id); await page.getByLabel('실제 입고 수량', { exact: true }).fill('12'); await page.getByLabel('실제 입고 금액 (원)', { exact: true }).fill('24'); await page.getByLabel('정정 이유', { exact: true }).fill('가상 입고 수량 정정'); await save(page); await preview(page); await finish(page)
  expect((await get(page, `/api/holdings/${raw.id}`)).settlement_basis.quantity).toBe('12')
  await start(page, receipt.result.confirmation.id); await page.getByLabel('정정 방법', { exact: true }).selectOption('void'); await page.getByLabel('정정 이유', { exact: true }).fill('가상 중복 입고 기록 취소'); await save(page); await preview(page); await expect(page.locator('.correction-preview')).toContainText('정산 기준 없음'); const result = await finish(page)
  expect(result.item.plan.materials[0].after.kind).toBe('not_established'); expect((await get(page, `/api/holdings/${raw.id}`)).settlement_basis).toBeNull(); expect((await get(page, `/api/receipts/${original.item.id}`)).item).toEqual(original.item)
})

test('production correction changes actual input and keeps explicit zero-percent output', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 생산 투입'), output = await material(page, '가상 생산 결과', 'mL'), byproduct = await material(page, '가상 0퍼센트 결과', '개'); await ordinaryReceipt(page, raw, '10', '10')
  const production = await apiConfirm(page, await draft(page, 'production', { inputs: [{ material_id: raw.id, material_version_id: raw.version_id, quantity: '2', unit: 'g' }], outputs: [{ material_id: output.id, material_version_id: output.version_id, quantity: '1', unit: 'mL', allocation_percent: '100' }, { material_id: byproduct.id, material_version_id: byproduct.version_id, quantity: '1', unit: '개', allocation_percent: '0' }], note: '' }))
  await start(page, production.result.confirmation.id); await page.locator('#correction-replacement-inputs-0-quantity').fill('3'); await page.getByLabel('정정 이유', { exact: true }).fill('가상 투입 수량 정정'); await save(page); await preview(page); const result = await finish(page)
  const after = result.item.plan.events.find((item: any) => item.reference.kind === 'production').after.result; expect(after.total_input_value).toBe('3'); expect(after.outputs[1].allocation_percent).toBe('0'); expect(after.outputs[1].value).toBe('0'); expect((await get(page, `/api/holdings/${raw.id}`)).settlement_basis.quantity).toBe('7')
})

test('waste replacement and cancellation preserve quantity semantics without creating a return', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 폐기 정정'); await ordinaryReceipt(page, raw, '10', '10'); const original = await waste(page, raw, '2')
  await start(page, original.result.confirmation.id); await page.getByLabel('실제 폐기 · 직원식 수량', { exact: true }).fill('4'); await page.getByLabel('정정 이유', { exact: true }).fill('가상 폐기 오입력'); await save(page); await preview(page); await finish(page); expect((await get(page, `/api/holdings/${raw.id}`)).settlement_basis.quantity).toBe('6')
  await start(page, original.result.confirmation.id); await page.getByLabel('정정 방법', { exact: true }).selectOption('void'); await page.getByLabel('정정 이유', { exact: true }).fill('가상 중복 폐기 취소'); await save(page); await preview(page); const result = await finish(page); expect(result.item.plan.events[0].after).toBeNull(); expect((await get(page, `/api/holdings/${raw.id}`)).settlement_basis.quantity).toBe('10'); expect((await get(page, '/api/receipts')).items).toHaveLength(1)
})

test('shrinking observation requires an explicit linked receipt revision with proposed candidates', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 포함 근거 축소'); const stock = await observe(page, raw, '10', '10')
  const receipt = await apiConfirm(page, await draft(page, 'receipt', { source: { kind: 'direct', material_id: raw.id, material_version_id: raw.version_id }, quantity: '3', unit: 'g', amount: '6', receipt_date: '2024-02-29', note: '', stock_effect: { kind: 'reconciled', selections: [{ observation_version_id: stock.observations[0].version_id, source: { kind: 'all_available' }, included_quantity: '3' }] } }))
  await start(page, stock.item.confirmation_id); await page.getByLabel('정정할 확인 수량 1', { exact: true }).fill('2'); await page.getByLabel('정정 이유', { exact: true }).fill('가상 실사 근거 축소'); await save(page); await page.getByRole('button', { name: '확정 전 미리보기', exact: true }).click(); await expect(page.getByRole('button', { name: '이 내용으로 확정', exact: true })).toBeDisabled()
  await page.getByRole('button', { name: '이 입고의 연결도 함께 정정', exact: true }).click(); await expect(page.locator('#correction-linked-0-included-0-quantity')).toHaveValue('3'); await page.locator('#correction-linked-0-included-0-quantity').fill('2'); await revise(page); await preview(page); const result = await finish(page)
  expect(result.item.input.linked_receipt_changes[0].receipt_id).toBe(receipt.result.confirmation.result.receipt_id); expect(result.item.plan.inclusions[0].after.included_quantity).toBe('2'); expect(result.item.resolved_refs.length).toBeGreaterThan(0)
})

test('lost correction requests replay unchanged while a changed target needs explicit version review', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 정정 요청 보존'); const receipt = await ordinaryReceipt(page, raw, '10', '10'); await start(page, receipt.result.confirmation.id); await page.getByLabel('실제 입고 수량', { exact: true }).fill('12'); await page.getByLabel('정정 이유', { exact: true }).fill('가상 요청 원문 보존')
  const creates: any[] = []; await page.route('**/api/corrections/drafts', async route => { creates.push(route.request().postDataJSON()); if (creates.length === 1) { await route.fetch(); await route.abort('failed') } else await route.continue() }); await page.getByRole('button', { name: '초안 임시 저장', exact: true }).click(); await expect(page.getByRole('button', { name: '같은 업무 요청 재시도', exact: true })).toBeVisible()
  await correctApi(page, receipt.result.confirmation.id, data => { data.action.replacement.data.quantity = '11' }); await page.reload(); await page.getByRole('button', { name: '같은 업무 요청 재시도', exact: true }).click(); const path = await openSaved(page); expect(creates[1]).toEqual(creates[0]); await expect(page.getByLabel('실제 입고 수량', { exact: true })).toHaveValue('12')
  await page.getByRole('button', { name: '확정 전 미리보기', exact: true }).click(); await expect(page.getByRole('button', { name: '이 내용으로 확정', exact: true })).toBeDisabled(); await page.getByRole('button', { name: '현재 대상 버전을 확인하고 입력 유지', exact: true }).click(); await revise(page); await preview(page)
  const confirmations: unknown[] = []; await page.route('**/api/business/*/confirm', async route => { confirmations.push(route.request().postDataJSON()); if (confirmations.length === 1) { await route.fetch(); await route.abort('failed') } else await route.continue() }); await page.getByRole('button', { name: '이 내용으로 확정', exact: true }).click(); await expect(page.getByRole('button', { name: '같은 업무 요청 재시도', exact: true })).toBeVisible(); await page.reload(); await page.getByRole('button', { name: '같은 업무 요청 재시도', exact: true }).click(); await expect(page.getByRole('link', { name: '정정 영향과 확정 이력 보기', exact: true })).toBeVisible(); expect(confirmations[1]).toEqual(confirmations[0]); expect((await get(page, `/api${path}`)).confirmations).toHaveLength(1)
})
