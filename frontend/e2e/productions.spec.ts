import { test, expect, type Page } from '@playwright/test'
import { spawn, type ChildProcess } from 'node:child_process'
import { once } from 'node:events'
import { randomUUID } from 'node:crypto'
import { cpSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

const binaryName = process.platform === 'win32' ? 'sunchang-hansang.exe' : 'sunchang-hansang'
let directory: string, binary: string, server: ChildProcess, closed: Promise<[number | null, NodeJS.Signals | null]>, launchUrl: string
async function launch() {
  server = spawn(binary, ['--no-browser'], { cwd: directory, stdio: ['ignore', 'pipe', 'pipe'] })
  closed = once(server, 'close') as Promise<[number | null, NodeJS.Signals | null]>
  launchUrl = await new Promise<string>((resolveUrl, reject) => {
    let output = '', errors = ''
    const timer = setTimeout(() => reject(new Error(`production fixture start timed out: ${errors}`)), 15_000)
    server.stderr!.on('data', data => { errors += String(data) })
    server.once('exit', code => { clearTimeout(timer); reject(new Error(`production fixture exited ${code}: ${errors}`)) })
    server.stdout!.on('data', data => { output += String(data); const match = output.match(/http:\/\/127\.0\.0\.1:\d+\/#token=[a-f0-9]+/); if (match) { clearTimeout(timer); resolveUrl(match[0]) } })
  })
}
test.beforeEach(async () => { directory = mkdtempSync(join(tmpdir(), 'sunchang-productions-')); binary = join(directory, binaryName); cpSync(resolve('..', 'target', 'release', binaryName), binary); await launch() })
test.afterEach(async () => { if (server && server.exitCode === null && server.signalCode === null) server.kill(); if (closed) await closed; if (directory) { expect(resolve(directory).startsWith(resolve(tmpdir()))).toBe(true); rmSync(directory, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 }) } })
async function open(page: Page) { await page.goto(launchUrl); await expect(page.getByLabel('재료 이름', { exact: false })).toBeEnabled() }
async function visit(page: Page, path: string) { await page.goto(new URL(path, launchUrl).toString()) }
async function get(page: Page, path: string) { const response = await page.request.get(new URL(path, launchUrl).toString()); expect(response.ok()).toBe(true); return response.json() }
async function post(page: Page, path: string, data: unknown) { const response = await page.request.post(new URL(path, launchUrl).toString(), { headers: { Origin: new URL(launchUrl).origin }, data }); expect(response.status(), await response.text()).toBe(201); return response.json() }
async function meta(page: Page) { const state = await get(page, '/api/state'); return { request_id: randomUUID(), epoch: state.epoch, expected_revision: state.revision } }
async function material(page: Page, name: string, kind = 'raw', unit = 'g') { return post(page, '/api/materials', { ...await meta(page), name, unit, kind, description: '' }) }

async function stock(page: Page, item: any, quantity = '3', amount = '30000') {
  const draft = await post(page, '/api/business/drafts', { meta: await meta(page), payload: { kind: 'receipt', version: 1, data: { source: { kind: 'direct', material_id: item.id, material_version_id: item.version_id }, quantity, unit: 'kg', amount, receipt_date: '2024-02-29', note: '', stock_effect: { kind: 'unreflected' } } }, corrects_confirmation_id: null })
  return post(page, `/api/business/${draft.result.item.id}/confirm`, { meta: await meta(page), expected_head_id: draft.result.head.id })
}
async function start(page: Page) { await visit(page, '/business?kind=production'); const reset = page.getByRole('button', { name: '새 생산 초안 작성', exact: true }); if (await reset.isVisible()) await reset.click(); await expect(page.getByRole('heading', { name: '실제 생산 초안', exact: true })).toBeVisible() }
async function row(page: Page, side: '투입' | '결과', index: number, id: string, quantity: string, unit = 'kg', allocation?: string) { await page.getByLabel(`${side} ${index} 품목`, { exact: true }).selectOption(id); await page.getByLabel(`${side} ${index} 실제 수량`, { exact: true }).fill(quantity); await page.getByLabel(`${side} ${index} 단위`, { exact: true }).fill(unit); if (allocation !== undefined) await page.getByLabel(`결과 ${index} 원가 배분 (%)`, { exact: true }).fill(allocation) }
async function saveNew(page: Page) { await page.getByRole('button', { name: '초안 임시 저장', exact: true }).click(); return openSaved(page) }
async function openSaved(page: Page) { const link = page.getByRole('link', { name: '저장한 초안 열기', exact: true }); await expect(link).toBeVisible(); const path = (await link.getAttribute('href'))!; await link.click(); await expect(page).toHaveURL(new URL(path, launchUrl).toString()); await expect(page.getByRole('button', { name: '확정 전 미리보기', exact: true })).toBeVisible(); return path }
async function revise(page: Page) { await page.getByRole('button', { name: '수정 내용 임시 저장', exact: true }).click(); await expect(page.getByText('수정 내용을 임시 저장했습니다.', { exact: true })).toBeVisible(); await expect(page.getByRole('button', { name: '확정 전 미리보기', exact: true })).toBeEnabled() }
async function preview(page: Page) { await page.getByRole('button', { name: '확정 전 미리보기', exact: true }).click(); await expect(page.getByRole('button', { name: '이 내용으로 확정', exact: true })).toBeEnabled() }
async function confirmed(page: Page) { const link = page.getByRole('link', { name: '실제 생산 기록 보기', exact: true }); await expect(link).toBeVisible(); const path = (await link.getAttribute('href'))!; await link.click(); await expect(page.getByRole('heading', { name: '확정한 실제 생산', exact: true })).toBeVisible(); return path }
async function confirm(page: Page) { await page.getByRole('button', { name: '이 내용으로 확정', exact: true }).click(); return confirmed(page) }
async function restart(page: Page) { await page.goto('about:blank'); let timer: ReturnType<typeof setTimeout> | undefined; const [code, signal] = await Promise.race([closed, new Promise<never>((_, reject) => { timer = setTimeout(() => reject(new Error('production normal shutdown timed out')), 180_000) })]).finally(() => clearTimeout(timer)); expect(code).toBe(0); expect(signal).toBeNull(); await launch(); await open(page) }

test('unfinished actual rows survive normal restart and full input produces exact value with six viewport themes', async ({ page }) => {
  test.setTimeout(260_000); await open(page)
  const raw = await material(page, '가상 원재료 — 손질 전 긴 채소 이름'), output = await material(page, '가상 생산 결과 — 천천히 손질한 채소와 긴 품목 이름', 'prepared')
  await stock(page, raw); await start(page)
  await expect(page.getByLabel('투입 1 실제 수량', { exact: true })).toHaveValue(''); await expect(page.getByLabel('결과 1 실제 수량', { exact: true })).toHaveValue(''); await expect(page.getByLabel('결과 1 원가 배분 (%)')).toHaveValue('100')
  await row(page, '투입', 1, raw.id, '3.'); await row(page, '결과', 1, output.id, ''); await page.getByLabel('생산 메모 (선택)').fill('가상 검증용 생산\n실제량은 직접 입력')
  const draft = await saveNew(page); await restart(page); await visit(page, draft)
  await expect(page.getByLabel('투입 1 실제 수량', { exact: true })).toHaveValue('3.'); await expect(page.getByLabel('결과 1 실제 수량', { exact: true })).toHaveValue('')
  await page.getByLabel('투입 1 실제 수량', { exact: true }).fill('3'); await page.getByLabel('결과 1 실제 수량', { exact: true }).fill('2'); await revise(page); await preview(page)
  await expect(page.locator('.production-preview')).toContainText('총 투입 원가 30,000원'); await expect(page.locator('.production-preview')).toContainText('아직 확인한 수량 없음')
  // Include editable multiple rows and the complete allocation preview at all widths.
  await page.getByRole('button', { name: '투입 행 추가', exact: true }).click(); await row(page, '투입', 2, raw.id, '1'); await page.getByLabel('투입 1 실제 수량', { exact: true }).fill('2'); await page.getByRole('button', { name: '결과 행 추가', exact: true }).click(); await row(page, '결과', 2, output.id, '0.5', 'kg', '30'); await page.getByLabel('결과 1 실제 수량', { exact: true }).fill('1.5'); await page.getByLabel('결과 1 원가 배분 (%)').fill('70'); await revise(page); await preview(page)
  for (const [width, height] of [[1440, 900], [768, 1024], [390, 844]] as const) { await page.setViewportSize({ width, height }); for (const theme of ['light', 'dark'] as const) { if (await page.locator('html').getAttribute('data-theme') !== theme) await page.getByRole('button', { name: theme === 'dark' ? '다크 모드로 전환' : '라이트 모드로 전환' }).click(); expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true); await page.screenshot({ path: test.info().outputPath(`production-${width}-${theme}.png`), fullPage: true }) } }
  const path = await confirm(page), detail = await get(page, `/api${path}`)
  expect(detail.item.total_input_value).toBe('30000'); expect(detail.item.outputs.map((value: any) => value.value)).toEqual(['21000', '9000']); expect(detail.materials).toHaveLength(2)
  const rawHolding = await get(page, `/api/holdings/${raw.id}`), outputHolding = await get(page, `/api/holdings/${output.id}`)
  expect(rawHolding.settlement_basis.quantity).toBe('0'); expect(rawHolding.last_valid_price.amount).toBe('30000'); expect(outputHolding.settlement_basis.quantity).toBe('2000'); expect(outputHolding.settlement_basis.value).toBe('30000'); expect(outputHolding.observation).toBeNull()
  await page.reload(); await expect(page.getByRole('heading', { name: '확정한 실제 생산', exact: true })).toBeVisible()
  await visit(page, `/materials/${output.id}`); await expect(page.getByRole('region', { name: '보유 근거', exact: true })).toContainText('첫 생산 근거'); expect(await page.locator('.holding-panel a[href^="/productions/"]').count()).toBe(0)
  await page.getByRole('link', { name: '이 품목의 생산 기록', exact: true }).click(); await expect(page.locator('.production-list > li')).toHaveCount(1)
})

test('allocation defaults remain manual and zero value plus same material net changes stay visible', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 반복 투입'), output = await material(page, '가상 부산물', 'stocked'); await stock(page, raw, '3', '30000'); await start(page)
  await row(page, '투입', 1, raw.id, '1'); await row(page, '결과', 1, raw.id, '1'); await page.getByRole('button', { name: '결과 행 추가', exact: true }).click(); await expect(page.getByLabel('결과 2 원가 배분 (%)')).toHaveValue('0'); await row(page, '결과', 2, output.id, '0.5')
  await page.getByLabel('결과 1 원가 배분 (%)').fill('70'); await page.getByLabel('결과 2 원가 배분 (%)').fill('30'); await page.getByRole('button', { name: '결과 2 행 삭제', exact: true }).click(); await expect(page.getByLabel('결과 1 원가 배분 (%)')).toHaveValue('70')
  await page.getByLabel('결과 1 원가 배분 (%)').fill('100'); await page.getByRole('button', { name: '결과 행 추가', exact: true }).click(); await row(page, '결과', 2, output.id, '0.5'); await saveNew(page); await preview(page); const path = await confirm(page), detail = await get(page, `/api${path}`)
  expect(detail.item.outputs.map((value: any) => value.value)).toEqual(['10000', '0']); expect(detail.materials).toHaveLength(2)
  await visit(page, `/materials/${raw.id}`); await expect(page.locator('.holding-changes')).toContainText('투입 1,000g · 10,000원 / 결과 1,000g · 10,000원'); await expect(page.locator('.holding-changes a')).toHaveAttribute('href', path)
  await visit(page, `/productions?material=${raw.id}`); await expect(page.locator('.production-list > li')).toHaveCount(1)
  await start(page); await row(page, '투입', 1, output.id, '0.5'); await row(page, '결과', 1, raw.id, '0.25'); await saveNew(page); await preview(page); await expect(page.locator('.production-preview')).toContainText('총 투입 원가 0원'); await confirm(page)
})

test('unknown or insufficient input keeps measurements and cannot use its own output to hide shortage', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 근거 확인 필요'), output = await material(page, '가상 확인 후 결과', 'menu'); await start(page); await row(page, '투입', 1, raw.id, '2'); await row(page, '결과', 1, raw.id, '3'); const draft = await saveNew(page)
  await page.getByRole('button', { name: '확정 전 미리보기', exact: true }).click(); await expect(page.getByRole('button', { name: '이 내용으로 확정' })).toBeDisabled(); await expect(page.getByLabel('투입 1 실제 수량', { exact: true })).toHaveValue('2'); await expect(page.locator('.production-preview .business-issues')).toBeVisible()
  await stock(page, raw, '1', '100'); await visit(page, draft); await page.getByRole('button', { name: '확정 전 미리보기', exact: true }).click(); await expect(page.getByRole('button', { name: '이 내용으로 확정' })).toBeDisabled(); await expect(page.getByLabel('결과 1 실제 수량', { exact: true })).toHaveValue('3')
  await stock(page, raw, '1', '100'); await visit(page, draft); await row(page, '결과', 1, output.id, '1'); await revise(page); await preview(page); await confirm(page)
})

