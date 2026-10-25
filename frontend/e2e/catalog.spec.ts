import { test, expect, type Page } from '@playwright/test'
import { spawn, type ChildProcess } from 'node:child_process'
import { once } from 'node:events'
import { cpSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from 'node:fs'
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
    const timer = setTimeout(() => reject(new Error(`catalog fixture start timed out: ${errors}`)), 15_000)
    server.stderr!.on('data', data => { errors += String(data) })
    server.once('exit', code => { clearTimeout(timer); reject(new Error(`catalog fixture exited ${code}: ${errors}`)) })
    server.stdout!.on('data', data => {
      output += String(data)
      const match = output.match(/http:\/\/127\.0\.0\.1:\d+\/#token=[a-f0-9]+/)
      if (match) { clearTimeout(timer); resolveUrl(match[0]) }
    })
  })
}
test.beforeEach(async () => {
  directory = mkdtempSync(join(tmpdir(), 'sunchang-catalog-'))
  binary = join(directory, binaryName)
  cpSync(resolve('..', 'target', 'release', binaryName), binary)
  await launch()
})
test.afterEach(async () => {
  if (server && server.exitCode === null && server.signalCode === null) server.kill()
  if (closed) await closed
  if (directory) rmSync(directory, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 })
})
async function open(page: Page) { await page.goto(launchUrl); await expect(page.getByLabel('재료 이름', { exact: false }).first()).toBeEnabled() }
async function register(page: Page, name: string, kind = 'raw', unit = 'g') {
  await page.getByLabel('재료 이름', { exact: false }).fill(name)
  await page.getByLabel('관리 단위', { exact: false }).fill(unit)
  await page.getByLabel('재료 종류').selectOption(kind)
  await page.getByRole('button', { name: '재료 등록', exact: true }).click()
  await expect(page.getByText('재료를 등록했습니다.', { exact: false })).toBeVisible()
  return (await page.getByRole('link', { name: '등록한 재료 보기' }).getAttribute('href'))!
}
async function visit(page: Page, path: string) { await page.goto(new URL(path, launchUrl).toString()) }
async function price(page: Page, amount: string, quantity = '100', unit = 'g') {
  await page.getByLabel('기준 금액 (원)').fill(amount)
  await page.getByLabel('가격 기준 수량', { exact: true }).fill(quantity)
  await page.getByLabel('가격 기준 단위').fill(unit)
  await page.getByRole('button', { name: '가격 등록', exact: true }).click()
  await expect(page.getByText('사전 가격을 등록했습니다.', { exact: true })).toBeVisible()
}
async function rows(page: Page) { return (await (await page.request.get(new URL('/api/materials?archive=all', launchUrl).toString())).json()) as Array<{ id: string; name: string }> }

