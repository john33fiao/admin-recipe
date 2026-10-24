import { test, expect, type Page } from '@playwright/test'
import { spawn, type ChildProcess } from 'node:child_process'
import { once } from 'node:events'
import { mkdtempSync, cpSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

const binaryName = process.platform === 'win32' ? 'sunchang-hansang.exe' : 'sunchang-hansang'
let directory: string
let server: ChildProcess
let closed: Promise<unknown>
let launchUrl: string

test.beforeAll(async () => {
  directory = mkdtempSync(join(tmpdir(), 'sunchang-ui-foundation-'))
  const binary = join(directory, binaryName)
  cpSync(resolve('..', 'target', 'release', binaryName), binary)
  server = spawn(binary, ['--no-browser'], { cwd: directory, stdio: ['ignore', 'pipe', 'pipe'] })
  closed = once(server, 'close')
  launchUrl = await new Promise<string>((resolveUrl, reject) => {
    let output = ''
    let errors = ''
    const timer = setTimeout(() => reject(new Error(`UI fixture start timed out: ${errors}`)), 15_000)
    server.stderr!.on('data', data => { errors += String(data) })
    server.once('exit', code => { clearTimeout(timer); reject(new Error(`UI fixture exited ${code}: ${errors}`)) })
    server.stdout!.on('data', data => {
      output += String(data)
      const match = output.match(/http:\/\/127\.0\.0\.1:\d+\/#token=[a-f0-9]+/)
      if (match) { clearTimeout(timer); resolveUrl(match[0]) }
    })
  })
})

test.afterAll(async () => {
  // UI coverage uses forced fixture cleanup. Lifecycle graceful shutdown is
  // independently verified by local-app.spec.ts, not asserted by this file.
  if (server && server.exitCode === null && server.signalCode === null) server.kill()
  if (closed) await closed
  if (directory) rmSync(directory, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 })
})

async function open(page: Page) {
  await page.goto(launchUrl)
  await expect(page.getByLabel('재료 이름')).toBeEnabled()
}

async function expectFits(page: Page) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true)
  await expect(page.locator('main')).toBeVisible()
}

test('empty, loading, failure, validation and retry give a usable next action', async ({ page }) => {
  await open(page)
  await expect(page.getByRole('heading', { name: '첫 재료를 기록해 보세요' })).toBeVisible()
  await page.keyboard.press('Tab')
  await expect(page.getByRole('link', { name: '본문으로 건너뛰기' })).toBeFocused()
  await page.keyboard.press('Enter')
  await expect(page.locator('#main-content')).toBeFocused()
  await page.getByRole('button', { name: '재료 등록', exact: true }).click()
  await expect(page.getByLabel('재료 이름')).toBeFocused()
  await expect(page.getByLabel('재료 이름')).toHaveAttribute('aria-invalid', 'true')
  await expect(page.locator('#name-error')).toHaveText('재료 이름을 입력해 주세요.')

  let release: (() => void) | undefined
  const responseGate = new Promise<void>(resolve => { release = resolve })
  await page.route('**/api/materials', async route => {
    if (route.request().method() === 'GET') {
      await responseGate
      await route.fulfill({ status: 503, contentType: 'application/json', body: '{"error":"fixture unavailable"}' })
    } else await route.continue()
  })
  await page.getByRole('button', { name: '새로고침', exact: true }).click()
  await expect(page.getByRole('button', { name: '불러오는 중…' })).toBeDisabled()
  release!()
  await expect(page.getByRole('alert')).toContainText('재료 목록을 불러오지 못했습니다.')
  await page.unroute('**/api/materials')
  await page.getByRole('button', { name: '새로고침', exact: true }).click()
  await expect(page.getByRole('alert')).not.toBeVisible()
})

