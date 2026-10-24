import { test, expect, type Page } from '@playwright/test'
import { spawn, type ChildProcess } from 'node:child_process'
import { once } from 'node:events'
import { cpSync, mkdtempSync, rmSync } from 'node:fs'
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
    let output = ''; let errors = ''
    const timer = setTimeout(() => reject(new Error(`order fixture start timed out: ${errors}`)), 15_000)
    server.stderr!.on('data', data => { errors += String(data) })
    server.once('exit', code => { clearTimeout(timer); reject(new Error(`order fixture exited ${code}: ${errors}`)) })
    server.stdout!.on('data', data => { output += String(data); const match = output.match(/http:\/\/127\.0\.0\.1:\d+\/#token=[a-f0-9]+/); if (match) { clearTimeout(timer); resolveUrl(match[0]) } })
  })
}
test.beforeEach(async () => {
  directory = mkdtempSync(join(tmpdir(), 'sunchang-orders-'))
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
async function newMaterial(page: Page, name = '가상 발주용 긴 재료 이름 — 국산 손질 채소') {
  await visit(page, '/')
  await page.getByLabel('재료 이름', { exact: false }).fill(name)
  await page.getByRole('button', { name: '재료 등록', exact: true }).click()
  const path = (await page.getByRole('link', { name: '등록한 재료 보기' }).getAttribute('href'))!
  return path
}
async function startOrder(page: Page, materialPath: string) {
  await visit(page, '/orders')
  await page.getByRole('link', { name: '주문 초안 작성', exact: true }).click()
  await page.getByLabel('주문 재료', { exact: true }).selectOption(materialPath.split('/').pop()!)
}
async function saveNew(page: Page) {
  await page.getByRole('button', { name: '초안 임시 저장', exact: true }).click()
  const link = page.getByRole('link', { name: '저장한 초안 열기' })
  await expect(link).toBeVisible()
  const path = (await link.getAttribute('href'))!
  await link.click()
  await expect(page).toHaveURL(new URL(path, launchUrl).toString())
  await expect(page.getByRole('button', { name: '확정 전 미리보기', exact: true })).toBeVisible()
  return path
}
async function preview(page: Page) { await page.getByRole('button', { name: '확정 전 미리보기', exact: true }).click(); await expect(page.getByRole('button', { name: '이 내용으로 확정' })).toBeEnabled() }
async function confirmedOrder(page: Page) {
  const link = page.getByRole('link', { name: '주문과 이력 보기', exact: true })
  await expect(link).toBeVisible()
  const path = (await link.getAttribute('href'))!
  await link.click()
  await expect(page.getByRole('heading', { name: '현재 주문 내용', exact: true })).toBeVisible()
  return path
}
async function confirm(page: Page) { await page.getByRole('button', { name: '이 내용으로 확정' }).click(); return confirmedOrder(page) }
async function placeOrder(page: Page, materialPath: string) {
  await startOrder(page, materialPath)
  await page.getByLabel('주문 수량', { exact: true }).fill('10')
  await page.getByLabel('주문 단위', { exact: true }).fill('kg')
  await saveNew(page); await preview(page)
  return confirm(page)
}
async function editOrder(page: Page, action: 'revise' | 'cancel', quantity = '12') {
  await page.getByRole('link', { name: action === 'revise' ? '주문 수정 초안' : '주문 취소 초안', exact: true }).click()
  await expect(page.getByRole('heading', { name: action === 'revise' ? '주문 수정 초안' : '주문 취소 초안', exact: true })).toBeVisible()
  if (action === 'revise') await page.getByLabel('주문 수량', { exact: true }).fill(quantity)
  return saveNew(page)
}

test('unfinished order survives a normal restart and place revise cancel retain history without changing price', async ({ page }) => {
  test.setTimeout(250_000)
  await open(page)
  const material = await newMaterial(page)
  await startOrder(page, material)
  await page.getByLabel('입고 예정일 (선택)', { exact: true }).fill('2028-0')
  const draft = await saveNew(page)
  await page.goto('about:blank')
  let timer: ReturnType<typeof setTimeout> | undefined
  const [code, signal] = await Promise.race([closed, new Promise<never>((_, reject) => { timer = setTimeout(() => reject(new Error('order normal shutdown timed out')), 180_000) })]).finally(() => clearTimeout(timer))
  expect(code).toBe(0); expect(signal).toBeNull()
  await launch(); await open(page); await visit(page, draft)
  await expect(page.getByLabel('입고 예정일 (선택)', { exact: true })).toHaveValue('2028-0')
  await page.getByLabel('주문 수량', { exact: true }).fill('10')
  await page.getByLabel('주문 단위', { exact: true }).fill('kg')
  await page.getByRole('button', { name: '수정 내용 임시 저장' }).click()
  await expect(page.getByText('수정 내용을 임시 저장했습니다.', { exact: true })).toBeVisible()
  await page.getByRole('button', { name: '확정 전 미리보기' }).click()
  await expect(page.getByLabel('입고 예정일 (선택)', { exact: true })).toBeFocused()
  await expect(page.getByRole('button', { name: '이 내용으로 확정' })).toBeDisabled()
  await page.getByLabel('입고 예정일 (선택)', { exact: true }).fill('2024-02-29')
  await page.getByLabel('거래처명 (선택)', { exact: true }).fill('가상 거래처 — 산지 공동 작업장과 포장 유통 협동 조합')
  await page.getByRole('button', { name: '수정 내용 임시 저장' }).click()
  await expect(page.getByText('수정 내용을 임시 저장했습니다.', { exact: true })).toBeVisible()
  await preview(page)
  await expect(page.locator('.business-preview')).toContainText('10,000g')
  for (const [width, height] of [[1440, 900], [768, 1024], [390, 844]] as const) {
    await page.setViewportSize({ width, height })
    for (const theme of ['light', 'dark'] as const) {
      if (await page.locator('html').getAttribute('data-theme') !== theme) await page.getByRole('button', { name: theme === 'dark' ? '다크 모드로 전환' : '라이트 모드로 전환' }).click()
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true)
      await page.screenshot({ path: test.info().outputPath(`order-preview-${width}-${theme}.png`), fullPage: true })
    }
  }
  const orderPath = await confirm(page)
  await expect(page.locator('.order-current')).toContainText('미입력')
  await page.reload()
  await expect(page.locator('.order-history li')).toHaveCount(1)
  await page.getByRole('link', { name: '주문 수정 초안', exact: true }).click()
  await page.getByLabel('주문 수량', { exact: true }).fill('12')
  await page.getByLabel('예상 금액 (원, 선택)', { exact: true }).fill('0')
  await expect(page.getByLabel('메모 (선택)', { exact: true })).toHaveValue('')
  await saveNew(page); await preview(page)
  await expect(page.locator('.preview-result')).toHaveText('10kg → 12kg')
  await confirm(page)
  await expect(page.locator('.order-current')).toContainText('0원')
  await expect(page.locator('.order-history li')).toHaveCount(2)
  await page.getByRole('link', { name: '주문 수정 초안', exact: true }).click()
  await expect(page.getByLabel('주문 수량', { exact: true })).toHaveValue('12')
  await expect(page.getByRole('button', { name: '초안 임시 저장', exact: true })).toBeEnabled()
  await visit(page, orderPath)
  await editOrder(page, 'cancel')
  await expect(page.getByLabel('메모 (선택)', { exact: true })).toHaveValue('')
  await preview(page); await confirm(page)
  await expect(page.locator('.order-history li')).toHaveCount(3)
  await expect(page.locator('.order-history li').last()).toContainText('10kg')
  await expect(page.getByRole('link', { name: '주문 수정 초안', exact: true })).not.toBeVisible()
  await visit(page, '/orders')
  await expect(page.getByText('미입고 주문이 없습니다', { exact: true })).toBeVisible()
  await page.getByLabel('주문 상태').selectOption('cancelled')
  await expect(page.locator('.order-list li')).toHaveCount(1)
  await visit(page, material)
  await expect(page.locator('.detail-fields')).toContainText('가격 없음')
})

test('lost confirmation responses preserve exact requests for place revise and cancel across reload', async ({ page }) => {
  await open(page)
  const material = await newMaterial(page, '가상 발주 재시도')
  await startOrder(page, material)
  await page.getByLabel('주문 수량', { exact: true }).fill('10')
  await saveNew(page)
  for (const action of ['place', 'revise', 'cancel'] as const) {
    if (action !== 'place') await editOrder(page, action)
    await preview(page)
    const requests: unknown[] = []
    await page.route('**/api/business/*/confirm', async route => { requests.push(route.request().postDataJSON()); if (requests.length === 1) { await route.fetch(); await route.abort('failed') } else await route.continue() })
    await page.getByRole('button', { name: '이 내용으로 확정' }).click()
    await expect(page.getByRole('button', { name: '같은 업무 요청 재시도' })).toBeVisible()
    await page.reload()
    await page.getByRole('button', { name: '같은 업무 요청 재시도' }).click()
    await confirmedOrder(page)
    expect(requests).toHaveLength(2); expect(requests[1]).toEqual(requests[0])
    await expect(page.locator('.order-history li')).toHaveCount({ place: 1, revise: 2, cancel: 3 }[action])
    await page.unroute('**/api/business/*/confirm')
  }
})

test('another order revision requires explicit reference review without replacing edited fields', async ({ page, context }) => {
  await open(page)
  const orderPath = await placeOrder(page, await newMaterial(page, '가상 주문 버전 충돌'))
  await editOrder(page, 'revise', '15'); await preview(page)
  const other = await context.newPage(); await visit(other, orderPath)
  await editOrder(other, 'revise', '12'); await preview(other); await confirm(other)
  await page.getByRole('button', { name: '이 내용으로 확정' }).click()
  await expect(page.getByRole('alert')).toContainText('처리하지 않았습니다')
  await page.getByRole('button', { name: '최신 기록 확인 후 입력 검토' }).click()
  await page.getByRole('button', { name: '확정 전 미리보기' }).click()
  await expect(page.getByRole('button', { name: '이 내용으로 확정' })).toBeDisabled()
  await page.getByRole('button', { name: '현재 주문 정보 적용' }).click()
  await expect(page.getByLabel('주문 수량', { exact: true })).toHaveValue('15')
  await page.getByRole('button', { name: '수정 내용 임시 저장' }).click()
  await expect(page.getByText('수정 내용을 임시 저장했습니다.', { exact: true })).toBeVisible()
  await preview(page)
  await expect(page.locator('.preview-result')).toHaveText('12kg → 15kg')
  await confirm(page)
  await expect(page.locator('.order-history li')).toHaveCount(3)
  await other.close()
})

test('a lost new revision draft keeps its original identity after another tab changes the order head', async ({ page, context }) => {
  await open(page)
  const orderPath = await placeOrder(page, await newMaterial(page, '가상 새 수정 초안 재시도'))
  await page.getByRole('link', { name: '주문 수정 초안', exact: true }).click()
  await page.getByLabel('주문 수량', { exact: true }).fill('15')
  const queryPath = new URL(page.url()).pathname + new URL(page.url()).search
  const requests: unknown[] = []
  await page.route('**/api/business/drafts', async route => { requests.push(route.request().postDataJSON()); if (requests.length === 1) { await route.fetch(); await route.abort('failed') } else await route.continue() })
  await page.getByRole('button', { name: '초안 임시 저장', exact: true }).click()
  await expect(page.getByRole('button', { name: '같은 업무 요청 재시도' })).toBeVisible()
  const other = await context.newPage(); await visit(other, orderPath)
  await editOrder(other, 'revise', '12'); await preview(other); await confirm(other)
  await visit(page, queryPath)
  await page.getByRole('button', { name: '같은 업무 요청 재시도' }).click()
  await expect(page.getByRole('link', { name: '저장한 초안 열기' })).toBeVisible()
  expect(requests).toHaveLength(2); expect(requests[1]).toEqual(requests[0])
  await page.getByRole('link', { name: '저장한 초안 열기' }).click()
  await expect(page.getByLabel('주문 수량', { exact: true })).toHaveValue('15')
  await visit(page, orderPath)
  await expect(page.locator('.order-history li')).toHaveCount(2)
  await other.close()
})

test('order material choices hide archived entries and late reference reads cannot overwrite a new selection', async ({ page }) => {
  await open(page)
  const first = await newMaterial(page, '가상 첫 발주 재료')
  const next = await newMaterial(page, '가상 다른 발주 재료')
  const archived = await newMaterial(page, '가상 보관한 발주 재료')
  await visit(page, archived)
  await page.getByRole('button', { name: '보관하기', exact: true }).click(); await page.getByRole('button', { name: '보관 확정', exact: true }).click()
  await expect(page.getByText('재료를 보관했습니다.', { exact: false })).toBeVisible()
  await startOrder(page, first)
  await expect(page.getByLabel('주문 재료', { exact: true }).locator('option').filter({ hasText: '가상 보관한 발주 재료' })).toHaveCount(0)
  let release: (() => void) | undefined
  const gate = new Promise<void>(resolve => { release = resolve })
  await page.route(`**/api${first}`, async route => { await gate; await route.continue() })
  await page.getByRole('button', { name: '현재 재료 정보 적용' }).click()
  await page.getByRole('navigation', { name: '주요 메뉴' }).getByRole('link', { name: '작업 시작' }).click()
  await expect(page.getByRole('heading', { name: '작업 시작', exact: true })).toBeVisible()
  await page.goBack()
  await page.getByLabel('주문 재료', { exact: true }).selectOption(next.split('/').pop()!)
  release!()
  await page.getByLabel('주문 수량', { exact: true }).fill('1')
  await saveNew(page)
  await expect(page.getByRole('heading', { name: '가상 다른 발주 재료', exact: true })).toBeVisible()
})
