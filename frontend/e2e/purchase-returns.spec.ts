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
    const timer = setTimeout(() => reject(new Error(`return fixture start timed out: ${errors}`)), 15_000)
    server.stderr!.on('data', data => { errors += String(data) })
    server.once('exit', code => { clearTimeout(timer); reject(new Error(`return fixture exited ${code}: ${errors}`)) })
    server.stdout!.on('data', data => { output += String(data); const match = output.match(/http:\/\/127\.0\.0\.1:\d+\/#token=[a-f0-9]+/); if (match) { clearTimeout(timer); resolveUrl(match[0]) } })
  })
}
test.beforeEach(async () => { directory = mkdtempSync(join(tmpdir(), 'sunchang-returns-')); binary = join(directory, binaryName); cpSync(resolve('..', 'target', 'release', binaryName), binary); await launch() })
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
async function restart(page: Page) { await page.goto('about:blank'); let timer: ReturnType<typeof setTimeout> | undefined; const [code, signal] = await Promise.race([closed, new Promise<never>((_, reject) => { timer = setTimeout(() => reject(new Error('return normal shutdown timed out')), 180_000) })]).finally(() => clearTimeout(timer)); expect(code).toBe(0); expect(signal).toBeNull(); await launch(); await open(page) }

async function returnData(page: Page, receiptId: string, quantity: string, unit?: string) { const source = await get(page, `/api/receipts/${receiptId}/return-source`); return { receipt_id: receiptId, expected_receipt_version_id: source.current.reference.version_id, quantity, unit: unit ?? source.current.unit, note: '' } }
async function apiReturn(page: Page, receiptId: string, quantity: string, unit?: string) { const saved = await apiConfirm(page, await draft(page, 'purchase_return', await returnData(page, receiptId, quantity, unit))); return get(page, `/api/purchase-returns/${saved.result.confirmation.result.purchase_return_id}`) }
async function start(page: Page, receiptId?: string) { await visit(page, `/business?kind=purchase_return${receiptId ? `&receipt_id=${receiptId}` : ''}`); await expect(page.getByRole('heading', { name: '구입 반품 초안', exact: true })).toBeVisible() }
async function openSaved(page: Page) { const link = page.getByRole('link', { name: '저장한 초안 열기', exact: true }); await expect(link).toBeVisible(); const path = (await link.getAttribute('href'))!; await link.click(); await expect(page).toHaveURL(new URL(path, launchUrl).toString()); await expect(page.getByRole('button', { name: '확정 전 미리보기', exact: true })).toBeVisible(); return path }
async function save(page: Page) { await page.getByRole('button', { name: '초안 임시 저장', exact: true }).click(); return openSaved(page) }
async function revise(page: Page) { await page.getByRole('button', { name: '수정 내용 임시 저장', exact: true }).click(); await expect(page.getByText('수정 내용을 임시 저장했습니다.', { exact: true })).toBeVisible(); await expect(page.getByRole('button', { name: '확정 전 미리보기', exact: true })).toBeEnabled() }
async function preview(page: Page, ready = true) { await page.getByRole('button', { name: '확정 전 미리보기', exact: true }).click(); await expect(page.getByRole('heading', { name: '구입 반품 미리보기', exact: true })).toBeVisible(); if (ready) await expect(page.getByRole('button', { name: '이 내용으로 확정', exact: true })).toBeEnabled(); else await expect(page.getByRole('button', { name: '이 내용으로 확정', exact: true })).toBeDisabled() }
async function finish(page: Page) { await page.getByRole('button', { name: '이 내용으로 확정', exact: true }).click(); const link = page.getByRole('link', { name: '구입 반품 기록 보기', exact: true }); await expect(link).toBeVisible(); const path = (await link.getAttribute('href'))!; await link.click(); await expect(page).toHaveURL(new URL(path, launchUrl).toString()); await expect(page.getByRole('heading', { name: '현재 유효한 구입 반품', exact: true })).toBeVisible(); return get(page, `/api${path}`) }
async function correctApi(page: Page, confirmation: string, change: (data: any, seed: any) => void) { const seed = await get(page, `/api/business/confirmations/${confirmation}/correction-seed`); const data = seed.draft; data.reason = '가상 연결 정정'; change(data, seed); const saved = await post(page, '/api/corrections/drafts', { meta: await meta(page), payload: { kind: 'correction', version: 1, data }, corrects_confirmation_id: seed.original.reference.original_confirmation_id }); return apiConfirm(page, saved) }
async function correctionStart(page: Page, confirmation: string) { await visit(page, `/business?kind=correction&corrects=${confirmation}`); await expect(page.getByRole('heading', { name: '정정 초안', exact: true })).toBeVisible() }
async function correctionFinish(page: Page) { await page.getByRole('button', { name: '확정 전 미리보기', exact: true }).click(); await expect(page.getByRole('button', { name: '이 내용으로 확정', exact: true })).toBeEnabled(); await page.getByRole('button', { name: '이 내용으로 확정', exact: true }).click(); const link = page.getByRole('link', { name: '정정 영향과 확정 이력 보기', exact: true }); await expect(link).toBeVisible(); const path = (await link.getAttribute('href'))!; await link.click(); await expect(page).toHaveURL(new URL(path, launchUrl).toString()); return get(page, `/api${path}`) }