test('catalog zero prices, independent IDs, versions and archive survive normal restart', async ({ page }) => {
  test.setTimeout(210_000)
  await open(page)
  const detailPath = await register(page, '가상 같은 이름 재료')
  await register(page, '가상 같은 이름 재료', 'stocked', 'kg')
  await expect(page.locator('.material-list li')).toHaveCount(2)
  await page.getByLabel('이름으로 찾기').fill('  없는 재료  ')
  await page.getByRole('button', { name: '검색', exact: true }).click()
  await expect(page.getByText('조건에 맞는 재료가 없습니다')).toBeVisible()
  await page.getByRole('navigation', { name: '주요 메뉴' }).getByRole('link', { name: '작업 시작' }).click()
  await expect(page.locator('.overview-count')).toHaveText('2개')
  await visit(page, detailPath)
  await expect(page.getByRole('heading', { name: '가상 같은 이름 재료', exact: true })).toBeVisible()
  await expect(page.locator('.detail-fields')).toContainText('가격 없음')
  await price(page, '0')
  await expect(page.locator('.detail-fields')).toContainText('0원 / 100g')
  await price(page, '1200', '200')
  await expect(page.getByRole('region', { name: '가격 변경 이력' }).locator('li')).toHaveCount(2)
  await page.getByLabel('재료 이름', { exact: true }).fill('가상 변경한 긴 재료 이름 '.repeat(3).trim())
  await page.getByLabel('관리 단위', { exact: true }).fill('kg')
  await page.getByLabel('재료 종류').selectOption('prepared')
  await page.getByLabel('설명', { exact: true }).fill('가상 설명 첫 줄\n둘째 줄')
  await page.getByRole('button', { name: '재료 정보 저장' }).click()
  await expect(page.getByText('재료 정보를 저장했습니다.', { exact: true })).toBeVisible()
  await expect(page.getByRole('region', { name: '정보 변경 이력' }).locator('li')).toHaveCount(2)
  await expect(page.getByRole('region', { name: '정보 변경 이력' })).toContainText('원물 · g')
  await expect(page.locator('.detail-fields')).toContainText('1,200원 / 200g')

  for (const [width, height] of [[1440, 900], [768, 1024], [390, 844]] as const) {
    await page.setViewportSize({ width, height })
    for (const theme of ['light', 'dark'] as const) {
      if (await page.locator('html').getAttribute('data-theme') !== theme) await page.getByRole('button', { name: theme === 'dark' ? '다크 모드로 전환' : '라이트 모드로 전환' }).click()
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true)
      await expect(page.getByRole('button', { name: '가격 등록', exact: true })).toBeEnabled()
      await page.screenshot({ path: test.info().outputPath(`catalog-detail-${width}-${theme}.png`), fullPage: true })
    }
  }
  await page.getByRole('button', { name: '보관하기', exact: true }).click()
  await page.getByRole('button', { name: '보관 확정', exact: true }).click()
  await expect(page.getByText('재료를 보관했습니다.', { exact: false })).toBeVisible()
  await visit(page, '/')
  await expect(page.locator('.material-list li')).toHaveCount(1)
  await page.getByLabel('조회 범위').selectOption('archived')
  await expect(page.locator('.material-list li')).toHaveCount(1)
  await expect(page.locator('.material-list')).toContainText('보관됨')
  await page.getByLabel('조회 범위').selectOption('all')
  await expect(page.locator('.material-list li')).toHaveCount(2)
  await visit(page, '/overview')
  await expect(page.getByText('전체 품목(보관 포함)', { exact: true })).toBeVisible()
  await expect(page.locator('.overview-count')).toHaveText('2개')

  await page.goto('about:blank')
  let timer: ReturnType<typeof setTimeout> | undefined
  const [code, signal] = await Promise.race([closed, new Promise<never>((_, reject) => { timer = setTimeout(() => reject(new Error('normal catalog shutdown timed out')), 180_000) })]).finally(() => clearTimeout(timer))
  expect(code).toBe(0)
  expect(signal).toBeNull()
  await launch()
  await open(page)
  await visit(page, detailPath)
  await expect(page.locator('.detail-fields')).toContainText('보관됨')
  await expect(page.getByRole('region', { name: '가격 변경 이력' }).locator('li')).toHaveCount(2)
  await expect(page.getByRole('region', { name: '정보 변경 이력' }).locator('li')).toHaveCount(3)
})