test('create revise and confirm retries preserve arrays UUID and atomic results after loss and JSON failure', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 재시도 투입'), output = await material(page, '가상 재시도 결과'); await stock(page, raw); await start(page); await row(page, '투입', 1, raw.id, '1'); await row(page, '결과', 1, output.id, '0.8')
  const creates: unknown[] = []; await page.route('**/api/production-drafts', async route => { creates.push(route.request().postDataJSON()); if (creates.length === 1) { await route.fetch(); await route.abort('failed') } else await route.continue() })
  await page.getByRole('button', { name: '초안 임시 저장', exact: true }).click(); await expect(page.getByRole('button', { name: '같은 업무 요청 재시도' })).toBeVisible(); await page.reload(); await page.getByRole('button', { name: '같은 업무 요청 재시도' }).click(); const draft = await openSaved(page); expect(creates[1]).toEqual(creates[0])
  const revisions: unknown[] = []; const blockedPath = resolve(directory, 'db', 'snapshots'); expect(blockedPath.startsWith(resolve(directory))).toBe(true); rmSync(blockedPath, { recursive: true }); writeFileSync(blockedPath, 'blocked by production fixture')
  await page.route('**/api/production-drafts/*/revisions', async route => { revisions.push(route.request().postDataJSON()); await route.continue() }); await page.getByLabel('생산 메모 (선택)').fill('원문과 배분 유지'); await page.getByRole('button', { name: '수정 내용 임시 저장' }).click(); await expect(page.getByRole('button', { name: '같은 업무 요청 재시도' })).toBeVisible(); rmSync(blockedPath); await page.reload(); await page.getByRole('button', { name: '같은 업무 요청 재시도' }).click(); await expect(page.getByText('수정 내용을 임시 저장했습니다.', { exact: true })).toBeVisible(); expect(revisions[1]).toEqual(revisions[0]); await preview(page)
  const confirms: unknown[] = []; await page.route('**/api/business/*/confirm', async route => { confirms.push(route.request().postDataJSON()); if (confirms.length === 1) { await route.fetch(); await route.abort('failed') } else await route.continue() }); await page.getByRole('button', { name: '이 내용으로 확정' }).click(); await expect(page.getByRole('button', { name: '같은 업무 요청 재시도' })).toBeVisible(); await page.reload(); await page.getByRole('button', { name: '같은 업무 요청 재시도' }).click(); await confirmed(page); expect(confirms[1]).toEqual(confirms[0]); expect((await get(page, '/api/productions')).items).toHaveLength(1); expect((await get(page, `/api${draft}`)).confirmations).toHaveLength(1); expect((await get(page, `/api/holdings/${raw.id}`)).settlement_basis.quantity).toBe('2000')
})

