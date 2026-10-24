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
    const timer = setTimeout(() => reject(new Error(`recipe fixture start timed out: ${errors}`)), 15_000)
    server.stderr!.on('data', data => { errors += String(data) })
    server.once('exit', code => { clearTimeout(timer); reject(new Error(`recipe fixture exited ${code}: ${errors}`)) })
    server.stdout!.on('data', data => { output += String(data); const match = output.match(/http:\/\/127\.0\.0\.1:\d+\/#token=[a-f0-9]+/); if (match) { clearTimeout(timer); resolveUrl(match[0]) } })
  })
}
test.beforeEach(async () => { directory = mkdtempSync(join(tmpdir(), 'sunchang-recipes-')); binary = join(directory, binaryName); cpSync(resolve('..', 'target', 'release', binaryName), binary); await launch() })
test.afterEach(async () => { if (server && server.exitCode === null && server.signalCode === null) server.kill(); if (closed) await closed; if (directory) { expect(resolve(directory).startsWith(resolve(tmpdir()))).toBe(true); rmSync(directory, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 }) } })
async function open(page: Page) { await page.goto(launchUrl); await expect(page.getByLabel('재료 이름', { exact: false })).toBeEnabled() }
async function visit(page: Page, path: string) { await page.goto(new URL(path, launchUrl).toString()) }
async function get(page: Page, path: string) { const response = await page.request.get(new URL(path, launchUrl).toString()); expect(response.ok()).toBe(true); return response.json() }
async function post(page: Page, path: string, data: unknown) { const response = await page.request.post(new URL(path, launchUrl).toString(), { headers: { Origin: new URL(launchUrl).origin }, data }); expect(response.status(), await response.text()).toBe(201); return response.json() }
async function meta(page: Page) { const state = await get(page, '/api/state'); return { request_id: randomUUID(), epoch: state.epoch, expected_revision: state.revision } }
async function material(page: Page, name: string, kind = 'raw', unit = 'g') { return post(page, '/api/materials', { ...await meta(page), name, unit, kind, description: '' }) }
async function price(page: Page, id: string, amount = '10', quantity = '1') { const item = await get(page, `/api/materials/${id}`); return post(page, `/api/materials/${id}/prices`, { ...await meta(page), expected_version_id: item.version_id, amount, quantity, unit: item.unit }) }
async function start(page: Page, name = '가상 배합 — 천천히 볶은 채소와 든든한 한 그릇') { await visit(page, '/recipes/new'); await expect(page.getByRole('region', { name: '레시피 편집', exact: true })).toBeVisible(); const reset = page.getByRole('button', { name: '새 레시피 작성', exact: true }); if (await reset.isVisible()) await reset.click(); await page.getByLabel('레시피 이름', { exact: true }).fill(name) }
async function add(page: Page, id: string, quantity = '0', note = '') { await page.getByRole('button', { name: '구성 재료 추가', exact: true }).click(); const index = await page.locator('.recipe-line').count(); await page.getByLabel(`구성 ${index} 재료`, { exact: true }).selectOption(id); await page.getByLabel(`구성 ${index} 기준 전체 수량`, { exact: true }).fill(quantity); if (note) await page.getByLabel(`구성 ${index} 메모 (선택)`, { exact: true }).fill(note) }
async function savedNew(page: Page) { const link = page.getByRole('link', { name: '저장한 레시피 보기', exact: true }); await expect(link).toBeVisible(); const path = (await link.getAttribute('href'))!; await link.click(); await expect(page).toHaveURL(new URL(path, launchUrl).toString()); await expect(page.getByRole('button', { name: '레시피 수정 저장', exact: true })).toBeEnabled(); return path }
async function saveNew(page: Page) { await page.getByRole('button', { name: '레시피 저장', exact: true }).click(); return savedNew(page) }
async function revise(page: Page) { await page.getByRole('button', { name: '레시피 수정 저장', exact: true }).click(); await expect(page.getByText('레시피를 저장했습니다. 이전 구성은 버전 이력에서 확인할 수 있습니다.', { exact: true })).toBeVisible(); await expect(page.getByRole('button', { name: '레시피 수정 저장', exact: true })).toBeEnabled() }
async function restart(page: Page) { await page.goto('about:blank'); let timer: ReturnType<typeof setTimeout> | undefined; const [code, signal] = await Promise.race([closed, new Promise<never>((_, reject) => { timer = setTimeout(() => reject(new Error('recipe normal shutdown timed out')), 180_000) })]).finally(() => clearTimeout(timer)); expect(code).toBe(0); expect(signal).toBeNull(); await launch(); await open(page) }