test('saving locks the payload and a failed tab draft write is visible', async ({ page }) => {
  await page.addInitScript(() => {
    const original = Storage.prototype.setItem
    Storage.prototype.setItem = function (key: string, value: string) {
      if (key === 'sunchang.draft-material.v1') throw new DOMException('fixture quota', 'QuotaExceededError')
      original.call(this, key, value)
    }
  })
  await open(page)
  await page.getByLabel('재료 이름').fill('가상 저장 중 재료')
  await expect(page.getByRole('alert')).toHaveText('입력 내용을 이 탭에 보관하지 못했습니다. 새로고침하면 사라질 수 있습니다.')
  let release: (() => void) | undefined
  const responseGate = new Promise<void>(resolve => { release = resolve })
  await page.route('**/api/materials', async route => {
    if (route.request().method() === 'POST') await responseGate
    await route.continue()
  })
  await page.getByRole('button', { name: '재료 등록', exact: true }).click()
  await expect(page.getByRole('button', { name: '등록 확인 중…' })).toBeDisabled()
  await expect(page.getByLabel('재료 이름')).toHaveAttribute('readonly', '')
  await expect(page.getByLabel('관리 단위')).toHaveAttribute('readonly', '')
  release!()
  await expect(page.getByText('재료를 등록했습니다.', { exact: false })).toBeVisible()
  await expect(page.getByLabel('재료 이름')).toHaveValue('')
  await expect(page.getByRole('alert')).not.toBeVisible()
})

test('unsubmitted input survives navigation, reload and connection recovery', async ({ page, context }) => {
  await open(page)
  await page.getByLabel('재료 이름').fill('가상 작성 중 재료')
  await page.getByLabel('관리 단위').fill('묶음')
  await page.getByRole('navigation', { name: '주요 메뉴' }).getByRole('link', { name: '작업 시작' }).click()
  await expect(page.locator('#main-content')).toBeFocused()
  await page.getByRole('navigation', { name: '주요 메뉴' }).getByRole('link', { name: '재료 관리' }).click()
  await expect(page.getByLabel('재료 이름')).toHaveValue('가상 작성 중 재료')
  await page.reload()
  await expect(page.getByLabel('재료 이름')).toHaveValue('가상 작성 중 재료')
  await expect(page.getByLabel('관리 단위')).toHaveValue('묶음')
  await expect(page.getByText('이 탭에서 작성하던 내용을 불러왔습니다.', { exact: true })).toBeVisible()
  await context.setOffline(true)
  await page.evaluate(() => window.dispatchEvent(new Event('app:reconnect')))
  await expect(page.getByLabel('재료 이름')).toBeDisabled()
  await expect(page.getByLabel('재료 이름')).toHaveValue('가상 작성 중 재료')
  await context.setOffline(false)
  await page.evaluate(() => window.dispatchEvent(new Event('online')))
  await expect(page.getByLabel('재료 이름')).toBeEnabled()
  await page.getByRole('button', { name: '재료 등록', exact: true }).click()
  await expect(page.getByText('재료를 등록했습니다.', { exact: false })).toBeVisible()
  await expect(page.getByLabel('재료 이름')).toBeFocused()
  await page.reload()
  await expect(page.getByLabel('재료 이름')).toHaveValue('')
})

test('a save completed after leaving the form clears its old tab draft', async ({ page }) => {
  await open(page)
  const name = '가상 화면 이동 중 저장'
  await page.getByLabel('재료 이름').fill(name)
  await page.getByLabel('관리 단위').fill('장')
  let release: (() => void) | undefined
  const responseGate = new Promise<void>(resolve => { release = resolve })
  await page.route('**/api/materials', async route => {
    if (route.request().method() === 'POST') await responseGate
    await route.continue()
  })
  await page.getByRole('button', { name: '재료 등록', exact: true }).click()
  await expect(page.getByRole('button', { name: '등록 확인 중…' })).toBeDisabled()
  await page.getByRole('navigation', { name: '주요 메뉴' }).getByRole('link', { name: '작업 시작' }).click()
  await expect(page.getByRole('heading', { name: '작업 시작', exact: true })).toBeVisible()
  await expect(page.getByLabel('재료 이름')).toHaveCount(0)
  release!()
  await expect.poll(() => page.evaluate(() => sessionStorage.getItem('sunchang.pending-material.v1'))).toBeNull()
  await page.goto(new URL('/', launchUrl).toString())
  await expect(page.getByLabel('재료 이름')).toHaveValue('')
  await expect(page.getByLabel('관리 단위')).toHaveValue('g')
  const stored = await (await page.request.get(new URL('/api/materials', launchUrl).toString())).json()
  expect(stored.filter((material: { name: string }) => material.name === name)).toHaveLength(1)
})