test('stale preview and another tab confirmation preserve local rows separately from confirmed input', async ({ page, context }) => {
  await open(page); const raw = await material(page, '가상 충돌 투입'), output = await material(page, '가상 충돌 결과'); await stock(page, raw); await start(page); await row(page, '투입', 1, raw.id, '1'); await row(page, '결과', 1, output.id, '1'); const draft = await saveNew(page); await preview(page)
  await stock(page, raw, '1', '100'); await page.getByRole('button', { name: '이 내용으로 확정' }).click(); await expect(page.getByRole('alert')).toContainText('이 요청은 처리하지 않았습니다'); await page.getByRole('button', { name: '최신 기록 확인 후 입력 검토' }).click(); await expect(page.getByLabel('투입 1 실제 수량', { exact: true })).toHaveValue('1'); await preview(page)
  const other = await context.newPage(); await visit(other, draft); await page.getByLabel('투입 1 실제 수량', { exact: true }).fill('2'); await preview(other); await confirm(other); await page.getByRole('button', { name: '수정 내용 임시 저장' }).click(); await expect(page.getByRole('button', { name: '최신 기록 확인 후 입력 검토' })).toBeVisible(); await page.getByRole('button', { name: '최신 기록 확인 후 입력 검토' }).click()
  await expect(page.getByRole('region', { name: '이 화면에 남은 미저장 입력' })).toContainText('2 kg'); await expect(page.getByRole('region', { name: '저장된 업무 내용' })).toContainText('1 kg'); await expect(page.getByRole('button', { name: '수정 내용 임시 저장' })).not.toBeVisible(); await other.close()
})

