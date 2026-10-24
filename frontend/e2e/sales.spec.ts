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
    const timer = setTimeout(() => reject(new Error(`sale fixture start timed out: ${errors}`)), 15_000)
    server.stderr!.on('data', data => { errors += String(data) })
    server.once('exit', code => { clearTimeout(timer); reject(new Error(`sale fixture exited ${code}: ${errors}`)) })
    server.stdout!.on('data', data => { output += String(data); const match = output.match(/http:\/\/127\.0\.0\.1:\d+\/#token=[a-f0-9]+/); if (match) { clearTimeout(timer); resolveUrl(match[0]) } })
  })
}
test.beforeEach(async () => { directory = mkdtempSync(join(tmpdir(), 'sunchang-sales-')); binary = join(directory, binaryName); cpSync(resolve('..', 'target', 'release', binaryName), binary); await launch() })
test.afterEach(async () => { if (server && server.exitCode === null && server.signalCode === null) server.kill(); if (closed) await closed; if (directory) { expect(resolve(directory).startsWith(resolve(tmpdir()))).toBe(true); rmSync(directory, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 }) } })
async function open(page: Page) { await page.goto(launchUrl); await expect(page.getByLabel('재료 이름', { exact: false })).toBeEnabled() }
async function visit(page: Page, path: string) { await page.goto(new URL(path, launchUrl).toString()) }
async function get(page: Page, path: string) { const response = await page.request.get(new URL(path, launchUrl).toString()); expect(response.ok()).toBe(true); return response.json() }
async function post(page: Page, path: string, data: unknown) { const response = await page.request.post(new URL(path, launchUrl).toString(), { headers: { Origin: new URL(launchUrl).origin }, data }); expect(response.status(), await response.text()).toBe(201); return response.json() }
async function meta(page: Page) { const state = await get(page, '/api/state'); return { request_id: randomUUID(), epoch: state.epoch, expected_revision: state.revision } }
async function material(page: Page, name: string, kind = 'raw', unit = 'g') { return post(page, '/api/materials', { ...await meta(page), name, unit, kind, description: '' }) }

function holdingValues(value: any) { const { state, ...values } = value; return values }
async function price(page: Page, item: any, amount = '10') { return post(page, `/api/materials/${item.id}/prices`, { ...await meta(page), expected_version_id: item.version_id, quantity: '1', amount, unit: item.unit }) }
function data(selling = '1000', lines: any[] = []) { return { basis_count: '3', reference_output: null, selling_price: selling, cooking_description: '', process: '', reference_time: '', lines } }
function line(item: any, quantity = '100') { return { material_id: item.id, material_version_id: item.version_id, quantity, unit: item.unit, note: '' } }
async function recipe(page: Page, name = '가상 판매 메뉴', selling: string | null = '1000', lines: any[] = []) { return post(page, '/api/recipes', { meta: await meta(page), owner: { mode: 'new', name, unit: '개', kind: 'menu', description: '' }, data: { ...data('1000', lines), selling_price: selling } }) }
async function apiSale(page: Page, entry: any, count = '3') { const draft = await post(page, '/api/business/drafts', { meta: await meta(page), payload: { kind: 'sale', version: 1, data: { material_id: entry.item.id, material_version_id: entry.item.version_id, recipe_version_id: entry.version.id, count, note: '' } }, corrects_confirmation_id: null }); return post(page, `/api/business/${draft.result.item.id}/confirm`, { meta: await meta(page), expected_head_id: draft.result.head.id }) }
async function start(page: Page, id?: string) { await visit(page, `/business?kind=sale${id ? `&material=${id}` : ''}`); const reset = page.getByRole('button', { name: '새 판매 초안 작성', exact: true }); if (await reset.isVisible()) await reset.click(); await expect(page.getByRole('heading', { name: '판매 초안', exact: true })).toBeVisible() }
async function openSaved(page: Page) { const link = page.getByRole('link', { name: '저장한 초안 열기', exact: true }); await expect(link).toBeVisible(); const path = (await link.getAttribute('href'))!; await link.click(); await expect(page.getByRole('button', { name: '확정 전 미리보기', exact: true })).toBeVisible(); return path }
async function saveNew(page: Page) { await page.getByRole('button', { name: '초안 임시 저장', exact: true }).click(); return openSaved(page) }
async function revise(page: Page) { await page.getByRole('button', { name: '수정 내용 임시 저장', exact: true }).click(); await expect(page.getByText('수정 내용을 임시 저장했습니다.', { exact: true })).toBeVisible(); await expect(page.getByRole('button', { name: '확정 전 미리보기', exact: true })).toBeEnabled() }
async function preview(page: Page) { await page.getByRole('button', { name: '확정 전 미리보기', exact: true }).click(); await expect(page.getByRole('button', { name: '이 내용으로 확정', exact: true })).toBeEnabled() }
async function confirmed(page: Page) { const link = page.getByRole('link', { name: '판매 기록과 개수 정정 이력 보기', exact: true }); await expect(link).toBeVisible(); const path = (await link.getAttribute('href'))!; await link.click(); await expect(page).toHaveURL(new URL(path, launchUrl).toString()); await expect(page.getByRole('heading', { name: '확정한 판매', exact: true })).toBeVisible(); return path }
async function confirm(page: Page) { await page.getByRole('button', { name: '이 내용으로 확정', exact: true }).click(); return confirmed(page) }
async function restart(page: Page) { await page.goto('about:blank'); let timer: ReturnType<typeof setTimeout> | undefined; const [code, signal] = await Promise.race([closed, new Promise<never>((_, reject) => { timer = setTimeout(() => reject(new Error('sale normal shutdown timed out')), 180_000) })]).finally(() => clearTimeout(timer)); expect(code).toBe(0); expect(signal).toBeNull(); await launch(); await open(page) }