test('return draft survives normal restart and separates original reference from actual average with six layouts', async ({ page }) => {
  test.setTimeout(260_000); await open(page)
  const raw = await material(page, '가상 구입 반품 — 원구입 참고 금액과 현재 평균 차감액을 구분하는 긴 품목 이름')
  const receipt = await ordinaryReceipt(page, raw, '3000', '30000'), receiptId = receipt.result.confirmation.result.receipt_id
  await apiReturn(page, receiptId, '1', 'kg')
  await visit(page, `/receipts/${receiptId}`); await page.getByRole('link', { name: '이 입고 반품 초안 작성', exact: true }).click()
  await expect(page.getByLabel('반품 단위', { exact: true })).toHaveValue('g')
  await page.getByLabel('실제 반품 수량', { exact: true }).fill('1.'); await page.getByLabel('반품 단위', { exact: true }).fill('kg'); const path = await save(page)
  await restart(page); await visit(page, path); await expect(page.getByLabel('실제 반품 수량', { exact: true })).toHaveValue('1.')
  await page.getByLabel('실제 반품 수량', { exact: true }).fill('1'); await page.getByLabel('반품 메모 (선택)', { exact: true }).fill('가상 포장 손상 반품\n실제 수량 확인'); await revise(page); await preview(page)
  await expect(page.locator('.purchase-return-preview')).toContainText('30,000원'); await expect(page.locator('.purchase-return-preview')).toContainText('10,000원')
  const result = await finish(page); expect(result.item.effect.deducted_value).toBe('10000'); expect(result.item.effect.source.original_amount).toBe('30000'); expect(result.item.effect.after.quantity).toBe('1000'); expect(result.item.effect.after.value).toBe('10000')
  for (const [width, height] of [[1440, 900], [768, 1024], [390, 844]] as const) { await page.setViewportSize({ width, height }); for (const theme of ['light', 'dark'] as const) { if (await page.locator('html').getAttribute('data-theme') !== theme) await page.getByRole('button', { name: theme === 'dark' ? '다크 모드로 전환' : '라이트 모드로 전환' }).click(); expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true); await page.screenshot({ path: test.info().outputPath(`purchase-return-${width}-${theme}.png`), fullPage: true }) } }
  await visit(page, `/purchase-returns?receipt=${receiptId}`); await expect(page.locator('.business-list > li')).toHaveCount(2)
  await visit(page, `/materials/${raw.id}`); await expect(page.locator('.holding-changes')).toContainText('구입 반품'); await expect(page.locator(`a[href="/purchase-returns/${result.item.id}"]`)).toBeVisible()
})