test('overview supports a direct visit and refresh', async ({ page }) => {
  await open(page)
  const response = await page.goto(new URL('/overview', launchUrl).toString())
  expect(response?.status()).toBe(200)
  await expect(page.getByRole('heading', { name: '작업 시작', exact: true })).toBeVisible()
  await page.reload()
  await expect(page.getByRole('heading', { name: '작업 시작', exact: true })).toBeVisible()
})

test('three widths and two themes keep overview, form, list and detail usable', async ({ page }) => {
  test.setTimeout(90_000)
  await open(page)
  const name = '가상 화면 검증용 긴 재료 이름 '.repeat(3).trim()
  await page.getByLabel('재료 이름').fill(name)
  await page.getByLabel('관리 단위').fill('가상 관리 단위')
  await page.getByRole('button', { name: '재료 등록', exact: true }).click()
  await expect(page.getByText('재료를 등록했습니다.', { exact: false })).toBeVisible()
  const detailPath = await page.getByRole('link', { name: '등록한 재료 보기' }).getAttribute('href')
  for (let index = 0; index < 12; index += 1) {
    const response = await page.request.post(new URL('/api/materials', launchUrl).toString(), {
      headers: { Origin: new URL(launchUrl).origin },
      data: { request_id: crypto.randomUUID(), name: `가상 긴 목록 재료 ${index + 1}`, unit: 'g' },
    })
    expect(response.ok()).toBe(true)
  }
  const stored = await (await page.request.get(new URL('/api/materials', launchUrl).toString())).json()

  for (const [width, height] of [[1440, 900], [768, 1024], [390, 844]] as const) {
    await page.setViewportSize({ width, height })
    for (const theme of ['light', 'dark'] as const) {
      await page.goto(new URL('/', launchUrl).toString())
      if (await page.locator('html').getAttribute('data-theme') !== theme) {
        await page.getByRole('button', { name: theme === 'dark' ? '다크 모드로 전환' : '라이트 모드로 전환' }).click()
      }
      await expect(page.locator('html')).toHaveAttribute('data-theme', theme)
      await expect(page.getByLabel('재료 이름')).toBeEnabled()
      await expect(page.getByRole('heading', { name: '등록한 재료', exact: true })).toBeVisible()
      await expectFits(page)
      expect(await page.evaluate(() => document.documentElement.scrollHeight > window.innerHeight)).toBe(true)
      await page.screenshot({ path: test.info().outputPath(`materials-${width}-${theme}.png`), fullPage: true })

      await page.getByRole('navigation', { name: '주요 메뉴' }).getByRole('link', { name: '작업 시작' }).click()
      await expect(page.getByRole('heading', { name: '작업 시작', exact: true })).toBeVisible()
      await expect(page.locator('.overview-count')).toHaveText(`${stored.length}개`)
      await expect(page.getByRole('navigation', { name: '주요 메뉴' }).getByRole('link', { name: '작업 시작' })).toHaveAttribute('aria-current', 'page')
      await expectFits(page)
      await page.screenshot({ path: test.info().outputPath(`overview-${width}-${theme}.png`), fullPage: true })

      await page.goto(new URL(detailPath!, launchUrl).toString())
      await expect(page.getByRole('heading', { name, exact: true })).toBeVisible()
      await expect(page.locator('.detail-fields').first().getByText('가상 관리 단위', { exact: true })).toBeVisible()
      await expectFits(page)
      await page.screenshot({ path: test.info().outputPath(`detail-${width}-${theme}.png`), fullPage: true })
    }
  }
})