test('exact batch recipe with zero memo and current costs survives normal restart and six viewport themes', async ({ page }) => {
  test.setTimeout(250_000); await open(page)
  const raw = await material(page, '가상 채소 — 손질 전 긴 재료 이름'), unpriced = await material(page, '가상 양념 적당량', 'prepared'); await price(page, raw.id)
  await start(page); await page.getByLabel('기준 개수', { exact: true }).fill('3'); await page.getByLabel('판매가 (원, 선택)', { exact: true }).fill('1000'); await add(page, raw.id, '100', '씻은 뒤 사용'); await add(page, unpriced.id, '0', '적당량')
  await page.getByLabel('조리 설명 (선택)', { exact: true }).fill('가상 검증용 조리 설명'); await page.getByLabel('조리 공정 (선택)', { exact: true }).fill('씻기\n천천히 볶기'); await page.getByLabel('참고 조리 시간 (선택)', { exact: true }).fill('약 20분')
  const path = await saveNew(page), detail = await get(page, `/api${path}`)
  expect(detail.head.basis_count).toBe('3'); expect(detail.head.lines[0].input_quantity).toBe('100'); expect(detail.head.lines[1].note).toBe('적당량')
  await expect(page.locator('.recipe-cost')).toContainText('333.333333원'); await expect(page.locator('.recipe-cost')).toContainText('100 ÷ 3 g'); await expect(page.locator('.recipe-cost')).toContainText('33.33%'); await expect(page.locator('.recipe-cost')).toContainText('가격 없음')
  await restart(page); await visit(page, path); await expect(page.getByLabel('기준 개수', { exact: true })).toHaveValue('3'); await expect(page.getByLabel('구성 2 메모 (선택)', { exact: true })).toHaveValue('적당량')
  for (const [width, height] of [[1440, 900], [768, 1024], [390, 844]] as const) { await page.setViewportSize({ width, height }); for (const theme of ['light', 'dark'] as const) { if (await page.locator('html').getAttribute('data-theme') !== theme) await page.getByRole('button', { name: theme === 'dark' ? '다크 모드로 전환' : '라이트 모드로 전환' }).click(); expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true); await page.screenshot({ path: test.info().outputPath(`recipe-${width}-${theme}.png`), fullPage: true }) } }
  await page.locator('.recipe-history a').first().click(); await expect(page.getByText('저장된 과거 버전', { exact: false })).toBeVisible(); await expect(page.locator('.recipe-cost')).toContainText('당시 판매 원가와 다릅니다')
})

