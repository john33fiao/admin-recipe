import { test, expect, type Page, type Request } from '@playwright/test'
import { spawn, type ChildProcess } from 'node:child_process'
import { once } from 'node:events'
import { randomUUID, createHash } from 'node:crypto'
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmdirSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve, sep } from 'node:path'

const binaryName = process.platform === 'win32' ? 'sunchang-hansang.exe' : 'sunchang-hansang'
let directory: string, binary: string, server: ChildProcess, launchUrl: string
let closed: Promise<[number | null, NodeJS.Signals | null]>
const directories: string[] = []
async function launch() {
  server = spawn(binary, ['--no-browser'], { cwd: directory, stdio: ['ignore', 'pipe', 'pipe'] })
  closed = once(server, 'close') as Promise<[number | null, NodeJS.Signals | null]>
  launchUrl = await new Promise<string>((resolveUrl, reject) => {
    let output = '', errors = ''
    const timer = setTimeout(() => reject(new Error(`photo fixture start timed out: ${errors}`)), 15000)
    server.stderr!.on('data', data => { errors += String(data) })
    server.once('exit', code => { clearTimeout(timer); reject(new Error(`photo fixture exited ${code}: ${errors}`)) })
    server.stdout!.on('data', data => { output += String(data); const match = output.match(/http:\/\/127\.0\.0\.1:\d+\/#token=[a-f0-9]+/); if (match) { clearTimeout(timer); resolveUrl(match[0]) } })
  })
}
test.beforeEach(async () => { directory = mkdtempSync(join(tmpdir(), 'sunchang-profile-')); directories.push(directory); binary = join(directory, binaryName); cpSync(resolve('..', 'target', 'release', binaryName), binary); await launch() })
test.afterEach(async () => {
  if (server?.exitCode === null && server.signalCode === null) server.kill()
  if (closed) await closed
  for (const path of directories.splice(0)) { expect(resolve(path).startsWith(`${resolve(tmpdir())}${sep}`)).toBe(true); rmSync(path, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 }) }
})
test.setTimeout(120000)
async function open(page: Page) { await page.goto(launchUrl); await expect(page.getByLabel('재료 이름', { exact: false })).toBeEnabled() }
async function get(page: Page, path: string) { const r = await page.request.get(new URL(path, launchUrl).toString()); expect(r.ok(), await r.text()).toBe(true); return r.json() }
async function meta(page: Page) { const state = await get(page, '/api/state'); return { request_id: randomUUID(), epoch: state.epoch, expected_revision: state.revision } }
async function post(page: Page, path: string, data: unknown) { const r = await page.request.post(new URL(path, launchUrl).toString(), { headers: { Origin: new URL(launchUrl).origin }, data }); expect(r.status(), await r.text()).toBe(201); return r.json() }
async function material(page: Page, name = '가상 사진 재료') { return post(page, '/api/materials', { ...await meta(page), name, unit: 'g', kind: 'raw', description: '기존 품목 설명' }) }
async function recipe(page: Page, item: any) { return post(page, '/api/recipes', { meta: await meta(page), owner: { mode: 'existing', material_id: item.id, expected_material_version_id: item.version_id }, data: { basis_count: '1', reference_output: null, selling_price: null, cooking_description: '가상 조리 설명', process: '', reference_time: '', lines: [] } }) }
async function visit(page: Page, id: string, kind = 'materials') { await page.goto(new URL(`/${kind}/${id}`, launchUrl).toString()); await expect(panel(page).getByRole('button', { name: '사진·구입처 저장', exact: true })).toBeEnabled() }
const panel = (page: Page) => page.locator('.material-profile')
async function picture(page: Page, color: string, format: 'png' | 'jpeg' = 'png') {
  const url = await page.evaluate(({ color, format }) => { const canvas = document.createElement('canvas'); canvas.width = 360; canvas.height = 240; const ctx = canvas.getContext('2d')!; ctx.fillStyle = color; ctx.fillRect(0, 0, 360, 240); ctx.fillStyle = '#ffffff'; ctx.fillRect(30, 30, 100, 80); return canvas.toDataURL(`image/${format}`) }, { color, format })
  return { name: `virtual.${format === 'jpeg' ? 'jpg' : 'png'}`, mimeType: `image/${format}`, buffer: Buffer.from(url.split(',')[1]!, 'base64') }
}
async function choose(page: Page, photo: Awaited<ReturnType<typeof picture>>, caption: string) { await panel(page).locator('input[type=file]').setInputFiles(photo); const field = panel(page).getByLabel(/^사진 \d+ 설명/).last(); await expect(field).toBeEnabled(); await field.fill(caption) }
async function save(page: Page, id: string, retry = false) {
  const waiting = page.waitForResponse(response => response.request().method() === 'POST' && new URL(response.url()).pathname === `/api/materials/${id}/profiles`)
  await panel(page).getByRole('button', { name: retry ? '같은 사진·구입처 요청 재시도' : '사진·구입처 저장', exact: true }).click()
  const response = await waiting; expect(response.status(), await response.text()).toBe(201)
  const result = await response.json(); await expect(panel(page).getByText('사진·구입처를 저장했습니다.', { exact: true })).toBeVisible(); return result
}
function metadata(request: Request) { const raw = request.postDataBuffer()!.toString('utf8'); return JSON.parse(raw.slice(raw.indexOf('name="metadata"')).split('\r\n\r\n')[1]!.split('\r\n--')[0]!) }
async function current(page: Page, id: string) { return get(page, `/api/materials/${id}/profile`) }
async function asset(page: Page, sha: string) { const r = await page.request.get(new URL(`/api/photo-assets/${sha}`, launchUrl).toString()); expect(r.status()).toBe(200); return r.body() }

test('empty profiles, two original photo formats, one primary and link-only or description-only suppliers', async ({ page }) => {
  await open(page); const item = await material(page); const png = await picture(page, '#295f48'), jpg = await picture(page, '#a64b32', 'jpeg')
  await visit(page, item.id); expect((await current(page, item.id)).version).toBeNull(); await save(page, item.id)
  await choose(page, png, '가상 초록 사진'); await panel(page).getByRole('button', { name: '구입처 추가', exact: true }).click(); await panel(page).getByLabel('구입처 1 링크 (선택)', { exact: true }).fill('https://example.invalid/virtual')
  await panel(page).getByRole('button', { name: '구입처 추가', exact: true }).click(); await panel(page).getByLabel('구입처 2 설명 (선택)', { exact: true }).fill('<script>가상 설명</script>')
  const first = await save(page, item.id); await choose(page, jpg, '가상 붉은 사진'); await panel(page).getByLabel('사진 2을 대표로 표시', { exact: true }).check(); const second = await save(page, item.id)
  expect(second.version.photos.map((photo: any) => photo.is_primary)).toEqual([false, true]); expect(second.version.suppliers[0].description).toBe(''); expect(second.version.suppliers[1].url).toBeNull()
  await expect(panel(page).locator('img')).toHaveCount(1); await expect(panel(page).locator('img')).toHaveAttribute('src', `/api/photo-assets/${second.version.photos[1].asset.sha256}`)
  expect(await asset(page, first.version.photos[0].asset.sha256)).toEqual(png.buffer); expect(await asset(page, second.version.photos[1].asset.sha256)).toEqual(jpg.buffer)
  expect((await get(page, `/api/materials/${item.id}`)).description).toBe('기존 품목 설명'); await expect(panel(page).locator('script')).toHaveCount(0)
})

test('photo replacement and unlink preserve past versions and copied menu profiles remain independent', async ({ page }) => {
  await open(page); const item = await material(page); await recipe(page, item)
  const firstFile = await picture(page, '#174b72'), secondFile = await picture(page, '#b07b24', 'jpeg')
  await visit(page, item.id, 'recipes'); await choose(page, firstFile, '원본 사진'); const first = await save(page, item.id)
  await page.reload(); await expect(page.getByRole('button', { name: '현재 저장본 독립 복제', exact: true })).toBeEnabled()
  const copying = page.waitForResponse(response => response.request().method() === 'POST' && new URL(response.url()).pathname.endsWith('/copies')); await page.getByRole('button', { name: '현재 저장본 독립 복제', exact: true }).click(); const copiedResponse = await copying; expect(copiedResponse.status()).toBe(201); const copied = await copiedResponse.json()
  const copyBefore = await current(page, copied.item.id); expect(copyBefore.version.photos[0].id).not.toBe(first.version.photos[0].id); expect(copyBefore.version.photos[0].asset.sha256).toBe(first.version.photos[0].asset.sha256)
  await visit(page, item.id); await panel(page).getByRole('button', { name: '사진 1 연결 해제', exact: true }).click(); await choose(page, secondFile, '교체 사진'); await save(page, item.id)
  expect((await current(page, copied.item.id)).version).toEqual(copyBefore.version)
  const latestMaterial = await get(page, `/api/materials/${item.id}`); await post(page, `/api/materials/${item.id}/archive`, { ...await meta(page), expected_version_id: latestMaterial.version_id })
  await panel(page).getByRole('button', { name: '사진·구입처 이력', exact: true }).click(); await panel(page).getByRole('button', { name: new RegExp(`기록 ${first.version.seq} 보기$`) }).click()
  await expect(panel(page).getByText('저장된 과거 사진·구입처', { exact: true })).toBeVisible(); await expect(panel(page).locator('img')).toHaveAttribute('src', `/api/photo-assets/${first.version.photos[0].asset.sha256}`)
  expect(await asset(page, first.version.photos[0].asset.sha256)).toEqual(firstFile.buffer)
})

test('lost upload response survives reload and only identical bytes replay the original request', async ({ page }) => {
  await open(page); const item = await material(page), photo = await picture(page, '#244f32'), other = await picture(page, '#814328'); await visit(page, item.id); await choose(page, photo, '불확실 원사진')
  let original: any, finish!: () => void; const handled = new Promise<void>(resolveDone => { finish = resolveDone })
  const path = `**/api/materials/${item.id}/profiles`
  await page.route(path, async route => { original = metadata(route.request()); await route.fetch(); await route.abort('failed'); finish() })
  await panel(page).getByRole('button', { name: '사진·구입처 저장', exact: true }).click(); await expect(panel(page).getByRole('button', { name: '같은 사진·구입처 요청 재시도', exact: true })).toBeVisible(); await handled; await page.unroute(path)
  await page.reload(); const retry = panel(page).getByRole('button', { name: '같은 사진·구입처 요청 재시도', exact: true }); await expect(retry).toBeDisabled()
  await panel(page).locator('input[type=file]').setInputFiles({ ...other, name: photo.name }); await expect(panel(page).getByText('원래 요청에 사용한 사진과 다릅니다. 같은 사진을 다시 선택해 주세요.', { exact: true })).toBeVisible(); await expect(retry).toBeDisabled()
  await panel(page).locator('input[type=file]').setInputFiles({ ...photo, name: 'same-bytes.png' }); await expect(retry).toBeEnabled()
  let repeated: any; await page.route(path, async route => { repeated = metadata(route.request()); await route.continue() }); await save(page, item.id, true)
  expect(repeated).toEqual(original); expect((await get(page, `/api/materials/${item.id}/profiles`)).versions).toHaveLength(1)
})

test('JSON publication failure preserves one photo effect and retries the same metadata after recovery', async ({ page }) => {
  await open(page); const item = await material(page), photo = await picture(page, '#596c36'); await visit(page, item.id); await choose(page, photo, 'JSON 복구 사진')
  const next = BigInt((await get(page, '/api/state')).revision) + 1n, blocked = join(directory, 'db', 'snapshots', `${next.toString().padStart(20, '0')}.json`); mkdirSync(blocked)
  const requests: any[] = []; await page.route(`**/api/materials/${item.id}/profiles`, async route => { requests.push(metadata(route.request())); await route.continue() })
  const response = page.waitForResponse(value => value.request().method() === 'POST' && new URL(value.url()).pathname.endsWith('/profiles')); await panel(page).getByRole('button', { name: '사진·구입처 저장', exact: true }).click(); expect((await response).status()).toBe(503)
  await expect(panel(page).getByRole('button', { name: '같은 사진·구입처 요청 재시도', exact: true })).toBeEnabled(); rmdirSync(blocked); const saved = await save(page, item.id, true)
  expect(requests).toHaveLength(2); expect(requests[1]).toEqual(requests[0]); expect((await get(page, `/api/materials/${item.id}/profiles`)).versions).toHaveLength(1)
  expect(JSON.parse(readFileSync(blocked, 'utf8')).request_id).toBe(requests[0].meta.request_id); expect(await asset(page, saved.version.photos[0].asset.sha256)).toEqual(photo.buffer)
})

test('unsafe URLs and broken images preserve previous connections and stale edits require explicit comparison', async ({ page }) => {
  await open(page); const item = await material(page), photo = await picture(page, '#445e7a'); await visit(page, item.id); await choose(page, photo, '유효 사진'); const first = await save(page, item.id)
  await panel(page).getByRole('button', { name: '구입처 추가', exact: true }).click(); await panel(page).getByLabel('구입처 1 링크 (선택)', { exact: true }).fill('javascript:alert(1)'); await panel(page).getByRole('button', { name: '사진·구입처 저장', exact: true }).click(); await expect(panel(page).getByLabel('구입처 1 링크 (선택)', { exact: true })).toBeFocused(); expect((await current(page, item.id)).version.id).toBe(first.version.id)
  await panel(page).getByLabel('구입처 1 링크 (선택)', { exact: true }).fill('https://example.invalid/virtual'); await panel(page).locator('input[type=file]').setInputFiles({ name: 'broken.png', mimeType: 'image/png', buffer: Buffer.from([137,80,78,71,13,10,26,10,1]) }); await expect(panel(page).getByLabel('사진 2 설명 (선택)', { exact: true })).toBeEnabled()
  let response = page.waitForResponse(value => value.request().method() === 'POST' && new URL(value.url()).pathname.endsWith('/profiles')); await panel(page).getByRole('button', { name: '사진·구입처 저장', exact: true }).click(); expect((await response).status()).toBe(400); expect((await current(page, item.id)).version.id).toBe(first.version.id)
  await panel(page).getByRole('button', { name: '최신 사진·구입처 확인 후 입력 검토', exact: true }).click(); await expect(panel(page).getByRole('button', { name: '사진 2 연결 해제', exact: true })).toBeEnabled(); await panel(page).getByRole('button', { name: '사진 2 연결 해제', exact: true }).click()
  await page.getByLabel('재료 이름', { exact: true }).fill('저장하지 않은 품목 이름')
  await material(page, '외부 변경'); response = page.waitForResponse(value => value.request().method() === 'POST' && new URL(value.url()).pathname.endsWith('/profiles')); await panel(page).getByRole('button', { name: '사진·구입처 저장', exact: true }).click(); expect((await response).status()).toBe(409)
  await panel(page).getByRole('button', { name: '최신 사진·구입처 확인 후 입력 검토', exact: true }).click(); await expect(panel(page).getByRole('button', { name: '사진·구입처 저장', exact: true })).toBeEnabled(); await save(page, item.id)
  await expect(page.getByLabel('재료 이름', { exact: true })).toHaveValue('저장하지 않은 품목 이름'); expect((await current(page, item.id)).version.photos[0].id).toBe(first.version.photos[0].id)
})

test('pending upload completed after switching to the recipe shares one editor identity and clears its file', async ({ page }) => {
  await open(page); const item = await material(page); await recipe(page, item); const photo = await picture(page, '#547c48'); await visit(page, item.id); await choose(page, photo, '늦은 완료 사진')
  let release!: () => void, received!: () => void; const gate = new Promise<void>(resolveGate => { release = resolveGate }), ready = new Promise<void>(resolveReady => { received = resolveReady })
  await page.route(`**/api/materials/${item.id}/profiles`, async route => { const response = await route.fetch(); received(); await gate; await route.fulfill({ response }) })
  await panel(page).getByRole('button', { name: '사진·구입처 저장', exact: true }).click(); await ready
  await page.getByRole('link', { name: '이 품목의 레시피 보기', exact: true }).click(); await expect(page).toHaveURL(new URL(`/recipes/${item.id}`, launchUrl).toString()); release()
  await expect(panel(page).getByText('사진·구입처를 저장했습니다.', { exact: true })).toBeVisible(); await expect(panel(page).getByRole('button', { name: '사진·구입처 저장', exact: true })).toBeEnabled(); await expect(panel(page).getByLabel('사진 1 설명 (선택)', { exact: true })).toHaveValue('늦은 완료 사진')
  await expect(panel(page).getByText('새 사진', { exact: true })).toHaveCount(0); expect((await get(page, `/api/materials/${item.id}/profiles`)).versions).toHaveLength(1)
  const ids = await page.locator('[id]').evaluateAll(nodes => nodes.map(node => node.id)); expect(new Set(ids).size).toBe(ids.length)
})

test('photo history survives normal restart and whole-folder move with six usable layouts', async ({ page }) => {
  test.setTimeout(260000)
  await open(page); const item = await material(page, '가상 긴 이름의 재료와 사진·구입처를 함께 확인하는 자료'), photo = await picture(page, '#36735c'), second = await picture(page, '#9c5b28', 'jpeg'); await visit(page, item.id); await choose(page, photo, '가상 초록 사진과 긴 설명을 모바일에서도 확인합니다'); const first = await save(page, item.id)
  await choose(page, second, '가상 두 번째 사진'); await panel(page).getByRole('button', { name: '구입처 추가', exact: true }).click(); await panel(page).getByLabel('구입처 1 링크 (선택)', { exact: true }).fill(`https://example.invalid/${'virtual-'.repeat(20)}`); await panel(page).getByLabel('구입처 1 설명 (선택)', { exact: true }).fill('가상 구입처 설명입니다.\n개인 주소나 실제 거래처 자료가 아닙니다.'); const last = await save(page, item.id)
  await page.goto('about:blank'); let timer: ReturnType<typeof setTimeout> | undefined
  const [code, signal] = await Promise.race([closed, new Promise<never>((_, reject) => { timer = setTimeout(() => reject(new Error('profile normal shutdown timed out')), 180000) })]).finally(() => clearTimeout(timer)); expect(code).toBe(0); expect(signal).toBeNull()
  const moved = mkdtempSync(join(tmpdir(), 'sunchang-가상 사진 이동 ')); directories.push(moved); cpSync(directory, moved, { recursive: true }); directory = moved; binary = join(directory, binaryName); await launch(); await open(page); await visit(page, item.id)
  expect((await current(page, item.id)).version).toEqual(last.version); expect((await get(page, `/api/materials/${item.id}/profiles/${first.version.id}`)).version).toEqual(first.version); expect(createHash('sha256').update(await asset(page, first.version.photos[0].asset.sha256)).digest('hex')).toBe(first.version.photos[0].asset.sha256)
  for (const width of [1440, 768, 390]) for (const theme of ['light', 'dark']) {
    await page.setViewportSize({ width, height: 900 })
    if (await page.locator('html').getAttribute('data-theme') !== theme) await page.getByRole('button', { name: theme === 'dark' ? '다크 모드로 전환' : '라이트 모드로 전환', exact: true }).click()
    await expect(page.locator('html')).toHaveAttribute('data-theme', theme)
    await expect(panel(page).locator('img')).toHaveCount(1); await expect.poll(() => panel(page).locator('img').evaluate((img: HTMLImageElement) => img.complete && img.naturalWidth > 0)).toBe(true)
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true)
    await page.screenshot({ path: test.info().outputPath(`profile-${width}-${theme}.png`), fullPage: true })
  }
})
