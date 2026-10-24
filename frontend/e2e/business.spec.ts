import { test, expect, type Page } from '@playwright/test'
import { spawn, type ChildProcess } from 'node:child_process'
import { once } from 'node:events'
import { cpSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

const binaryName = process.platform === 'win32' ? 'sunchang-hansang.exe' : 'sunchang-hansang'
let directory: string
let binary: string
let server: ChildProcess
let closed: Promise<[number | null, NodeJS.Signals | null]>
let launchUrl: string
async function launch() {
  server = spawn(binary, ['--no-browser'], { cwd: directory, stdio: ['ignore', 'pipe', 'pipe'] })
  closed = once(server, 'close') as Promise<[number | null, NodeJS.Signals | null]>
  launchUrl = await new Promise<string>((resolveUrl, reject) => {
    let output = ''
    let errors = ''
    const timer = setTimeout(() => reject(new Error(`business fixture start timed out: ${errors}`)), 15_000)
    server.stderr!.on('data', data => { errors += String(data) })
    server.once('exit', code => { clearTimeout(timer); reject(new Error(`business fixture exited ${code}: ${errors}`)) })
    server.stdout!.on('data', data => { output += String(data); const match = output.match(/http:\/\/127\.0\.0\.1:\d+\/#token=[a-f0-9]+/); if (match) { clearTimeout(timer); resolveUrl(match[0]) } })
  })
}
test.beforeEach(async () => {
  directory = mkdtempSync(join(tmpdir(), 'sunchang-business-'))
  binary = join(directory, binaryName)
  cpSync(resolve('..', 'target', 'release', binaryName), binary)
  await launch()
})
test.afterEach(async () => {
  if (server && server.exitCode === null && server.signalCode === null) server.kill()
  if (closed) await closed
  if (directory) rmSync(directory, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 })
})
async function open(page: Page) { await page.goto(launchUrl); await expect(page.getByLabel('재료 이름', { exact: false })).toBeEnabled() }
async function visit(page: Page, path: string) { await page.goto(new URL(path, launchUrl).toString()) }
async function normalRestart(page: Page) {
  await page.goto('about:blank')
  let timer: ReturnType<typeof setTimeout> | undefined
  const [code, signal] = await Promise.race([closed, new Promise<never>((_, reject) => { timer = setTimeout(() => reject(new Error('business normal shutdown timed out')), 180_000) })]).finally(() => clearTimeout(timer))
  expect(code).toBe(0)
  expect(signal).toBeNull()
  await launch()
  await open(page)
}
async function newPrice(page: Page, name: string) {
  await visit(page, '/')
  await page.getByLabel('재료 이름', { exact: false }).fill(name)
  await page.getByRole('button', { name: '재료 등록', exact: true }).click()
  const materialPath = (await page.getByRole('link', { name: '등록한 재료 보기' }).getAttribute('href'))!
  await page.getByRole('link', { name: '등록한 재료 보기' }).click()
  await page.getByRole('link', { name: '가격 초안 작성', exact: true }).click()
  await expect(page.getByRole('heading', { name: '사전 가격 초안 작성', exact: true })).toBeVisible()
  return materialPath
}
async function saveNew(page: Page, quantity = '1', amount = '10') {
  await page.getByLabel('가격 기준 수량', { exact: true }).fill(quantity)
  await page.getByLabel('기준 금액 (원)', { exact: true }).fill(amount)
  await page.getByRole('button', { name: '초안 임시 저장', exact: true }).click()
  await expect(page.getByRole('link', { name: '저장한 초안 열기' })).toBeVisible()
  const path = (await page.getByRole('link', { name: '저장한 초안 열기' }).getAttribute('href'))!
  await page.getByRole('link', { name: '저장한 초안 열기' }).click()
  return path
}
async function preview(page: Page) {
  await page.getByRole('button', { name: '확정 전 미리보기', exact: true }).click()
  await expect(page.getByRole('button', { name: '이 내용으로 확정' })).toBeEnabled()
}
async function confirm(page: Page) {
  await page.getByRole('button', { name: '이 내용으로 확정' }).click()
  await expect(page.getByText('가격을 확정했습니다.', { exact: false })).toBeVisible()
}

test('normal UI stores unfinished input, survives two process restarts and confirms a correction chain', async ({ page }) => {
  test.setTimeout(420_000)
  await open(page)
  const materialPath = await newPrice(page, '가상 초안 재료')
  const businessPath = await saveNew(page, '', '0')
  await page.getByRole('navigation', { name: '주요 메뉴' }).getByRole('link', { name: '업무 기록', exact: true }).click()
  await expect(page.locator('.business-list')).toContainText('입력 필요')
  await visit(page, materialPath)
  await expect(page.locator('.detail-fields')).toContainText('가격 없음')
  await normalRestart(page)
  await page.getByRole('navigation', { name: '주요 메뉴' }).getByRole('link', { name: '업무 기록', exact: true }).click()
  await page.locator('.business-list').getByRole('link').click()
  await expect(page).toHaveURL(new URL(businessPath, launchUrl).toString())
  await expect(page.getByLabel('가격 기준 수량', { exact: true })).toHaveValue('')
  await expect(page.getByLabel('기준 금액 (원)', { exact: true })).toHaveValue('0')
  await page.getByLabel('가격 기준 수량', { exact: true }).fill('1')
  await page.getByLabel('가격 기준 단위', { exact: true }).fill('kg')
  await page.getByRole('button', { name: '수정 내용 임시 저장' }).click()
  await expect(page.getByText('수정 내용을 임시 저장했습니다.', { exact: true })).toBeVisible()
  await preview(page)
  await expect(page.locator('.preview-result')).toHaveText('1,000 g 기준 0원')
  await confirm(page)
  await expect(page.locator('.business-chain li')).toHaveCount(1)
  await page.getByRole('link', { name: '재료와 가격 확인' }).click()
  await expect(page.locator('.detail-fields')).toContainText('0원 / 1,000g')
  await normalRestart(page)
  await visit(page, businessPath)
  await expect(page.getByRole('region', { name: '입력과 상태 이력' }).locator('li')).toHaveCount(3)
  await page.getByRole('link', { name: '이 가격 정정하기' }).click()
  await page.getByLabel('기준 금액 (원)', { exact: true }).fill('10')
  await page.getByLabel('정정 이유').fill('가상 정정 이유: 확인한 가격 반영')
  await page.getByRole('button', { name: '초안 임시 저장', exact: true }).click()
  const correctionPath = (await page.getByRole('link', { name: '저장한 초안 열기' }).getAttribute('href'))!
  await page.getByRole('link', { name: '저장한 초안 열기' }).click()
  await preview(page)
  await expect(page.locator('.business-price-comparison')).toContainText('정정 대상 가격')
  await expect(page.locator('.preview-result')).toHaveText('1,000 g 기준 10원')
  for (const [width, height] of [[1440, 900], [768, 1024], [390, 844]] as const) {
    await page.setViewportSize({ width, height })
    for (const theme of ['light', 'dark'] as const) {
      if (await page.locator('html').getAttribute('data-theme') !== theme) await page.getByRole('button', { name: theme === 'dark' ? '다크 모드로 전환' : '라이트 모드로 전환' }).click()
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true)
      await page.screenshot({ path: test.info().outputPath(`business-correction-${width}-${theme}.png`), fullPage: true })
    }
  }
  await confirm(page)
  await expect(page.locator('.business-chain li')).toHaveCount(2)
  await page.locator('.business-chain').getByRole('link', { name: /원본 확정/ }).click()
  await expect(page).toHaveURL(new URL(businessPath, launchUrl).toString())
  await expect(page.getByRole('region', { name: '저장된 업무 내용' })).toContainText('0원')
  await page.getByRole('link', { name: '현재 유효한 확정 열기' }).click()
  await expect(page).toHaveURL(new URL(correctionPath, launchUrl).toString())
  await page.getByRole('link', { name: '재료와 가격 확인' }).click()
  await expect(page.locator('.detail-fields')).toContainText('10원 / 1,000g')
})

test('a stale preview requires another review and shows the current independent price', async ({ page, context }) => {
  await open(page)
  const materialPath = await newPrice(page, '가상 미리보기 충돌')
  await saveNew(page, '1', '30')
  await preview(page)
  const other = await context.newPage()
  await visit(other, materialPath)
  await other.getByLabel('기준 금액 (원)').fill('20')
  await other.getByRole('button', { name: '가격 등록', exact: true }).click()
  await expect(other.getByText('사전 가격을 등록했습니다.', { exact: true })).toBeVisible()
  await page.getByRole('button', { name: '이 내용으로 확정' }).click()
  await expect(page.getByRole('alert')).toContainText('처리하지 않았습니다')
  await page.getByRole('button', { name: '최신 기록 확인 후 입력 검토' }).click()
  await preview(page)
  await expect(page.locator('.business-price-comparison')).toContainText('20원 / 1g')
  await confirm(page)
  await expect(page.locator('.business-chain li')).toHaveCount(1)
  await other.close()
})

test('another tab confirmation keeps unsaved local input separate from the confirmed record', async ({ page, context }) => {
  await open(page)
  await newPrice(page, '가상 확정 입력 구분')
  const path = await saveNew(page, '1', '10')
  await page.getByLabel('기준 금액 (원)').fill('100')
  const other = await context.newPage()
  await visit(other, path)
  await preview(other)
  await confirm(other)
  await page.getByRole('button', { name: '수정 내용 임시 저장' }).click()
  await expect(page.getByRole('alert')).toContainText('처리하지 않았습니다')
  await page.getByRole('button', { name: '최신 기록 확인 후 입력 검토' }).click()
  await expect(page.getByRole('region', { name: '저장된 업무 내용' })).toContainText('10원')
  await expect(page.getByRole('region', { name: '이 화면에 남은 미저장 입력' })).toContainText('100원')
  await expect(page.getByRole('button', { name: '수정 내용 임시 저장' })).not.toBeVisible()
  await page.getByRole('navigation', { name: '주요 메뉴' }).getByRole('link', { name: '작업 시작' }).click()
  await expect(page.getByRole('heading', { name: '작업 시작', exact: true })).toBeVisible()
  await page.goBack()
  await expect(page.getByRole('region', { name: '저장된 업무 내용' })).toContainText('10원')
  await expect(page.getByRole('region', { name: '이 화면에 남은 미저장 입력' })).toContainText('100원')
  await other.close()
})

test('response loss and JSON failure preserve the exact confirmation request across reload', async ({ page }) => {
  await open(page)
  await newPrice(page, '가상 확정 재시도')
  await saveNew(page, '1', '0')
  await preview(page)
  const requests: unknown[] = []
  await page.route('**/api/business/*/confirm', async route => {
    requests.push(route.request().postDataJSON())
    if (requests.length === 1) { await route.fetch(); await route.abort('failed') } else await route.continue()
  })
  await page.getByRole('button', { name: '이 내용으로 확정' }).click()
  await expect(page.getByRole('button', { name: '같은 업무 요청 재시도' })).toBeVisible()
  await page.reload()
  await page.getByRole('button', { name: '같은 업무 요청 재시도' }).click()
  await expect(page.getByText('가격을 확정했습니다.', { exact: false })).toBeVisible()
  expect(requests).toHaveLength(2)
  expect(requests[1]).toEqual(requests[0])
  await expect(page.locator('.business-chain li')).toHaveCount(1)
  await page.unroute('**/api/business/*/confirm')
  await newPrice(page, '가상 JSON 확정 재시도')
  await saveNew(page, '1', '5')
  await preview(page)
  rmSync(join(directory, 'db', 'snapshots'), { recursive: true })
  writeFileSync(join(directory, 'db', 'snapshots'), 'blocked by business verification fixture')
  await page.getByRole('button', { name: '이 내용으로 확정' }).click()
  await expect(page.getByRole('button', { name: '같은 업무 요청 재시도' })).toBeVisible()
  rmSync(join(directory, 'db', 'snapshots'))
  await page.reload()
  await page.getByRole('button', { name: '같은 업무 요청 재시도' }).click()
  await expect(page.getByText('가격을 확정했습니다.', { exact: false })).toBeVisible()
  await expect(page.locator('.business-chain li')).toHaveCount(1)
})

test('saving during navigation does not resurrect a local draft and archived input remains readable', async ({ page }) => {
  await open(page)
  await newPrice(page, '가상 이동 중 초안')
  await page.getByLabel('기준 금액 (원)').fill('0')
  let release: (() => void) | undefined
  const gate = new Promise<void>(resolve => { release = resolve })
  await page.route('**/api/business/drafts', async route => { await gate; await route.continue() })
  await page.getByRole('button', { name: '초안 임시 저장', exact: true }).click()
  await expect(page.getByLabel('기준 금액 (원)')).toHaveAttribute('readonly', '')
  await page.getByRole('navigation', { name: '주요 메뉴' }).getByRole('link', { name: '작업 시작' }).click()
  await expect(page.getByRole('heading', { name: '작업 시작', exact: true })).toBeVisible()
  await page.goBack()
  release!()
  await expect(page.getByRole('link', { name: '저장한 초안 열기' })).toBeVisible()
  const path = (await page.getByRole('link', { name: '저장한 초안 열기' }).getAttribute('href'))!
  await page.reload()
  await expect(page.getByLabel('기준 금액 (원)')).toHaveValue('')
  await expect(page.locator('.business-list li')).toHaveCount(1)
  await visit(page, path)
  await page.getByRole('button', { name: '확정 전 미리보기', exact: true }).click()
  await expect(page.getByLabel('가격 기준 수량', { exact: true })).toBeFocused()
  await page.getByLabel('기준 금액 (원)').fill('0'.repeat(81))
  await page.getByRole('button', { name: '수정 내용 임시 저장' }).click()
  await expect(page.getByLabel('기준 금액 (원)')).toBeFocused()
  await page.getByRole('button', { name: '초안 보관', exact: true }).click()
  await page.getByRole('button', { name: '초안 보관 확정' }).click()
  await expect(page.getByText('초안을 보관했습니다.', { exact: false })).toBeVisible()
  await page.getByRole('navigation', { name: '주요 메뉴' }).getByRole('link', { name: '업무 기록', exact: true }).click()
  await expect(page.getByText('이 상태의 업무가 없습니다')).toBeVisible()
  await page.getByLabel('업무 상태').selectOption('archived')
  await page.locator('.business-list').getByRole('link').click()
  await expect(page.getByRole('region', { name: '입력과 상태 이력' }).locator('li')).toHaveCount(2)
})

test('request conflicts never generate an automatic replacement confirmation', async ({ page }) => {
  await open(page)
  await newPrice(page, '가상 확정 ID 충돌')
  await saveNew(page)
  await preview(page)
  let calls = 0
  await page.route('**/api/business/*/confirm', async route => { calls += 1; await route.fulfill({ status: 409, contentType: 'application/json', body: JSON.stringify({ code: 'request_conflict', error: 'synthetic request conflict' }) }) })
  await page.getByRole('button', { name: '이 내용으로 확정' }).click()
  await expect(page.getByRole('alert')).toContainText('다른 처리 이력')
  await expect(page.getByRole('button', { name: '같은 업무 요청 재시도' })).not.toBeVisible()
  await expect(page.getByRole('button', { name: '최신 기록 확인 후 입력 검토' })).not.toBeVisible()
  expect(calls).toBe(1)
})

test('new choices hide archived materials and a late material refresh cannot replace a new selection', async ({ page }) => {
  await open(page)
  const activePath = await newPrice(page, '가상 사용 중 선택')
  const archivedPath = await newPrice(page, '가상 보관한 선택')
  await visit(page, archivedPath)
  await page.getByRole('button', { name: '보관하기', exact: true }).click()
  await page.getByRole('button', { name: '보관 확정', exact: true }).click()
  await expect(page.getByText('재료를 보관했습니다.', { exact: false })).toBeVisible()
  await visit(page, '/business')
  await expect(page.getByLabel('재료', { exact: true }).locator('option').filter({ hasText: '가상 보관한 선택' })).toHaveCount(0)
  const archivedId = archivedPath.split('/').pop()!
  const activeId = activePath.split('/').pop()!
  await visit(page, `/business?material=${archivedId}`)
  await expect(page.getByLabel('재료', { exact: true })).toHaveValue(archivedId)
  let release: (() => void) | undefined
  const gate = new Promise<void>(resolve => { release = resolve })
  let gated = false
  await page.route(`**/api/materials/${archivedId}`, async route => { if (!gated) { gated = true; await gate }; await route.continue() })
  await page.getByRole('button', { name: '현재 재료 정보 적용' }).click()
  await page.getByRole('navigation', { name: '주요 메뉴' }).getByRole('link', { name: '작업 시작' }).click()
  await expect(page.getByRole('heading', { name: '작업 시작', exact: true })).toBeVisible()
  await page.goBack()
  await page.getByLabel('재료', { exact: true }).selectOption(activeId)
  release!()
  let sent: { payload: { data: { material_id: string; material_version_id: string } } } | undefined
  await page.route('**/api/business/drafts', async route => { sent = route.request().postDataJSON(); await route.continue() })
  await saveNew(page)
  const active = await (await page.request.get(new URL(`/api${activePath}`, launchUrl).toString())).json()
  expect(sent?.payload.data.material_id).toBe(activeId)
  expect(sent?.payload.data.material_version_id).toBe(active.version_id)
})