test('cost overflow does not lock recipe edits and zero absent prices remain distinct', async ({ page }) => {
  await open(page); const expensive = await material(page, '가상 상한 가격'), missing = await material(page, '가상 미등록 가격'); await price(page, expensive.id, '9000000000000', '0.000001')
  await start(page, '가상 계산 오류 수정'); await add(page, expensive.id, '1'); const path = await saveNew(page)
  await expect(page.locator('.recipe-cost').getByRole('alert')).toContainText('원가를 계산하지 못했습니다'); await expect(page.getByRole('button', { name: '레시피 수정 저장' })).toBeEnabled()
  await page.getByLabel('구성 1 기준 전체 수량', { exact: true }).fill('0'); await page.getByLabel('판매가 (원, 선택)', { exact: true }).fill('100'); await add(page, missing.id, '0', '가격 없어도 0 수량'); await revise(page)
  await expect(page.locator('.recipe-cost')).toContainText('0%'); await expect(page.locator('.recipe-cost')).toContainText('가격 없음')
  await page.getByLabel('구성 2 기준 전체 수량', { exact: true }).fill('1'); await revise(page); await expect(page.locator('.recipe-cost')).toContainText('미계산 1행')
  await page.getByLabel('판매가 (원, 선택)', { exact: true }).fill('0'); await revise(page); expect((await get(page, `/api${path}`)).head.selling_price).toBe('0')
  await page.getByLabel('판매가 (원, 선택)', { exact: true }).fill(''); await revise(page); expect((await get(page, `/api${path}`)).head.selling_price).toBeNull()
})

test('existing stocked reference and independent copy preserve the source and exact long-name suffix', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 비축 구성'), stock = await material(page, '가상 비축 소스', 'stocked', 'kg'); await price(page, raw.id)
  await visit(page, `/recipes/new?material=${stock.id}`); await page.getByLabel('참고 생산량 기록', { exact: true }).check(); await page.getByLabel('참고 생산량', { exact: true }).fill('2.5'); await add(page, raw.id, '100'); const stockPath = await saveNew(page)
  await expect(page.getByRole('heading', { name: '기준 배치 식재료비', exact: true })).toBeVisible(); expect((await get(page, `/api${stockPath}`)).head.reference_output.input_quantity).toBe('2.5')
  const name = '🍚'.repeat(96) + '가상메뉴'; await start(page, name); await add(page, stock.id, '1', '비축 직접 참조'); const original = await saveNew(page)
  await expect(page.locator('.recipe-cost')).toContainText('미계산'); await page.getByRole('button', { name: '현재 저장본 독립 복제', exact: true }).click(); const link = page.getByRole('link', { name: '복제한 레시피 보기', exact: true }); await expect(link).toBeVisible(); const copy = (await link.getAttribute('href'))!; await link.click(); await expect(page).toHaveURL(new URL(copy, launchUrl).toString())
  const copied = await get(page, `/api${copy}`); expect([...copied.item.name]).toHaveLength(100); expect(copied.item.name.endsWith(' (2)')).toBe(true); expect(copied.item.id).not.toBe(original.split('/').pop())
  await page.getByLabel('구성 1 메모 (선택)', { exact: true }).fill('복제에서만 수정'); await revise(page)
  expect((await get(page, `/api${original}`)).head.lines[0].note).toBe('비축 직접 참조')
  await visit(page, `/materials/${stock.id}`); await expect(page.getByRole('heading', { name: '이 재료를 사용하는 레시피', exact: true })).toBeVisible(); await expect(page.getByRole('link', { name: copied.item.name, exact: true })).toBeVisible()
})

test('recipe edits preserve past component versions and explicitly apply current archived references', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 이전 이름'); await start(page, '가상 과거 참조'); await add(page, raw.id, '0', '원본 메모'); const path = await saveNew(page), first = await get(page, `/api${path}`)
  const updated = await post(page, `/api/materials/${raw.id}/versions`, { ...await meta(page), expected_version_id: raw.version_id, name: '가상 현재 이름', unit: 'g', kind: 'prepared', description: '' })
  await post(page, `/api/materials/${raw.id}/archive`, { ...await meta(page), expected_version_id: updated.version_id }); await visit(page, path)
  await page.getByLabel('구성 1 메모 (선택)', { exact: true }).fill('메모만 수정'); await revise(page)
  const preserved = await get(page, `/api${path}`); expect(preserved.head.lines[0].material_version_id).toBe(raw.version_id); expect(preserved.head.lines[0].material_name).toBe('가상 이전 이름')
  await Promise.all([page.waitForResponse(response => new URL(response.url()).pathname === `/api/materials/${raw.id}`), page.getByRole('button', { name: '구성 1 현재 재료 정보 적용', exact: true }).click()]); await expect(page.getByRole('button', { name: '구성 1 현재 재료 정보 적용', exact: true })).toBeEnabled(); await expect(page.getByLabel('구성 1 메모 (선택)', { exact: true })).toHaveValue('메모만 수정'); await revise(page)
  const current = await get(page, `/api${path}`); expect(current.head.lines[0].material_version_id).not.toBe(raw.version_id)
  await visit(page, `${path}/versions/${first.head.id}`); await expect(page.getByText('원본 메모', { exact: true })).toBeVisible(); await expect(page.getByRole('link', { name: '가상 이전 이름의 현재 품목 정보', exact: true })).toBeVisible()
  await start(page, '가상 새 선택'); await page.getByRole('button', { name: '구성 재료 추가', exact: true }).click(); await expect(page.getByLabel('구성 1 재료', { exact: true }).locator('option').filter({ hasText: '가상 현재 이름' })).toHaveCount(0)
})