test('mixed average partial and full returns consume the exact residual with a fresh same-source write', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 반품 반올림 잔여'), first = await ordinaryReceipt(page, raw, '3000', '30000'); const receiptId = first.result.confirmation.result.receipt_id
  await ordinaryReceipt(page, raw, '1000', '10000'); await waste(page, raw, '1000')
  await start(page, receiptId); await page.getByLabel('실제 반품 수량', { exact: true }).fill('1'); await page.getByLabel('반품 단위', { exact: true }).fill('kg'); await save(page); await preview(page); const one = await finish(page)
  expect(one.item.effect.deducted_value).toBe('13333.333333'); expect(one.item.effect.after.value).toBe('26666.666667')
  await page.getByRole('link', { name: '같은 원입고의 새 반품 초안', exact: true }).click(); await expect(page.getByRole('button', { name: '새 구입 반품 초안 작성', exact: true })).toBeVisible(); await page.getByRole('button', { name: '새 구입 반품 초안 작성', exact: true }).click()
  await expect(page.getByLabel('실제 반품 수량', { exact: true })).toHaveValue(''); await page.getByLabel('실제 반품 수량', { exact: true }).fill('2'); await page.getByLabel('반품 단위', { exact: true }).fill('kg'); await save(page); await preview(page); const two = await finish(page)
  expect(two.item.id).not.toBe(one.item.id); expect(two.item.effect.deducted_value).toBe('26666.666667'); expect(two.item.effect.after.quantity).toBe('0'); expect(two.item.effect.after.value).toBe('0')
  const holding = await get(page, `/api/holdings/${raw.id}`); expect(holding.last_valid_price).not.toBeNull(); expect((await get(page, `/api/receipts/${receiptId}/return-source`)).current.remaining_quantity).toBe('0')
})

test('fully included receipt uses actual R limit and preserves an excessive second return', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 포함 입고 반품 한도'), stock = await observe(page, raw, '10', '10')
  const receipt = await apiConfirm(page, await draft(page, 'receipt', { source: { kind: 'direct', material_id: raw.id, material_version_id: raw.version_id }, quantity: '3', unit: 'g', amount: '3', receipt_date: '2024-02-29', note: '', stock_effect: { kind: 'reconciled', selections: [{ observation_version_id: stock.observations[0].version_id, source: { kind: 'all_available' }, included_quantity: '3' }] } })), receiptId = receipt.result.confirmation.result.receipt_id
  expect((await get(page, `/api/receipts/${receiptId}`)).reconciliation.effect.unreflected_quantity).toBe('0')
  await start(page, receiptId); await page.getByLabel('실제 반품 수량', { exact: true }).fill('2'); await save(page); await preview(page); await finish(page)
  await start(page); await page.getByLabel('반품할 원입고', { exact: true }).selectOption(receiptId); await expect(page.getByLabel('반품 단위', { exact: true })).toHaveValue('g'); await page.getByLabel('실제 반품 수량', { exact: true }).fill('2'); await save(page); await preview(page, false)
  await expect(page.getByLabel('실제 반품 수량', { exact: true })).toHaveValue('2'); await expect(page.locator('.business-issues')).not.toBeEmpty()
  expect((await get(page, `/api/holdings/${raw.id}`)).settlement_basis.quantity).toBe('8'); expect((await get(page, `/api/receipts/${receiptId}/return-source`)).current.remaining_quantity).toBe('1')
})

test('known zero value is valid and dimension precision errors stay editable before the next observation', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 무상 입고 반품'), receipt = await ordinaryReceipt(page, raw, '10', '0'), receiptId = receipt.result.confirmation.result.receipt_id
  await observe(page, raw, '10'); await start(page, receiptId); await page.getByLabel('실제 반품 수량', { exact: true }).fill('0.0000001'); const path = await save(page); await preview(page, false)
  await expect(page.getByLabel('실제 반품 수량', { exact: true })).toHaveValue('0.0000001'); await page.getByLabel('실제 반품 수량', { exact: true }).fill('2'); await page.getByLabel('반품 단위', { exact: true }).fill('mL'); await revise(page); await preview(page, false)
  await expect(page.getByLabel('반품 단위', { exact: true })).toHaveValue('mL'); await page.getByLabel('반품 단위', { exact: true }).fill('g'); await revise(page); await preview(page); const returned = await finish(page)
  expect(returned.item.effect.deducted_value).toBe('0'); expect(returned.item.effect.after.quantity).toBe('8'); expect(returned.item.effect.after.value).toBe('0')
  const ending = await observe(page, raw, '6'); expect(ending.observations[0].effect.calculation.consumption_quantity).toBe('2'); expect(ending.observations[0].effect.calculation.consumption_value).toBe('0')
  const excessive = await draft(page, 'purchase_return', await returnData(page, receiptId, '7')); await visit(page, `/business/${excessive.result.item.id}`); await preview(page, false); await expect(page.getByLabel('실제 반품 수량', { exact: true })).toHaveValue('7'); expect((await get(page, `/api${path}`)).confirmations).toHaveLength(1)
})

