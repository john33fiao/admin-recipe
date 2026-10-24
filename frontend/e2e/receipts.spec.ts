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
    let output = ''; let errors = ''
    const timer = setTimeout(() => reject(new Error(`receipt fixture start timed out: ${errors}`)), 15_000)
    server.stderr!.on('data', data => { errors += String(data) })
    server.once('exit', code => { clearTimeout(timer); reject(new Error(`receipt fixture exited ${code}: ${errors}`)) })
    server.stdout!.on('data', data => { output += String(data); const match = output.match(/http:\/\/127\.0\.0\.1:\d+\/#token=[a-f0-9]+/); if (match) { clearTimeout(timer); resolveUrl(match[0]) } })
  })
}
test.beforeEach(async () => {
  directory = mkdtempSync(join(tmpdir(), 'sunchang-receipts-'))
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
async function newMaterial(page: Page, name = '가상 입고 재료') {
  await visit(page, '/')
  await page.getByLabel('재료 이름', { exact: false }).fill(name)
  await page.getByRole('button', { name: '재료 등록', exact: true }).click()
  const path = (await page.getByRole('link', { name: '등록한 재료 보기' }).getAttribute('href'))!
  return path
}
async function saveNew(page: Page) {
  await page.getByRole('button', { name: '초안 임시 저장', exact: true }).click()
  const link = page.getByRole('link', { name: '저장한 초안 열기', exact: true })
  await expect(link).toBeVisible()
  const path = (await link.getAttribute('href'))!
  await link.click()
  await expect(page).toHaveURL(new URL(path, launchUrl).toString())
  await expect(page.getByRole('button', { name: '확정 전 미리보기', exact: true })).toBeVisible()
  return path
}
async function preview(page: Page) { await page.getByRole('button', { name: '확정 전 미리보기', exact: true }).click(); await expect(page.getByRole('button', { name: '이 내용으로 확정', exact: true })).toBeEnabled() }
async function confirmReceipt(page: Page) {
  await page.getByRole('button', { name: '이 내용으로 확정', exact: true }).click()
  const link = page.getByRole('link', { name: '실제 입고 기록 보기', exact: true })
  await expect(link).toBeVisible()
  const path = (await link.getAttribute('href'))!
  await link.click()
  await expect(page.getByRole('heading', { name: '확정한 실제 입고', exact: true })).toBeVisible()
  return path
}
async function startReceipt(page: Page, material: string) {
  await visit(page, material)
  await page.getByRole('link', { name: '직접 입고 초안', exact: true }).click()
  await expect(page.getByRole('region', { name: '실제 입고 초안 입력', exact: true })).toBeVisible()
  const reset = page.getByRole('button', { name: '새 입고 초안 작성', exact: true })
  if (await reset.isVisible()) await reset.click()
  await expect(page.getByRole('heading', { name: '실제 입고 초안', exact: true })).toBeVisible()
}
async function fillReceipt(page: Page, quantity = '1', amount = '10000', date = '2024-02-29') {
  await page.getByLabel('실제 입고 수량', { exact: true }).fill(quantity)
  await page.getByLabel('입고 단위', { exact: true }).fill('kg')
  await page.getByLabel('실제 구입금액 (원)', { exact: true }).fill(amount)
  await page.getByLabel('실제 입고일', { exact: true }).fill(date)
}
async function normalRestart(page: Page) {
  await page.goto('about:blank')
  let timer: ReturnType<typeof setTimeout> | undefined
  const [code, signal] = await Promise.race([closed, new Promise<never>((_, reject) => { timer = setTimeout(() => reject(new Error('receipt normal shutdown timed out')), 180_000) })]).finally(() => clearTimeout(timer))
  expect(code).toBe(0); expect(signal).toBeNull()
  await launch(); await open(page)
}
async function order(page: Page, material: string, revise = false) {
  await visit(page, '/orders')
  await page.getByRole('link', { name: '주문 초안 작성', exact: true }).click()
  await page.getByLabel('주문 재료', { exact: true }).selectOption(material.split('/').pop()!)
  await page.getByLabel('주문 수량', { exact: true }).fill('10')
  await page.getByLabel('주문 단위', { exact: true }).fill('kg')
  await page.getByLabel('예상 금액 (원, 선택)', { exact: true }).fill('999')
  await saveNew(page); await preview(page)
  await page.getByRole('button', { name: '이 내용으로 확정', exact: true }).click()
  await page.getByRole('link', { name: '주문과 이력 보기', exact: true }).click()
  await expect(page.getByRole('heading', { name: '현재 주문 내용', exact: true })).toBeVisible()
  if (revise) {
    await page.getByRole('link', { name: '주문 수정 초안', exact: true }).click()
    await page.getByLabel('주문 수량', { exact: true }).fill('12')
    await saveNew(page); await preview(page)
    await page.getByRole('button', { name: '이 내용으로 확정', exact: true }).click()
    await page.getByRole('link', { name: '주문과 이력 보기', exact: true }).click()
    await expect(page.getByRole('heading', { name: '현재 주문 내용', exact: true })).toBeVisible()
  }
  return new URL(page.url()).pathname
}

test('direct unfinished receipt survives a normal restart and produces exact holding amounts without a fake observation', async ({ page }) => {
  test.setTimeout(260_000)
  await open(page)
  const material = await newMaterial(page, '가상 입고 — 손질 채소와 긴 품목 이름')
  await visit(page, material)
  await expect(page.getByRole('region', { name: '보유 근거', exact: true })).toContainText('아직 정산 기준 없음')
  await startReceipt(page, material)
  const draft = await saveNew(page)
  await normalRestart(page); await visit(page, draft)
  await expect(page.getByLabel('실제 입고 수량', { exact: true })).toHaveValue('')
  await expect(page.getByLabel('실제 구입금액 (원)', { exact: true })).toHaveValue('')
  await fillReceipt(page, '1', '', '9999-01-01')
  await page.getByRole('button', { name: '수정 내용 임시 저장', exact: true }).click()
  await expect(page.getByText('수정 내용을 임시 저장했습니다.', { exact: true })).toBeVisible()
  await page.getByRole('button', { name: '확정 전 미리보기', exact: true }).click()
  await expect(page.getByLabel('실제 구입금액 (원)', { exact: true })).toBeFocused()
  await expect(page.getByRole('button', { name: '이 내용으로 확정', exact: true })).toBeDisabled()
  await page.getByLabel('실제 구입금액 (원)', { exact: true }).fill('10000')
  await page.getByRole('button', { name: '수정 내용 임시 저장', exact: true }).click()
  await expect(page.getByText('수정 내용을 임시 저장했습니다.', { exact: true })).toBeVisible()
  await page.getByRole('button', { name: '확정 전 미리보기', exact: true }).click()
  await expect(page.getByLabel('실제 입고일', { exact: true })).toBeFocused()
  await page.getByLabel('실제 입고일', { exact: true }).fill('2024-02-29')
  await page.getByRole('button', { name: '수정 내용 임시 저장', exact: true }).click()
  await expect(page.getByText('수정 내용을 임시 저장했습니다.', { exact: true })).toBeVisible()
  await preview(page); await confirmReceipt(page)
  await expect(page.locator('.receipt-detail')).toContainText('10,000원')
  await startReceipt(page, material)
  await fillReceipt(page, '2', '40000')
  await saveNew(page); await preview(page)
  await expect(page.locator('.receipt-preview')).toContainText('3,000g · 50,000원')
  await expect(page.locator('.receipt-preview')).toContainText('아직 확인한 수량 없음')
  for (const [width, height] of [[1440, 900], [768, 1024], [390, 844]] as const) {
    await page.setViewportSize({ width, height })
    for (const theme of ['light', 'dark'] as const) {
      if (await page.locator('html').getAttribute('data-theme') !== theme) await page.getByRole('button', { name: theme === 'dark' ? '다크 모드로 전환' : '라이트 모드로 전환' }).click()
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true)
      await page.screenshot({ path: test.info().outputPath(`receipt-preview-${width}-${theme}.png`), fullPage: true })
    }
  }
  await confirmReceipt(page)
  await page.getByRole('link', { name: '재료와 보유 근거 확인', exact: true }).click()
  await expect(page.locator('.holding-fields')).toContainText('3,000g')
  await expect(page.locator('.holding-fields')).toContainText('50,000원')
  await expect(page.locator('.detail-fields')).toContainText('50,000원 / 3,000g')
  await expect(page.getByRole('region', { name: '가격 변경 이력', exact: true }).locator('li')).toHaveCount(0)
  await expect(page.locator('.holding-changes li')).toHaveCount(1)
  await page.screenshot({ path: test.info().outputPath('holding-390-dark.png'), fullPage: true })
  await page.getByRole('link', { name: '이 재료의 입고 기록', exact: true }).click()
  await expect(page.locator('.receipt-list li')).toHaveCount(2)
  const zero = await newMaterial(page, '가상 실제 0원 입고')
  await startReceipt(page, zero); await fillReceipt(page, '1', '0'); await saveNew(page); await preview(page); await confirmReceipt(page)
  await page.getByRole('link', { name: '재료와 보유 근거 확인', exact: true }).click()
  await expect(page.locator('.detail-fields')).toContainText('0원 / 1,000g')
})

test('a revised order receives actual quantity once and links original current and received quantities', async ({ page }) => {
  await open(page)
  const material = await newMaterial(page, '가상 주문 실측 입고')
  const orderPath = await order(page, material, true)
  await page.getByRole('link', { name: '입고 초안 작성', exact: true }).click()
  await expect(page.getByLabel('실제 입고 수량', { exact: true })).toHaveValue('')
  await expect(page.getByLabel('실제 구입금액 (원)', { exact: true })).toHaveValue('')
  await fillReceipt(page, '11.8', '20000'); await saveNew(page); await preview(page)
  await expect(page.locator('.receipt-order-comparison')).toContainText('10,000g')
  await expect(page.locator('.receipt-order-comparison')).toContainText('12,000g')
  await expect(page.locator('.receipt-order-comparison')).toContainText('11,800g')
  await expect(page.locator('.receipt-order-comparison')).toContainText('-200g')
  const receipt = await confirmReceipt(page)
  await expect(page.locator('.receipt-order-closure')).toContainText('-200g')
  await page.getByRole('link', { name: '완료한 주문과 이력 보기', exact: true }).click()
  await expect(page).toHaveURL(new URL(orderPath, launchUrl).toString())
  await expect(page.locator('.order-current')).toContainText('12kg')
  await expect(page.locator('.order-history li')).toHaveCount(3)
  await expect(page.locator('.order-history li').first()).toContainText('입고 확정')
  await expect(page.getByRole('link', { name: '입고 초안 작성', exact: true })).not.toBeVisible()
  await page.locator('.order-history li').first().getByRole('link').click()
  await page.getByRole('link', { name: '실제 입고 기록 보기', exact: true }).click()
  await expect(page).toHaveURL(new URL(receipt, launchUrl).toString())
  await visit(page, '/orders'); await page.getByLabel('주문 상태').selectOption('received'); await expect(page.locator('.order-list li')).toHaveCount(1)
})

test('included or unconfirmed stock remains a persisted blocked draft after normal restart', async ({ page }) => {
  test.setTimeout(250_000)
  await open(page)
  const material = await newMaterial(page, '가상 포함 확인 입고')
  const orderPath = await order(page, material)
  await page.getByRole('link', { name: '입고 초안 작성', exact: true }).click()
  await fillReceipt(page)
  await page.getByLabel('재고 반영 구분', { exact: true }).selectOption('needs_reconciliation')
  await page.getByLabel('메모 (선택)', { exact: true }).fill('가상 확인 메모: 이전 확인에 포함했는지 점검')
  const draft = await saveNew(page)
  await normalRestart(page); await visit(page, draft)
  await expect(page.getByLabel('재고 반영 구분', { exact: true })).toHaveValue('needs_reconciliation')
  await expect(page.getByLabel('메모 (선택)', { exact: true })).toHaveValue('가상 확인 메모: 이전 확인에 포함했는지 점검')
  const before = await (await page.request.get(new URL('/api/state', launchUrl).toString())).json()
  await page.getByRole('button', { name: '확정 전 미리보기', exact: true }).click()
  await expect(page.getByRole('button', { name: '이 내용으로 확정', exact: true })).toBeDisabled()
  await expect(page.locator('.business-issues')).toContainText('확정할 수 없습니다')
  expect(await (await page.request.get(new URL('/api/state', launchUrl).toString())).json()).toEqual(before)
  const data = await (await page.request.get(new URL(`/api${draft}`, launchUrl).toString())).json()
  expect(data.head.payload.data.stock_effect).toEqual({ kind: 'needs_reconciliation', basis_id: null, included_quantity: '' })
  await visit(page, orderPath); await expect(page.getByRole('link', { name: '입고 초안 작성', exact: true })).toBeVisible(); await expect(page.locator('.order-history li')).toHaveCount(1)
  await visit(page, material); await expect(page.locator('.holding-fields')).toContainText('아직 정산 기준 없음')
})

test('receipt confirmation loss and JSON failure preserve the exact request without duplicate holding', async ({ page }) => {
  await open(page)
  const material = await newMaterial(page, '가상 입고 재시도')
  await startReceipt(page, material); await fillReceipt(page); await saveNew(page); await preview(page)
  const requests: unknown[] = []
  await page.route('**/api/business/*/confirm', async route => { requests.push(route.request().postDataJSON()); if (requests.length === 1) { await route.fetch(); await route.abort('failed') } else await route.continue() })
  await page.getByRole('button', { name: '이 내용으로 확정', exact: true }).click()
  await expect(page.getByRole('button', { name: '같은 업무 요청 재시도', exact: true })).toBeVisible()
  await page.reload(); await page.getByRole('button', { name: '같은 업무 요청 재시도', exact: true }).click()
  await expect(page.getByRole('link', { name: '실제 입고 기록 보기', exact: true })).toBeVisible()
  expect(requests).toHaveLength(2); expect(requests[1]).toEqual(requests[0])
  await page.unroute('**/api/business/*/confirm')
  await startReceipt(page, material); await fillReceipt(page, '2', '40000'); await saveNew(page); await preview(page)
  rmSync(join(directory, 'db', 'snapshots'), { recursive: true }); writeFileSync(join(directory, 'db', 'snapshots'), 'blocked by receipt verification fixture')
  await page.getByRole('button', { name: '이 내용으로 확정', exact: true }).click(); await expect(page.getByRole('button', { name: '같은 업무 요청 재시도', exact: true })).toBeVisible()
  rmSync(join(directory, 'db', 'snapshots'))
  await page.reload(); await page.getByRole('button', { name: '같은 업무 요청 재시도', exact: true }).click(); await expect(page.getByRole('link', { name: '실제 입고 기록 보기', exact: true })).toBeVisible()
  await visit(page, material); await expect(page.locator('.holding-fields')).toContainText('3,000g'); await expect(page.locator('.holding-fields')).toContainText('50,000원')
  await visit(page, '/receipts'); await expect(page.locator('.receipt-list li')).toHaveCount(2)
})

test('stale receipt preview needs a new explicit review of current holding', async ({ page, context }) => {
  await open(page)
  const material = await newMaterial(page, '가상 보유 변경 충돌')
  await startReceipt(page, material); await fillReceipt(page, '1', '10000'); await saveNew(page); await preview(page)
  const other = await context.newPage(); await startReceipt(other, material); await fillReceipt(other, '2', '40000'); await saveNew(other); await preview(other); await confirmReceipt(other)
  await page.getByRole('button', { name: '이 내용으로 확정', exact: true }).click()
  await expect(page.getByRole('alert')).toContainText('처리하지 않았습니다')
  await page.getByRole('button', { name: '최신 기록 확인 후 입력 검토', exact: true }).click()
  await preview(page); await expect(page.locator('.receipt-preview')).toContainText('3,000g · 50,000원'); await confirmReceipt(page)
  await other.close()
})

test('nested source edits and saving across navigation preserve the transmitted receipt', async ({ page }) => {
  await open(page)
  const first = await newMaterial(page, '가상 첫 입고 대상')
  const second = await newMaterial(page, '가상 바꾼 입고 대상')
  await startReceipt(page, first); await fillReceipt(page); await saveNew(page); await preview(page)
  await page.getByLabel('입고 재료', { exact: true }).selectOption(second.split('/').pop()!)
  await expect(page.locator('.receipt-preview')).not.toBeVisible()
  await expect(page.getByRole('button', { name: '확정 전 미리보기', exact: true })).toBeDisabled()
  let release: (() => void) | undefined
  const gate = new Promise<void>(resolve => { release = resolve })
  let body: { payload: { data: { source: { material_id: string } } } } | undefined
  await page.route('**/api/business/*/draft-revisions', async route => { body = route.request().postDataJSON(); await gate; await route.continue() })
  await page.getByRole('button', { name: '수정 내용 임시 저장', exact: true }).click()
  await expect(page.getByLabel('입고 방식', { exact: true })).toBeDisabled()
  await page.getByRole('navigation', { name: '주요 메뉴' }).getByRole('link', { name: '작업 시작', exact: true }).click(); await expect(page.getByRole('heading', { name: '작업 시작', exact: true })).toBeVisible()
  await page.goBack(); release!()
  await expect(page.getByText('수정 내용을 임시 저장했습니다.', { exact: true })).toBeVisible()
  expect(body?.payload.data.source.material_id).toBe(second.split('/').pop())
  await expect(page.getByLabel('입고 재료', { exact: true })).toHaveValue(second.split('/').pop()!)
  await preview(page); await confirmReceipt(page)
  await expect(page.getByRole('heading', { name: '가상 바꾼 입고 대상', exact: true })).toBeVisible()
})

test('a late direct reference cannot overwrite an explicitly selected order after navigation', async ({ page }) => {
  await open(page)
  const first = await newMaterial(page, '가상 늦은 직접 참조')
  const second = await newMaterial(page, '가상 명시 주문 참조')
  const orderPath = await order(page, second)
  await startReceipt(page, first)
  let release: (() => void) | undefined
  let gated = false
  const gate = new Promise<void>(resolve => { release = resolve })
  await page.route(`**/api${first}`, async route => { if (!gated) { gated = true; await gate }; await route.continue() })
  await page.getByRole('button', { name: '현재 입고 대상 정보 적용', exact: true }).click()
  await page.getByRole('navigation', { name: '주요 메뉴' }).getByRole('link', { name: '작업 시작', exact: true }).click(); await expect(page.getByRole('heading', { name: '작업 시작', exact: true })).toBeVisible()
  await page.goBack()
  await page.getByLabel('입고 방식', { exact: true }).selectOption('order')
  await page.getByLabel('연결할 주문', { exact: true }).selectOption(orderPath.split('/').pop()!)
  release!()
  await fillReceipt(page)
  const draft = await saveNew(page)
  const stored = await (await page.request.get(new URL(`/api${draft}`, launchUrl).toString())).json()
  expect(stored.head.payload.data.source.kind).toBe('order')
  expect(stored.head.payload.data.source.order_id).toBe(orderPath.split('/').pop())
  expect(Object.hasOwn(stored.head.payload.data.source, 'material_id')).toBe(false)
  await preview(page); await confirmReceipt(page)
  await expect(page.getByRole('heading', { name: '가상 명시 주문 참조', exact: true })).toBeVisible()
})