test('unfinished sale survives normal restart then preview shows frozen cost at six viewport themes', async ({ page }) => {
  test.setTimeout(260_000); await open(page)
  const raw = await material(page, '가상 채소 — 천천히 손질한 긴 재료 이름'), missing = await material(page, '가상 가격 미입력 양념'); await price(page, raw)
  const entry = await recipe(page, '가상 판매 메뉴 — 천천히 볶은 채소와 든든한 한 그릇', '1000', [line(raw), line(missing, '0')])
  const holding = await get(page, `/api/holdings/${raw.id}`)
  await start(page, entry.item.id); await expect(page.getByLabel('판매 개수', { exact: true })).toHaveValue(''); await page.getByLabel('판매 개수', { exact: true }).fill('3.'); await page.getByLabel('판매 메모 (선택)').fill('가상 판매 검증\n실측 소모와 구분')
  const draft = await saveNew(page); await restart(page); await visit(page, draft); await expect(page.getByLabel('판매 개수', { exact: true })).toHaveValue('3.'); await expect(page.getByLabel('판매 메모 (선택)')).toHaveValue('가상 판매 검증\n실측 소모와 구분')
  await page.getByLabel('판매 개수', { exact: true }).fill('3'); await revise(page); await preview(page); await expect(page.locator('.sale-preview')).toContainText('3개 · 3,000원'); await expect(page.locator('.sale-cost-snapshot')).toContainText('333.333333원'); await expect(page.locator('.sale-cost-snapshot')).toContainText('가격 없음')
  for (const [width, height] of [[1440, 900], [768, 1024], [390, 844]] as const) { await page.setViewportSize({ width, height }); for (const theme of ['light', 'dark'] as const) { if (await page.locator('html').getAttribute('data-theme') !== theme) await page.getByRole('button', { name: theme === 'dark' ? '다크 모드로 전환' : '라이트 모드로 전환' }).click(); expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true); await page.screenshot({ path: test.info().outputPath(`sale-${width}-${theme}.png`), fullPage: true }) } }
  const path = await confirm(page), sale = await get(page, `/api${path}`); expect(sale.item.recipe_cost.total_cost).toBe('333.333333'); expect(sale.head.amount).toBe('3000'); expect(sale.consumption_status).toBe('unconfirmed'); expect(holdingValues(await get(page, `/api/holdings/${raw.id}`))).toEqual(holdingValues(holding))
  await page.reload(); await expect(page.locator('.sale-cost-snapshot')).toContainText('333.333333원'); await expect(page.getByRole('heading', { name: '실사 소모 미확정', exact: true })).toBeVisible()
})