test('price response loss and JSON failure replay one immutable mutation after refresh', async ({ page }) => {
  await open(page)
  const detailPath = await register(page, '가상 가격 재시도')
  await visit(page, detailPath)
  const attempts: unknown[] = []
  let interrupted = false
  await page.route('**/api/materials/*/prices', async route => {
    attempts.push(route.request().postDataJSON())
    if (!interrupted) { interrupted = true; await route.fetch(); await route.abort('failed') } else await route.continue()
  })
  await page.getByLabel('기준 금액 (원)').fill('0')
  await page.getByRole('button', { name: '가격 등록', exact: true }).click()
  await expect(page.getByRole('button', { name: '같은 변경 요청 재시도' })).toBeVisible()
  await page.reload()
  await expect(page.getByLabel('기준 금액 (원)')).toHaveValue('0')
  await page.getByRole('button', { name: '같은 변경 요청 재시도' }).click()
  await expect(page.getByText('사전 가격을 등록했습니다.', { exact: true })).toBeVisible()
  expect(attempts).toHaveLength(2)
  expect(attempts[1]).toEqual(attempts[0])
  await expect(page.getByRole('region', { name: '가격 변경 이력' }).locator('li')).toHaveCount(1)

  rmSync(join(directory, 'db', 'snapshots'), { recursive: true })
  writeFileSync(join(directory, 'db', 'snapshots'), 'blocked by catalog verification fixture')
  await page.getByLabel('설명', { exact: true }).fill('가상 기록 장애 수정')
  await page.getByRole('button', { name: '재료 정보 저장' }).click()
  await expect(page.getByRole('button', { name: '같은 변경 요청 재시도' })).toBeVisible()
  await expect(page.getByLabel('설명', { exact: true })).toHaveValue('가상 기록 장애 수정')
  rmSync(join(directory, 'db', 'snapshots'))
  await page.getByRole('button', { name: '같은 변경 요청 재시도' }).click()
  await expect(page.getByText('재료 정보를 저장했습니다.', { exact: true })).toBeVisible()
  await expect(page.getByRole('region', { name: '정보 변경 이력' }).locator('li')).toHaveCount(2)
  expect(readdirSync(join(directory, 'db', 'snapshots')).filter(name => name.endsWith('.json'))).toHaveLength(3)
})

test('stale state requires review, invalid dimensions preserve input and conflicts never auto-resubmit', async ({ page }) => {
  await open(page)
  const detailPath = await register(page, '가상 동시 수정')
  await visit(page, detailPath)
  const detail = await (await page.request.get(new URL(`/api${detailPath}`, launchUrl).toString())).json()
  await page.getByLabel('재료 이름', { exact: true }).fill('가상 내 수정')
  const outside = await page.request.post(new URL(`/api${detailPath}/versions`, launchUrl).toString(), { headers: { Origin: new URL(launchUrl).origin }, data: { request_id: crypto.randomUUID(), epoch: detail.epoch, expected_revision: detail.revision, expected_version_id: detail.version_id, name: '가상 다른 탭 수정', unit: 'g', kind: 'raw', description: '' } })
  expect(outside.ok()).toBe(true)
  const ids: string[] = []
  await page.route('**/api/materials/*/versions', async route => { ids.push(route.request().postDataJSON().request_id); await route.continue() })
  await page.getByRole('button', { name: '재료 정보 저장' }).click()
  await expect(page.getByRole('alert')).toContainText('이 요청은 저장하지 않았습니다')
  await page.getByRole('button', { name: '최신 상태 확인 후 입력 검토' }).click()
  await expect(page.getByRole('heading', { name: '가상 다른 탭 수정', exact: true })).toBeVisible()
  await expect(page.getByLabel('재료 이름', { exact: true })).toHaveValue('가상 내 수정')
  await page.getByRole('button', { name: '재료 정보 저장' }).click()
  await expect(page.getByText('재료 정보를 저장했습니다.', { exact: true })).toBeVisible()
  expect(ids).toHaveLength(2)
  expect(ids[1]).not.toBe(ids[0])
  await page.getByLabel('관리 단위', { exact: true }).fill('mL')
  await page.getByRole('button', { name: '재료 정보 저장' }).click()
  await expect(page.getByRole('alert')).toBeVisible()
  await expect(page.getByLabel('관리 단위', { exact: true })).toHaveValue('mL')
  await page.getByRole('button', { name: '최신 상태 확인 후 입력 검토' }).click()
  await page.getByLabel('관리 단위', { exact: true }).fill('g')
  await page.unroute('**/api/materials/*/versions')
  let conflictingRequests = 0
  await page.route('**/api/materials/*/versions', async route => { conflictingRequests += 1; await route.fulfill({ status: 409, contentType: 'application/json', body: JSON.stringify({ code: 'request_conflict', error: 'fixture reused request' }) }) })
  await page.getByRole('button', { name: '재료 정보 저장' }).click()
  await expect(page.getByRole('alert')).toContainText('다른 처리 이력')
  await expect(page.getByRole('button', { name: '같은 변경 요청 재시도' })).not.toBeVisible()
  await expect(page.getByRole('button', { name: '최신 상태 확인 후 입력 검토' })).not.toBeVisible()
  expect(conflictingRequests).toBe(1)
})