test('late row reference and save completion cannot overwrite new selections across navigation', async ({ page }) => {
  await open(page); const first = await material(page, '가상 이전 품목'), next = await material(page, '가상 다음 품목'), archived = await material(page, '가상 보관 품목'); await post(page, `/api/materials/${archived.id}/archive`, { ...await meta(page), expected_version_id: archived.version_id }); await start(page); await expect(page.getByLabel('투입 1 품목', { exact: true }).locator(`option[value="${archived.id}"]`)).toHaveCount(0); await row(page, '투입', 1, first.id, '1'); await row(page, '결과', 1, next.id, '2'); const draft = await saveNew(page)
  let release = () => {}; const gate = new Promise<void>(resolveGate => { release = resolveGate }); await page.route(`**/api/materials/${first.id}`, async route => { await gate; await route.continue() }); await page.getByRole('button', { name: '투입 1 현재 품목 정보 적용' }).click(); await page.getByRole('navigation', { name: '주요 메뉴' }).getByRole('link', { name: '작업 시작', exact: true }).click(); await expect(page.getByRole('heading', { name: '작업 시작', exact: true })).toBeVisible(); await page.goBack(); await page.getByLabel('투입 1 품목', { exact: true }).selectOption(next.id)
  const late = page.waitForResponse(response => new URL(response.url()).pathname === `/api/materials/${first.id}`); release(); await late; await page.unroute(`**/api/materials/${first.id}`)
  let finish = () => {}; const savingGate = new Promise<void>(resolveGate => { finish = resolveGate }); let captured: any; await page.route('**/api/production-drafts/*/revisions', async route => { captured = route.request().postDataJSON(); const response = await route.fetch(); await savingGate; await route.fulfill({ response }) }); await page.getByRole('button', { name: '수정 내용 임시 저장' }).click(); await page.getByRole('navigation', { name: '주요 메뉴' }).getByRole('link', { name: '작업 시작', exact: true }).click(); await expect(page.getByRole('heading', { name: '작업 시작', exact: true })).toBeVisible(); await page.goBack(); await expect(page.getByRole('button', { name: '수정 내용 임시 저장' })).toBeDisabled(); finish(); await expect(page.getByRole('button', { name: '수정 내용 임시 저장' })).toBeEnabled(); expect(captured.payload.data.inputs[0].material_id).toBe(next.id); expect((await get(page, `/api${draft}`)).head.payload.data.inputs[0].material_version_id).toBe(next.version_id)
})

