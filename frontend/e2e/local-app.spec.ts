import { test, expect, type Page, type BrowserContext } from '@playwright/test'
import { spawn, type ChildProcess } from 'node:child_process'
import { once } from 'node:events'
import { mkdtempSync, cpSync, readFileSync, readdirSync, renameSync, mkdirSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

const binaryName = process.platform === 'win32' ? 'sunchang-hansang.exe' : 'sunchang-hansang'
const source = resolve('..', 'target', 'release', binaryName)
let directory: string
let binary: string
let child: ChildProcess
let launchUrl: string
let context: BrowserContext
const outputs = new WeakMap<ChildProcess, () => string>()
const exits = new WeakMap<ChildProcess, Promise<[number | null, NodeJS.Signals | null]>>()

async function launch(path = binary) {
  const proc = spawn(path, ['--no-browser'], { cwd: tmpdir(), env: { ...process.env, PATH: '' }, stdio: ['ignore','pipe','pipe'] })
  let output = ''
  let errors = ''
  exits.set(proc, once(proc, 'close') as Promise<[number | null, NodeJS.Signals | null]>)
  outputs.set(proc, () => output)
  proc.stderr!.on('data', chunk => { errors += String(chunk) })
  const url = await new Promise<string>((resolve, reject) => {
    const timer = setTimeout(() => { proc.kill(); reject(new Error(`서버 시작 지연: ${errors}`)) }, 10_000)
    proc.once('exit', code => { clearTimeout(timer); reject(new Error(`서버 종료 ${code}: ${errors}`)) })
    proc.stdout!.on('data', chunk => {
      output += String(chunk)
      const match = output.match(/http:\/\/127\.0\.0\.1:\d+\/#token=[a-f0-9]+/)
      if (match) { clearTimeout(timer); resolve(match[0]) }
    })
  })
  return { proc, url }
}
async function stop() {
  if (!child) return
  // Windows kill('SIGINT') is a forced termination. Disconnect the actual
  // pages and let the unchanged 120-second heartbeat grace stop the app.
  // Check an already exited child too; close waits for stdout to finish.
  await Promise.all(context.pages().filter(page => !page.isClosed()).map(page => page.goto('about:blank')))
  let timer: ReturnType<typeof setTimeout> | undefined
  const [code, signal] = await Promise.race([
    exits.get(child)!,
    new Promise<never>((_, reject) => { timer = setTimeout(() => reject(new Error('normal shutdown timed out')), 180_000) }),
  ]).finally(() => clearTimeout(timer))
  expect(code, 'normal shutdown must exit successfully').toBe(0)
  expect(signal).toBeNull()
  expect(outputs.get(child)!().includes('순창한상을 안전하게 종료했습니다.')).toBe(true)
}
async function open(page: Page) {
  await page.goto(launchUrl)
  await expect(page.getByLabel('재료 이름')).toBeEnabled()
  await expect(page).not.toHaveURL(/token=/)
}
async function register(page: Page, name: string, unit = 'g') {
  await page.getByLabel('재료 이름').fill(name)
  await page.getByLabel('관리 단위').fill(unit)
  await page.getByRole('button', { name:'재료 등록', exact:true }).click()
  await expect(page.getByText('재료를 등록했습니다.', { exact:false })).toBeVisible()
}

test.beforeEach(async ({ context: browserContext }) => {
  test.setTimeout(240_000)
  context = browserContext
  directory = mkdtempSync(join(tmpdir(), '순창 한상 검증 '))
  binary = join(directory, binaryName)
  cpSync(source, binary)
  const launched = await launch()
  child = launched.proc
  launchUrl = launched.url
})
test.afterEach(async () => {
  try {
    await stop()
  } finally {
    // A forced cleanup after a timeout must never turn that failed test green.
    if (child && child.exitCode === null && child.signalCode === null) {
      child.kill()
      await exits.get(child)
    }
    rmSync(directory, {recursive:true, force:true, maxRetries:5, retryDelay:200})
  }
})

test('register, detail reload, restart and stopped data move preserve IDs without runtime PATH', async ({ page }) => {
  await open(page)
  await register(page, '가상 채소', '장')
  await page.getByRole('link', {name:'등록한 재료 보기'}).click()
  await expect(page).toHaveURL(/\/materials\/[0-9a-f-]+$/)
  const detailPath = new URL(page.url()).pathname
  await expect(page.getByRole('heading', {name:'가상 채소', exact:true})).toBeVisible()
  await page.reload()
  await expect(page.getByRole('heading', {name:'가상 채소', exact:true})).toBeVisible()
  const snapshotFiles = readdirSync(join(directory, 'db', 'snapshots'))
  expect(snapshotFiles.filter(x => x.endsWith('.json'))).toHaveLength(1)
  const snapshot = JSON.parse(readFileSync(join(directory, 'db', 'snapshots', snapshotFiles[0]!), 'utf8'))
  expect(snapshot.state.materials[0].unit).toBe('장')
  expect(detailPath.endsWith(snapshot.state.materials[0].id)).toBeTruthy()
  await stop()
  // Replace only the stopped executable, preserving the complete db directory.
  cpSync(source, binary)
  const moved = join(directory, '옮긴 폴더')
  mkdirSync(moved)
  cpSync(binary, join(moved, binaryName))
  renameSync(join(directory, 'db'), join(moved, 'db'))
  const next = await launch(join(moved, binaryName)); child = next.proc; launchUrl = next.url
  await open(page)
  await page.goto(new URL(detailPath, launchUrl).toString())
  await expect(page.getByRole('heading', {name:'가상 채소', exact:true})).toBeVisible()
})

test('ambiguous saved response retries one operation across refresh', async ({ page }) => {
  await open(page)
  let intercepted = false
  await page.route('**/api/materials', async route => {
    if (route.request().method() === 'POST' && !intercepted) {
      intercepted = true
      await route.fetch()
      await route.abort('failed')
    } else await route.continue()
  })
  await page.getByLabel('재료 이름').fill('가상 응답 유실')
  await page.getByRole('button', {name:'재료 등록', exact:true}).click()
  await expect(page.getByRole('button', {name:'같은 요청 재시도'})).toBeVisible()
  await page.reload()
  await expect(page.getByRole('button', {name:'같은 요청 재시도'})).toBeEnabled()
  await page.getByRole('button', {name:'같은 요청 재시도'}).click()
  await expect(page.getByText('재료를 등록했습니다.', {exact:false})).toBeVisible()
  expect(readdirSync(join(directory, 'db', 'snapshots')).filter(x => x.endsWith('.json'))).toHaveLength(1)
})

test('JSON failure preserves input and recovers through the same request', async ({ page }) => {
  await open(page)
  rmSync(join(directory, 'db', 'snapshots'), {recursive:true})
  writeFileSync(join(directory, 'db', 'snapshots'), 'blocked by verification fixture')
  await page.getByLabel('재료 이름').fill('가상 기록 장애')
  await page.getByRole('button', {name:'재료 등록', exact:true}).click()
  await expect(page.getByRole('button', {name:'같은 요청 재시도'})).toBeVisible()
  await expect(page.getByLabel('재료 이름')).toHaveValue('가상 기록 장애')
  await expect(page.getByText('재료를 등록했습니다.', {exact:false})).not.toBeVisible()
  rmSync(join(directory, 'db', 'snapshots'))
  await page.getByRole('button', {name:'같은 요청 재시도'}).click()
  await expect(page.getByText('재료를 등록했습니다.', {exact:false})).toBeVisible()
  expect(readdirSync(join(directory, 'db', 'snapshots')).filter(x => x.endsWith('.json'))).toHaveLength(1)
})

test('multiple tabs, responsive dark view and explicit API/static errors', async ({ page, context }) => {
  await open(page)
  const other = await context.newPage()
  await other.goto(new URL('/', launchUrl).toString())
  await expect(other.getByLabel('재료 이름')).toBeEnabled()
  await page.close()
  await register(other, '가상 복수 탭')
  const markup = '<img src=x onerror="document.body.dataset.xss=1">'
  await register(other, markup)
  await other.getByRole('link', {name:'등록한 재료 보기'}).click()
  await expect(other.getByRole('heading', {name:markup, exact:true})).toBeVisible()
  expect(await other.evaluate(() => document.body.dataset.xss)).toBeUndefined()
  await other.goto(new URL('/', launchUrl).toString())
  await context.setOffline(true)
  await other.evaluate(() => window.dispatchEvent(new Event('app:reconnect')))
  await expect(other.getByLabel('재료 이름')).toBeDisabled()
  await context.setOffline(false)
  await other.evaluate(() => window.dispatchEvent(new Event('online')))
  await expect(other.getByLabel('재료 이름')).toBeEnabled()
  await other.screenshot({path:test.info().outputPath('desktop.png'), fullPage:true})
  await other.setViewportSize({width:390, height:844})
  await other.getByRole('button', {name:'다크 모드로 전환'}).click()
  await expect(other.locator('html')).toHaveAttribute('data-theme', 'dark')
  expect(await other.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBeTruthy()
  await other.screenshot({path:test.info().outputPath('mobile-dark.png'), fullPage:true})
  for (const path of ['/api/missing','/_nuxt/missing.js','/photos/missing']) {
    const response = await context.request.get(new URL(path, launchUrl).toString())
    expect(response.status()).toBe(404)
    expect(response.headers()['content-type']).not.toContain('text/html')
  }
})

test('second writer is refused and an unusable db directory reports an error', async ({ page }) => {
  await open(page)
  const duplicate = spawn(binary, ['--no-browser'], {cwd:tmpdir()})
  let error = ''; duplicate.stderr!.on('data', x => {error += String(x)})
  const [code] = await once(duplicate, 'exit')
  expect(code).not.toBe(0); expect(error).toContain('이미 실행 중')
  await stop()
  renameSync(join(directory, 'db'), join(directory, 'db-kept'))
  writeFileSync(join(directory, 'db'), 'not a directory')
  const broken = spawn(binary, ['--no-browser'], {cwd:tmpdir()})
  let brokenError = ''; broken.stderr!.on('data', x => {brokenError += String(x)})
  const [brokenCode] = await once(broken, 'exit')
  expect(brokenCode).not.toBe(0); expect(brokenError).toContain('폴더')
})

test('refresh during failed session bootstrap retains the launch token', async ({ page }) => {
  await page.route('**/api/session', route => route.abort('failed'))
  await page.goto(launchUrl)
  await expect(page).toHaveURL(/#token=/)
  await page.unroute('**/api/session')
  await page.reload()
  await expect(page.getByLabel('재료 이름')).toBeEnabled()
  await expect(page).not.toHaveURL(/#token=/)
})