test('legacy pending body and literal wildcard search retain compatibility', async ({ page }) => {
  await open(page)
  const legacy = { request_id: crypto.randomUUID(), name: '가상 %_ 재료', unit: '장' }
  await page.evaluate(value => sessionStorage.setItem('sunchang.pending-material.v1', JSON.stringify(value)), legacy)
  await page.reload()
  let sent: unknown
  await page.route('**/api/materials', async route => { if (route.request().method() === 'POST') sent = route.request().postDataJSON(); await route.continue() })
  await page.getByRole('button', { name: '같은 요청 재시도' }).click()
  await expect(page.getByText('재료를 등록했습니다.', { exact: false })).toBeVisible()
  expect(sent).toEqual(legacy)
  await register(page, '가상 별도 재료')
  await page.getByLabel('이름으로 찾기').fill('  %_  ')
  await page.getByRole('button', { name: '검색', exact: true }).click()
  await expect(page.locator('.material-list li')).toHaveCount(1)
  await expect(page.locator('.material-list')).toContainText('종류 미지정')
  expect(await rows(page)).toHaveLength(2)
})

test('a pending price completed after leaving and reentering clears inputs and advances the next edit', async ({ page }) => {
  await open(page)
  const detailPath = await register(page, '가상 재진입 가격')
  await visit(page, detailPath)
  let release: (() => void) | undefined
  const responseGate = new Promise<void>(resolve => { release = resolve })
  await page.route('**/api/materials/*/prices', async route => { await responseGate; await route.continue() })
  await page.getByLabel('기준 금액 (원)').fill('1234')
  await page.getByRole('button', { name: '가격 등록', exact: true }).click()
  await expect(page.getByLabel('기준 금액 (원)')).toHaveAttribute('readonly', '')
  await page.getByRole('navigation', { name: '주요 메뉴' }).getByRole('link', { name: '작업 시작' }).click()
  await expect(page.getByRole('heading', { name: '작업 시작', exact: true })).toBeVisible()
  await expect(page.getByLabel('기준 금액 (원)')).toHaveCount(0)
  await page.getByRole('navigation', { name: '주요 메뉴' }).getByRole('link', { name: '재료 관리' }).click()
  await page.locator('.material-list').getByRole('link').click()
  await expect(page.getByLabel('기준 금액 (원)')).toHaveValue('1234')
  release!()
  await expect(page.getByText('사전 가격을 등록했습니다.', { exact: true })).toBeVisible()
  await expect(page.getByLabel('기준 금액 (원)')).toHaveValue('')
  await expect(page.getByRole('region', { name: '가격 변경 이력' }).locator('li')).toHaveCount(1)
  await page.getByLabel('설명', { exact: true }).fill('가격 저장 이후의 정보 수정')
  await page.getByRole('button', { name: '재료 정보 저장' }).click()
  await expect(page.getByText('재료 정보를 저장했습니다.', { exact: true })).toBeVisible()
  await expect(page.getByRole('alert')).not.toBeVisible()
  await expect(page.getByRole('region', { name: '정보 변경 이력' }).locator('li')).toHaveCount(2)
})