test('large draft alias preserves 100 rows and explicit size review allows edit then archive', async ({ page }) => {
  test.setTimeout(60_000); await open(page); const item = await material(page, '가상 큰 초안 품목'); const input = { material_id: item.id, material_version_id: item.version_id, quantity: '1.', unit: 'kg' }; const value = { kind: 'production', version: 1, data: { inputs: Array.from({ length: 100 }, () => ({ ...input })), outputs: Array.from({ length: 100 }, (_, i) => ({ ...input, allocation_percent: i === 0 ? '70' : '0' })), note: '가상 큰 초안' } }
  const draft = await post(page, '/api/production-drafts', { meta: await meta(page), payload: value, corrects_confirmation_id: null }); await visit(page, `/business/${draft.result.item.id}`); await expect(page.locator('.production-row')).toHaveCount(200); await expect(page.getByRole('button', { name: '투입 행 추가', exact: true })).toBeDisabled(); await expect(page.getByLabel('결과 1 원가 배분 (%)')).toHaveValue('70'); await page.getByRole('button', { name: '결과 100 행 삭제', exact: true }).click(); await expect(page.getByLabel('결과 1 원가 배분 (%)')).toHaveValue('70')
  await page.route('**/api/production-drafts/*/revisions', route => route.fulfill({ status: 413, contentType: 'application/json', body: JSON.stringify({ error: '입력 크기를 줄여 주세요.', code: 'validation' }) })); await page.getByRole('button', { name: '수정 내용 임시 저장' }).click(); await expect(page.getByRole('alert')).toContainText('입력 크기를 줄여'); await expect(page.getByRole('button', { name: '같은 업무 요청 재시도' })).not.toBeVisible(); await page.unroute('**/api/production-drafts/*/revisions'); await page.getByRole('button', { name: '최신 기록 확인 후 입력 검토' }).click(); await expect(page.locator('.production-row')).toHaveCount(199); await revise(page)
  await page.getByRole('button', { name: '초안 보관', exact: true }).click(); await page.getByRole('button', { name: '초안 보관 확정', exact: true }).click(); await expect(page.getByText('초안을 보관했습니다. 이전 입력은 이력에 남아 있습니다.', { exact: true })).toBeVisible(); expect((await get(page, '/api/productions')).items).toHaveLength(0)
})