test('lost create copy replies and JSON failure replay exact recipe bodies once', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 요청 원문'); await start(page, '가상 요청 복구'); await add(page, raw.id, '0', '요청 메모')
  const created: unknown[] = []
  await page.route('**/api/recipes', async route => { if (route.request().method() !== 'POST') return route.continue(); created.push(route.request().postDataJSON()); if (created.length === 1) { await route.fetch(); await route.abort('failed') } else await route.continue() })
  await page.getByRole('button', { name: '레시피 저장', exact: true }).click(); await expect(page.getByRole('button', { name: '같은 레시피 요청 재시도' })).toBeVisible(); await page.reload(); await page.getByRole('button', { name: '같은 레시피 요청 재시도' }).click(); const path = await savedNew(page); expect(created[1]).toEqual(created[0]); await page.unroute('**/api/recipes')
  const snapshotPath = resolve(directory, 'db', 'snapshots'); expect(snapshotPath.startsWith(resolve(directory))).toBe(true); rmSync(snapshotPath, { recursive: true }); writeFileSync(snapshotPath, 'blocked by recipe fixture')
  await page.getByLabel('조리 공정 (선택)', { exact: true }).fill('JSON 복구 후 한 번'); await page.getByRole('button', { name: '레시피 수정 저장' }).click(); await expect(page.getByRole('button', { name: '같은 레시피 요청 재시도' })).toBeVisible(); rmSync(snapshotPath); await page.reload(); await page.getByRole('button', { name: '같은 레시피 요청 재시도' }).click(); await expect(page.getByText('레시피를 저장했습니다.', { exact: false })).toBeVisible(); expect((await get(page, `/api${path}`)).versions).toHaveLength(2)
  const copied: unknown[] = []; await page.route('**/api/recipes/*/copies', async route => { copied.push(route.request().postDataJSON()); if (copied.length === 1) { await route.fetch(); await route.abort('failed') } else await route.continue() })
  await page.getByRole('button', { name: '현재 저장본 독립 복제' }).click(); await expect(page.getByRole('button', { name: '같은 레시피 요청 재시도' })).toBeVisible(); await page.reload(); await page.getByRole('button', { name: '같은 레시피 요청 재시도' }).click(); await expect(page.getByRole('link', { name: '복제한 레시피 보기' })).toBeVisible(); expect(copied[1]).toEqual(copied[0]); expect((await get(page, '/api/recipes')).items).toHaveLength(2)
})