test('sale snapshots survive changed recipes prices names and restart while count corrections keep original sequence', async ({ page }) => {
  test.setTimeout(260_000); await open(page); const raw = await material(page, '가상 판매 당시 재료'); await price(page, raw)
  const entry = await recipe(page, '가상 당시 메뉴', '1000', [line(raw)]); await start(page, entry.item.id); await page.getByLabel('판매 개수', { exact: true }).fill('3'); await saveNew(page); await preview(page); const path = await confirm(page), before = await get(page, `/api${path}`)
  await price(page, raw, '20'); await post(page, `/api/materials/${raw.id}/versions`, { ...await meta(page), expected_version_id: raw.version_id, name: '가상 현재 변경 이름', unit: 'g', kind: 'raw', description: '' })
  await post(page, `/api/recipes/${entry.item.id}/versions`, { meta: await meta(page), expected_material_version_id: entry.item.version_id, expected_recipe_version_id: entry.version.id, data: data('2000', [line(raw, '60')]) })
  await restart(page); await visit(page, path); await expect(page.locator('.sale-cost-snapshot')).toContainText('333.333333원'); await expect(page.locator('.sale-cost-snapshot')).toContainText('가상 판매 당시 재료'); await expect(page.locator('.sale-cost-snapshot')).not.toContainText('가상 현재 변경 이름')
  const costs: string[] = []; page.on('request', request => { if (new URL(request.url()).pathname.endsWith('/cost')) costs.push(request.url()) })
  for (const count of ['1', '0']) { await page.getByRole('link', { name: '판매 개수 정정하기', exact: true }).click(); await expect(page.getByRole('heading', { name: '판매 개수 정정 초안', exact: true })).toBeVisible(); await expect(page.getByLabel('판매 메뉴', { exact: true })).toHaveCount(0); await expect(page.getByRole('button', { name: '현재 메뉴·레시피 정보 적용' })).toHaveCount(0); await page.getByLabel('판매 개수', { exact: true }).fill(count); const correction = await saveNew(page); await page.getByRole('button', { name: '확정 전 미리보기', exact: true }).click(); await expect(page.getByRole('button', { name: '이 내용으로 확정' })).toBeDisabled(); await page.getByLabel('판매 정정 이유').fill(`가상 개수 ${count}로 정정`); await revise(page); await preview(page); await expect(page.locator('.sale-preview')).toContainText(`${count}개 · ${count === '1' ? '1,000' : '0'}원`); expect(await confirm(page)).toBe(path); expect(correction).toMatch(/^\/business\//) }
  const after = await get(page, `/api${path}`); expect(after.item).toEqual(before.item); expect(after.versions.map((value: any) => value.count)).toEqual(['3', '1', '0']); expect(after.head.amount).toBe('0'); expect(costs).toEqual([]); await expect(page.getByRole('heading', { name: '원본과 개수 정정 이력' })).toBeVisible()
})

test('zero count zero price missing price and unknown ingredient cost stay distinct without consuming stock', async ({ page }) => {
  await open(page); const missing = await material(page, '가상 원가 미입력 재료'), zero = await recipe(page, '가상 무료 메뉴', '0', [line(missing, '0')]), absent = await recipe(page, '가상 판매가 미입력', null, [line(missing, '1')]), empty = await recipe(page, '가상 빈 구성', '1000')
  const holding = await get(page, `/api/holdings/${missing.id}`)
  for (const [entry, count, amount, cost] of [[zero, '3', '0', '0'], [absent, '0', null, null], [empty, '0', '0', '0']] as const) { await start(page, entry.item.id); await page.getByLabel('판매 개수', { exact: true }).fill(count); await saveNew(page); await preview(page); const path = await confirm(page), value = await get(page, `/api${path}`); expect(value.head.amount).toBe(amount); expect(value.item.recipe_cost.total_cost).toBe(cost); await expect(page.locator('.sale-detail')).toContainText(amount === null ? '미계산' : '0원') }
  expect(holdingValues(await get(page, `/api/holdings/${missing.id}`))).toEqual(holdingValues(holding)); await visit(page, '/sales'); await expect(page.locator('.sale-totals')).toContainText('0원'); await expect(page.locator('.sale-totals')).toContainText('1건'); await expect(page.locator('.sale-list > li')).toHaveCount(3)
})

test('create revise confirm retries keep exact UUID body after response loss and JSON failure', async ({ page }) => {
  await open(page); const entry = await recipe(page); await start(page, entry.item.id); await page.getByLabel('판매 개수', { exact: true }).fill('0')
  const creates: unknown[] = []; await page.route('**/api/business/drafts', async route => { creates.push(route.request().postDataJSON()); if (creates.length === 1 || creates.length === 3) { await route.fetch(); await route.abort('failed') } else await route.continue() })
  await page.getByRole('button', { name: '초안 임시 저장' }).click(); await expect(page.getByRole('button', { name: '같은 업무 요청 재시도' })).toBeVisible(); await page.reload(); await page.getByRole('button', { name: '같은 업무 요청 재시도' }).click(); const draft = await openSaved(page); expect(creates[1]).toEqual(creates[0])
  const revisions: unknown[] = [], blocked = resolve(directory, 'db', 'snapshots'); expect(blocked.startsWith(resolve(directory))).toBe(true); rmSync(blocked, { recursive: true }); writeFileSync(blocked, 'blocked by sale fixture')
  await page.route('**/api/business/*/draft-revisions', async route => { revisions.push(route.request().postDataJSON()); await route.continue() }); await page.getByLabel('판매 개수', { exact: true }).fill('3'); await page.getByRole('button', { name: '수정 내용 임시 저장' }).click(); await expect(page.getByRole('button', { name: '같은 업무 요청 재시도' })).toBeVisible(); rmSync(blocked); await page.reload(); await page.getByRole('button', { name: '같은 업무 요청 재시도' }).click(); await expect(page.getByText('수정 내용을 임시 저장했습니다.', { exact: true })).toBeVisible(); expect(revisions[1]).toEqual(revisions[0]); await preview(page)
  const confirms: unknown[] = []; await page.route('**/api/business/*/confirm', async route => { confirms.push(route.request().postDataJSON()); if (confirms.length === 1) { await route.fetch(); await route.abort('failed') } else await route.continue() }); await page.getByRole('button', { name: '이 내용으로 확정' }).click(); await expect(page.getByRole('button', { name: '같은 업무 요청 재시도' })).toBeVisible(); await page.reload(); await page.getByRole('button', { name: '같은 업무 요청 재시도' }).click(); await confirmed(page); expect(confirms[1]).toEqual(confirms[0]); expect((await get(page, '/api/sales')).items).toHaveLength(1); expect((await get(page, `/api${draft}`)).confirmations).toHaveLength(1)
  const original = (await get(page, '/api/sales')).items[0], sale = await get(page, `/api/sales/${original.id}`)
  await page.getByRole('link', { name: '판매 개수 정정하기', exact: true }).click(); await page.getByLabel('판매 개수', { exact: true }).fill('1'); await page.getByLabel('판매 정정 이유').fill('응답 유실 정정'); await page.getByRole('button', { name: '초안 임시 저장', exact: true }).click(); await expect(page.getByRole('button', { name: '같은 업무 요청 재시도' })).toBeVisible()
  const competing = await post(page, '/api/business/drafts', { meta: await meta(page), payload: { kind: 'sale', version: 1, data: { material_id: entry.item.id, material_version_id: entry.item.version_id, recipe_version_id: entry.version.id, count: '2', note: '다른 탭의 정정' } }, corrects_confirmation_id: sale.head.confirmation_id })
  await post(page, `/api/business/${competing.result.item.id}/confirm`, { meta: await meta(page), expected_head_id: competing.result.head.id })
  await page.reload(); await page.getByRole('button', { name: '같은 업무 요청 재시도' }).click(); await openSaved(page); expect(creates[3]).toEqual(creates[2]); await expect(page.getByLabel('판매 개수', { exact: true })).toHaveValue('1'); await page.getByRole('button', { name: '확정 전 미리보기', exact: true }).click(); await expect(page.getByRole('button', { name: '이 내용으로 확정' })).toBeDisabled(); expect((await get(page, '/api/sales')).items).toHaveLength(1)

})

test('stale preview and another tab confirmation preserve local count separately from stored sale', async ({ page, context }) => {
  await open(page); const entry = await recipe(page); await start(page, entry.item.id); await page.getByLabel('판매 개수', { exact: true }).fill('3'); const draft = await saveNew(page); await preview(page); await material(page, '가상 동시 자료 변경'); await page.getByRole('button', { name: '이 내용으로 확정' }).click(); await expect(page.getByRole('alert')).toContainText('이 요청은 처리하지 않았습니다'); await page.getByRole('button', { name: '최신 기록 확인 후 입력 검토' }).click(); await expect(page.getByLabel('판매 개수', { exact: true })).toHaveValue('3'); await preview(page)
  const other = await context.newPage(); await visit(other, draft); await page.getByLabel('판매 개수', { exact: true }).fill('9'); await preview(other); await confirm(other); await page.getByRole('button', { name: '수정 내용 임시 저장' }).click(); await expect(page.getByRole('button', { name: '최신 기록 확인 후 입력 검토' })).toBeVisible(); await page.getByRole('button', { name: '최신 기록 확인 후 입력 검토' }).click(); await expect(page.getByRole('region', { name: '이 화면에 남은 미저장 입력' })).toContainText('판매 9개'); await expect(page.getByRole('region', { name: '저장된 업무 내용' })).toContainText('판매 3개'); await expect(page.getByRole('button', { name: '수정 내용 임시 저장' })).toHaveCount(0); await other.close()
})

test('late current recipe response cannot overwrite selection across navigation and archived choices stay explicit', async ({ page }) => {
  await open(page); const first = await recipe(page, '가상 이전 메뉴'), next = await recipe(page, '가상 다음 메뉴'), archived = await recipe(page, '가상 보관 메뉴'); await post(page, `/api/materials/${archived.item.id}/archive`, { ...await meta(page), expected_version_id: archived.item.version_id }); await start(page); await expect(page.getByLabel('판매 메뉴', { exact: true }).locator(`option[value="${archived.item.id}"]`)).toHaveCount(0); await page.getByLabel('판매 메뉴', { exact: true }).selectOption(first.item.id); await page.getByLabel('판매 개수', { exact: true }).fill('1'); const draft = await saveNew(page)
  let release = () => {}; const gate = new Promise<void>(resolveGate => { release = resolveGate }); await page.route(`**/api/recipes/${first.item.id}`, async route => { await gate; await route.continue() }); await page.getByRole('button', { name: '현재 메뉴·레시피 정보 적용' }).click(); await page.getByRole('navigation', { name: '주요 메뉴' }).getByRole('link', { name: '작업 시작', exact: true }).click(); await expect(page.getByRole('heading', { name: '작업 시작', exact: true })).toBeVisible(); await page.goBack(); await page.getByLabel('판매 메뉴', { exact: true }).selectOption(next.item.id)
  const late = page.waitForResponse(response => new URL(response.url()).pathname === `/api/recipes/${first.item.id}`); release(); await late; await page.unroute(`**/api/recipes/${first.item.id}`); await expect(page.getByLabel('판매 메뉴', { exact: true })).toHaveValue(next.item.id); await revise(page); expect((await get(page, `/api${draft}`)).head.payload.data.recipe_version_id).toBe(next.version.id)
  await page.getByRole('button', { name: '초안 보관', exact: true }).click(); await page.getByRole('button', { name: '초안 보관 확정' }).click(); await expect(page.getByText('초안을 보관했습니다. 이전 입력은 이력에 남아 있습니다.', { exact: true })).toBeVisible(); expect((await get(page, '/api/sales')).items).toHaveLength(0)
  await start(page, archived.item.id); await expect(page.getByLabel('판매 메뉴', { exact: true }).locator(`option[value="${archived.item.id}"]`)).toHaveCount(1)
})

test('independent identical sales remain separate and exact aggregate exceeds a single amount bound', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 직접 참조'), entry = await recipe(page, '가상 큰 판매가', '9000000000000', [line(raw, '0'), line(raw, '0')]); await apiSale(page, entry, '1'); await apiSale(page, entry, '1'); const list = await get(page, '/api/sales'); expect(list.items).toHaveLength(2); expect(list.known_revenue).toBe('18000000000000'); expect(list.missing_revenue_count).toBe(0)
  await visit(page, '/sales'); await expect(page.locator('.sale-totals')).toContainText('18,000,000,000,000원'); await expect(page.locator('.sale-list > li')).toHaveCount(2); await page.getByLabel('판매한 메뉴 조회').selectOption(entry.item.id); await expect(page.locator('.sale-list > li')).toHaveCount(2); await visit(page, `/sales?material=${raw.id}`); await expect(page.locator('.sale-list > li')).toHaveCount(0)
  await visit(page, `/sales/${list.items[0].id}`); await expect(page.locator('.sale-cost-snapshot .recipe-cost-rows > li')).toHaveCount(2); await expect(page.getByRole('region', { name: '실사 소모 미확정', exact: true }).locator('li')).toHaveCount(1)
})