test('return replace void and source correction restoration preserve originals and use current source template', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 반품 복원'), receipt = await ordinaryReceipt(page, raw, '10', '20'), receiptId = receipt.result.confirmation.result.receipt_id; const returned = await apiReturn(page, receiptId, '2')
  await correctionStart(page, returned.item.confirmation_id); await page.getByLabel('실제 반품 수량', { exact: true }).fill('1'); await page.getByLabel('정정 이유', { exact: true }).fill('가상 반환 수량 수정'); await save(page); await correctionFinish(page)
  expect((await get(page, `/api/purchase-returns/${returned.item.id}`)).effective.effect.quantity).toBe('1')
  const constrained = await get(page, `/api/business/confirmations/${receipt.result.confirmation.id}/correction-seed`); constrained.draft.reason = '가상 입고 취소 충돌'; constrained.draft.action = { kind: 'void' }; const conflict = await post(page, '/api/corrections/drafts', { meta: await meta(page), payload: { kind: 'correction', version: 1, data: constrained.draft }, corrects_confirmation_id: receipt.result.confirmation.id })
  await visit(page, `/business/${conflict.result.item.id}`); await page.getByRole('button', { name: '확정 전 미리보기', exact: true }).click(); await expect(page.getByRole('button', { name: '이 내용으로 확정', exact: true })).toBeDisabled(); await expect(page.locator('.business-issues')).toContainText(returned.item.id)
  await correctApi(page, returned.item.confirmation_id, data => { data.action = { kind: 'void' } })
  await visit(page, `/purchase-returns/${returned.item.id}`); await expect(page.getByText('이 반품의 유효 효과가 취소되었습니다.', { exact: false })).toBeVisible(); expect((await get(page, `/api/purchase-returns/${returned.item.id}`)).item).toEqual(returned.item)
  await correctApi(page, receipt.result.confirmation.id, data => { data.action = { kind: 'void' } })
  await correctionStart(page, returned.item.confirmation_id); await page.getByLabel('정정 방법', { exact: true }).selectOption('replace'); await expect(page.getByText('이 원입고의 유효 효과가 취소되었습니다.', { exact: false })).toBeVisible(); await expect(page.getByLabel('실제 반품 수량', { exact: true })).toHaveValue('2')
  await correctApi(page, receipt.result.confirmation.id, (data, seed) => { data.action = { kind: 'replace', replacement: seed.replacement_template }; data.action.replacement.data.quantity = '12'; data.action.replacement.data.amount = '24' })
  await page.reload(); await page.getByLabel('정정 방법', { exact: true }).selectOption('replace'); await page.getByRole('button', { name: '현재 원입고 정보 적용', exact: true }).click(); await expect(page.locator('.return-source-context')).toContainText('12g'); await page.getByLabel('정정 이유', { exact: true }).fill('가상 입고 개정 후 반품 명시 복원')
  const staleResponse = page.waitForResponse(response => response.url().endsWith('/api/corrections/drafts') && response.request().method() === 'POST')
  await page.getByRole('button', { name: '초안 임시 저장', exact: true }).click()
  const stale = await staleResponse; expect(stale.status()).toBe(409); expect((await stale.json()).code).toBe('stale_state'); const staleBody = stale.request().postDataJSON()
  await page.getByRole('button', { name: '최신 기록 확인 후 입력 검토', exact: true }).click(); await expect(page.getByLabel('실제 반품 수량', { exact: true })).toBeEnabled(); await expect(page.getByLabel('실제 반품 수량', { exact: true })).toHaveValue('2')
  const acceptedResponse = page.waitForResponse(response => response.url().endsWith('/api/corrections/drafts') && response.request().method() === 'POST')
  await save(page); const accepted = await acceptedResponse; expect(accepted.status()).toBe(201); const acceptedBody = accepted.request().postDataJSON()
  expect(acceptedBody.payload).toEqual(staleBody.payload); expect(acceptedBody.meta.request_id).not.toBe(staleBody.meta.request_id); expect(acceptedBody.meta.expected_revision).not.toBe(staleBody.meta.expected_revision)
  await correctionFinish(page)
  const restored = await get(page, `/api/purchase-returns/${returned.item.id}`); expect(restored.item).toEqual(returned.item); expect(restored.effective.kind).toBe('active'); expect(restored.effective.effect.quantity).toBe('2'); expect(restored.effective.effect.source.reference.version_id).toBe((await get(page, `/api/receipts/${receiptId}/return-source`)).current.reference.version_id)
})