test('late component reads cannot overwrite a new selection and save completion survives navigation', async ({ page }) => {
  await open(page); const first = await material(page, '가상 첫 재료'), next = await material(page, '가상 다른 재료'); await start(page, '가상 늦은 조회'); await add(page, first.id, '0'); const path = await saveNew(page)
  let release: () => void = () => {}; const gate = new Promise<void>(resolveGate => { release = resolveGate })
  await page.route(`**/api/materials/${first.id}`, async route => { await gate; await route.continue() })
  await page.getByRole('button', { name: '구성 1 현재 재료 정보 적용' }).click(); await page.getByRole('navigation', { name: '주요 메뉴' }).getByRole('link', { name: '작업 시작' }).click(); await expect(page.getByRole('heading', { name: '작업 시작', exact: true })).toBeVisible(); await page.goBack(); await page.getByLabel('구성 1 재료', { exact: true }).selectOption(next.id)
  const lateResponse = page.waitForResponse(response => new URL(response.url()).pathname === `/api/materials/${first.id}`)
  release(); await lateResponse; await page.unroute(`**/api/materials/${first.id}`)
  let finish: () => void = () => {}; const savingGate = new Promise<void>(resolveGate => { finish = resolveGate }); let captured: any
  await page.route('**/api/recipes/*/versions', async route => { captured = route.request().postDataJSON(); const response = await route.fetch(); await savingGate; await route.fulfill({ response }) })
  await page.getByLabel('구성 1 메모 (선택)', { exact: true }).fill('탐색 후 저장'); await page.getByRole('button', { name: '레시피 수정 저장' }).click(); await page.getByRole('navigation', { name: '주요 메뉴' }).getByRole('link', { name: '작업 시작' }).click(); await expect(page.getByRole('heading', { name: '작업 시작', exact: true })).toBeVisible(); await page.goBack(); await expect(page.getByRole('button', { name: '레시피 수정 저장' })).toBeDisabled(); finish()
  await expect(page.getByRole('button', { name: '레시피 수정 저장' })).toBeEnabled(); expect(captured.data.lines[0].material_id).toBe(next.id); expect((await get(page, `/api${path}`)).head.lines[0].note).toBe('탐색 후 저장')
})

test('stale and oversized recipe requests require explicit review while cycle inputs remain visible', async ({ page, context }) => {
  await open(page); const raw = await material(page, '가상 충돌 재료'); await start(page, '가상 충돌 배합'); await add(page, raw.id, '0'); const path = await saveNew(page)
  const other = await context.newPage(); await visit(other, path); await page.getByLabel('구성 1 메모 (선택)', { exact: true }).fill('내 입력 보존'); await other.getByLabel('조리 설명 (선택)', { exact: true }).fill('다른 탭 저장'); await revise(other)
  await page.getByRole('button', { name: '레시피 수정 저장' }).click(); await expect(page.getByRole('alert')).toContainText('이 요청은 저장하지 않았습니다'); await page.getByRole('button', { name: '최신 저장본 확인 후 입력 검토' }).click(); await expect(page.getByLabel('구성 1 메모 (선택)', { exact: true })).toHaveValue('내 입력 보존'); await revise(page); await other.close()
  await page.route('**/api/recipes/*/versions', route => route.fulfill({ status: 413, contentType: 'application/json', body: JSON.stringify({ error: '입력 크기를 줄여 주세요.', code: 'validation' }) }))
  await page.getByLabel('조리 공정 (선택)', { exact: true }).fill('크기 거절 뒤 입력'); await page.getByRole('button', { name: '레시피 수정 저장' }).click(); await expect(page.getByRole('alert')).toContainText('입력 크기를 줄여'); await expect(page.getByRole('button', { name: '같은 레시피 요청 재시도' })).not.toBeVisible(); await page.unroute('**/api/recipes/*/versions'); await page.getByRole('button', { name: '최신 저장본 확인 후 입력 검토' }).click(); await expect(page.getByLabel('조리 공정 (선택)', { exact: true })).toHaveValue('크기 거절 뒤 입력'); await revise(page)
  await page.getByLabel('구성 1 재료', { exact: true }).selectOption(path.split('/').pop()!); await page.getByRole('button', { name: '레시피 수정 저장' }).click(); await expect(page.getByRole('alert')).toBeVisible(); await expect(page.getByLabel('구성 1 기준 전체 수량', { exact: true })).toHaveValue('0'); expect((await get(page, `/api${path}`)).head.lines[0].material_id).toBe(raw.id)
})