test('uncertain return create and confirmation replay original requests after source changes', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 반품 요청 재생'), receipt = await ordinaryReceipt(page, raw, '10', '10'), receiptId = receipt.result.confirmation.result.receipt_id
  await start(page, receiptId); await page.getByLabel('실제 반품 수량', { exact: true }).fill('2')
  const creates: any[] = []; await page.route('**/api/business/drafts', async route => { creates.push(route.request().postDataJSON()); if (creates.length === 1) { await route.fetch(); await route.abort('failed') } else await route.continue() }); await page.getByRole('button', { name: '초안 임시 저장', exact: true }).click(); await expect(page.getByRole('button', { name: '같은 업무 요청 재시도', exact: true })).toBeVisible()
  await correctApi(page, receipt.result.confirmation.id, data => { data.action = { kind: 'void' } }); await page.reload(); await page.getByRole('button', { name: '같은 업무 요청 재시도', exact: true }).click(); const path = await openSaved(page); expect(creates[1]).toEqual(creates[0]); await preview(page, false)
  await correctApi(page, receipt.result.confirmation.id, (data, seed) => { data.action = { kind: 'replace', replacement: seed.replacement_template } }); await page.reload(); await page.getByRole('button', { name: '현재 원입고 정보 적용', exact: true }).click(); await expect(page.locator('.draft-note')).toBeVisible(); await revise(page); await preview(page)
  const confirmations: unknown[] = []; await page.route('**/api/business/*/confirm', async route => { confirmations.push(route.request().postDataJSON()); if (confirmations.length === 1) { await route.fetch(); await route.abort('failed') } else await route.continue() }); await page.getByRole('button', { name: '이 내용으로 확정', exact: true }).click(); await expect(page.getByRole('button', { name: '같은 업무 요청 재시도', exact: true })).toBeVisible(); await page.reload(); await page.getByRole('button', { name: '같은 업무 요청 재시도', exact: true }).click(); await expect(page.getByRole('link', { name: '구입 반품 기록 보기', exact: true })).toBeVisible(); expect(confirmations[1]).toEqual(confirmations[0]); expect((await get(page, `/api${path}`)).confirmations).toHaveLength(1); expect((await get(page, `/api/holdings/${raw.id}`)).settlement_basis.quantity).toBe('8')
})

test('late source responses preserve another selected receipt and a sent draft', async ({ page }) => {
  await open(page); const first = await material(page, '가상 늦은 입고 A'), second = await material(page, '가상 현재 입고 B', '개'), a = await ordinaryReceipt(page, first, '10', '10'), b = await ordinaryReceipt(page, second, '8', '16'), aId = a.result.confirmation.result.receipt_id, bId = b.result.confirmation.result.receipt_id
  await start(page); let release!: () => void; const gate = new Promise<void>(resolve => { release = resolve }); let arrived = 0; const completions: Promise<void>[] = []
  await page.route(`**/api/receipts/${aId}/return-source`, route => { const completing = (async () => { const response = await route.fetch(); arrived += 1; await gate; await route.fulfill({ response }) })(); completions.push(completing); return completing })
  await page.getByLabel('반품할 원입고', { exact: true }).selectOption(aId); await expect.poll(() => arrived).toBeGreaterThan(0); await page.getByLabel('반품할 원입고', { exact: true }).selectOption(bId); await expect(page.getByLabel('반품 단위', { exact: true })).toHaveValue('개'); await page.getByLabel('실제 반품 수량', { exact: true }).fill('2'); await save(page); release(); await Promise.all(completions); await page.unroute(`**/api/receipts/${aId}/return-source`)
  await expect(page.getByLabel('반품할 원입고', { exact: true })).toHaveValue(bId); await expect(page.getByLabel('반품 단위', { exact: true })).toHaveValue('개'); await preview(page); const result = await finish(page); expect(result.item.effect.source.reference.id).toBe(bId); expect(result.item.effect.deducted_value).toBe('4')
})
