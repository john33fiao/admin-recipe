import { test, expect, type Page } from '@playwright/test'
import { spawn, type ChildProcess } from 'node:child_process'
import { once } from 'node:events'
import { createHash, randomUUID } from 'node:crypto'
import { cpSync, mkdtempSync, readFileSync, readdirSync, renameSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve, sep } from 'node:path'

const binaryName = process.platform === 'win32' ? 'sunchang-hansang.exe' : 'sunchang-hansang'
let directory: string, binary: string, server: ChildProcess, closed: Promise<[number | null, NodeJS.Signals | null]>, launchUrl: string
async function launch() {
  server = spawn(binary, ['--no-browser'], { cwd: directory, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] }); closed = once(server, 'close') as typeof closed
  launchUrl = await new Promise<string>((resolveUrl, reject) => {
    let output = '', errors = ''; const timer = setTimeout(() => reject(Error(`restore fixture start timed out: ${errors}`)), 15_000)
    server.stderr!.on('data', data => { errors += String(data) }); server.once('exit', code => { clearTimeout(timer); reject(Error(`restore fixture exited ${code}: ${errors}`)) })
    server.stdout!.on('data', data => { output += String(data); const match = output.match(/http:\/\/127\.0\.0\.1:\d+\/#token=[a-f0-9]+/); if (match) { clearTimeout(timer); resolveUrl(match[0]) } })
  })
}
test.beforeEach(async () => { directory = mkdtempSync(join(tmpdir(), 'sunchang-restore-')); binary = join(directory, binaryName); cpSync(resolve('..', 'target', 'release', binaryName), binary); await launch() })
test.afterEach(async () => { if (server?.exitCode === null && server.signalCode === null) server.kill(); if (closed) await closed; if (directory) { expect(resolve(directory).startsWith(`${resolve(tmpdir())}${sep}`)).toBe(true); rmSync(directory, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 }) } })
test.setTimeout(150_000)
async function open(page: Page) { await page.goto(launchUrl); await expect(page.getByLabel('재료 이름', { exact: false })).toBeEnabled() }
async function visit(page: Page, path = '/restore') { await page.goto(new URL(path, launchUrl).toString()); await expect(page.getByRole('heading', { name: '자료 복원', exact: true })).toBeVisible(); await expect(page.getByRole('button', { name: '처음부터 다시 조회', exact: true })).toBeEnabled() }
async function get(page: Page, path: string) { const response = await page.request.get(new URL(path, launchUrl).toString()); expect(response.ok(), await response.text()).toBe(true); return response.json() }
async function post(page: Page, path: string, data: unknown) { const response = await page.request.post(new URL(path, launchUrl).toString(), { headers: { Origin: new URL(launchUrl).origin }, data }); expect(response.status(), await response.text()).toBe(201); return response.json() }
async function meta(page: Page) { const state = await get(page, '/api/state'); return { request_id: randomUUID(), epoch: state.epoch, expected_revision: state.revision } }
async function material(page: Page, name: string) { return post(page, '/api/materials', { ...await meta(page), name, unit: 'g', kind: 'raw', description: '' }) }
async function targets(page: Page) { return get(page, '/api/restore-targets?source=journal') }
async function latest(page: Page) { return (await targets(page)).rows[0] }
async function choose(page: Page, target: any) { await page.locator(`[data-restore-target="${target.target_id}"]`).click(); await page.getByRole('button', { name: '전체 영향 미리보기', exact: true }).click(); await expect(page.getByRole('heading', { name: '전체 상태 변경 미리보기', exact: true })).toBeVisible() }
async function restore(page: Page) { const waiting = page.waitForResponse(response => response.request().method() === 'POST' && new URL(response.url()).pathname === '/api/restores'); await page.getByRole('button', { name: '선택한 전체 상태로 복원 확정', exact: true }).click(); const response = await waiting; expect(response.status(), await response.text()).toBe(201); const result = await response.json(); await expect(page.getByRole('heading', { name: '전체 상태 복원 완료', exact: true })).toBeVisible(); return result }
function snapshots() { const root = join(directory, 'db', 'snapshots'); return readdirSync(root).map(name => ({ path: join(root, name), bytes: readFileSync(join(root, name)) })) }
function hash(bytes: Buffer) { return createHash('sha256').update(bytes).digest('hex') }

test('whole-state restore preserves selected records, recipe and draft while retaining later snapshots and permits new writes', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 복원 원재료')
  const recipe = await post(page, '/api/recipes', { meta: await meta(page), owner: { mode: 'new', name: '가상 복원 메뉴', unit: '개', kind: 'menu', description: '' }, data: { basis_count: '2', reference_output: null, selling_price: null, cooking_description: '', process: '가상 공정', reference_time: '', lines: [{ material_id: raw.id, material_version_id: raw.version_id, quantity: '0', unit: 'g', note: '' }] } })
  const draft = await post(page, '/api/business/drafts', { meta: await meta(page), payload: { kind: 'sale', version: 1, data: { material_id: recipe.item.id, material_version_id: recipe.item.version_id, recipe_version_id: recipe.version.id, count: '0', note: '복원할 초안' } }, corrects_confirmation_id: null })
  const selected = await latest(page), beforeRecipe = await get(page, `/api/recipes/${recipe.item.id}`), beforeDraft = await get(page, `/api/business/${draft.result.item.id}`)
  const later = await material(page, '가상 선택 이후 재료'), originalFiles = snapshots().map(value => ({ path: value.path, hash: hash(value.bytes) })), oldState = await get(page, '/api/state')
  await visit(page); await choose(page, selected); await expect(page.locator('.restore-comparison')).toContainText('DB 임시 저장'); const restored = await restore(page)
  expect(restored.state.epoch).not.toBe(oldState.epoch); expect(BigInt(restored.state.revision)).toBeGreaterThan(BigInt(oldState.revision)); expect((await get(page, '/api/materials')).map((item: any) => item.id)).not.toContain(later.id)
  const afterRecipe = await get(page, `/api/recipes/${recipe.item.id}`), afterDraft = await get(page, `/api/business/${draft.result.item.id}`); expect(afterRecipe.version).toEqual(beforeRecipe.version); expect(afterDraft.head).toEqual(beforeDraft.head)
  for (const file of originalFiles) expect(hash(readFileSync(file.path))).toBe(file.hash)
  const created = await material(page, '가상 복원 뒤 신규 재료'); expect(created.id).not.toBe(later.id)
  await page.getByRole('button', { name: '복원 직전 상태의 영향 검토하기', exact: true }).click(); await page.getByRole('button', { name: '전체 영향 미리보기', exact: true }).click(); await restore(page)
  expect((await get(page, '/api/materials')).map((item: any) => item.id)).toContain(later.id); expect((await get(page, '/api/materials')).map((item: any) => item.id)).not.toContain(created.id)
})

test('raw JSON import is byte-identical, historical v1 restores and invalid input does not change active state', async ({ page }) => {
  await open(page); await material(page, '가상 가져오기 전 자료'); await visit(page)
  const before = await get(page, '/api/state'), bytes = readFileSync(resolve('..', 'tests', 'fixtures', 'snapshot-v1.json'))
  await page.getByLabel('복원 JSON 파일 · 최대 64 MiB', { exact: true }).setInputFiles({ name: 'virtual-legacy.json', mimeType: 'application/json', buffer: bytes })
  const uploaded = page.waitForResponse(response => new URL(response.url()).pathname === '/api/restore-targets/import' && response.request().method() === 'POST'); await page.getByRole('button', { name: 'JSON 원문 가져오기', exact: true }).click()
  const importResponse = await uploaded; expect(importResponse.status(), await importResponse.text()).toBe(201); expect(importResponse.request().headers()['content-type']).toBe('application/json')
  const imported = await importResponse.json(), expectedHash = hash(bytes); expect(imported.sha256).toBe(expectedHash); expect(imported.byte_length).toBe(String(bytes.length)); expect(readFileSync(join(directory, 'db', 'imports', `${expectedHash}.json`))).toEqual(bytes)
  await expect(page.getByText('JSON을 보관했습니다. 아래에서 전체 영향을 미리보기해 주세요.', { exact: true })).toBeVisible(); expect(await get(page, '/api/state')).toEqual(before)
  await page.getByRole('button', { name: '전체 영향 미리보기', exact: true }).click(); await restore(page); expect((await get(page, '/api/materials')).map((item: any) => [item.name, item.unit])).toEqual(expect.arrayContaining([['가상 이전 같은 이름', 'g'], ['가상 이전 같은 이름', '장']]))
  const restoredState = await get(page, '/api/state'); await page.getByLabel('복원 JSON 파일 · 최대 64 MiB', { exact: true }).setInputFiles({ name: 'invalid.json', mimeType: 'application/json', buffer: Buffer.from('{"format_version":999,"format_version":1}') })
  await page.getByRole('button', { name: 'JSON 원문 가져오기', exact: true }).click(); await expect(page.locator('[aria-labelledby="restore-import-title"] [role="alert"]')).toBeVisible(); expect(await get(page, '/api/state')).toEqual(restoredState)
  await page.getByRole('button', { name: '원파일 확인 정보를 보관하고 다른 파일 선택', exact: true }).click(); await expect(page.getByLabel('복원 JSON 파일 · 최대 64 MiB', { exact: true })).toBeEnabled()
})

test('missing photo blocks the complete restore and replacing the original file permits preview without dropping photo references', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 사진 복원')
  const dataUrl = await page.evaluate(() => { const canvas = document.createElement('canvas'); canvas.width = 16; canvas.height = 16; const ctx = canvas.getContext('2d')!; ctx.fillStyle = '#447755'; ctx.fillRect(0, 0, 16, 16); return canvas.toDataURL('image/png') })
  const photo = Buffer.from(dataUrl.split(',')[1]!, 'base64'), metadata = { meta: await meta(page), expected_material_version_id: raw.version_id, expected_profile_version_id: null, photos: [{ source: 'upload', caption: '가상 보존 사진' }], primary_position: 0, suppliers: [{ url: null, description: '가상 구입처' }] }
  const response = await page.request.post(new URL(`/api/materials/${raw.id}/profiles`, launchUrl).toString(), { headers: { Origin: new URL(launchUrl).origin }, multipart: { metadata: JSON.stringify(metadata), photo: { name: 'virtual.png', mimeType: 'image/png', buffer: photo } } }); expect(response.status(), await response.text()).toBe(201)
  const profile = await response.json(), selected = await latest(page), photoPath = resolve(directory, 'db', profile.version.photos[0].asset.asset_key), moved = `${photoPath}.test-preserved`
  expect(photoPath.startsWith(`${resolve(directory, 'db')}${sep}`)).toBe(true); renameSync(photoPath, moved)
  try { const before = await get(page, '/api/state'); await visit(page); await choose(page, selected); await expect(page.locator('.restore-comparison')).toContainText('누락 1개'); await expect(page.getByRole('button', { name: '선택한 전체 상태로 복원 확정', exact: true })).toBeDisabled(); expect(await get(page, '/api/state')).toEqual(before) } finally { renameSync(moved, photoPath) }
  await page.getByRole('button', { name: '전체 영향 미리보기', exact: true }).click(); await expect(page.getByRole('button', { name: '선택한 전체 상태로 복원 확정', exact: true })).toBeEnabled(); await restore(page); expect((await get(page, `/api/materials/${raw.id}/profile`)).version).toEqual(profile.version)
})

test('lost restore response survives a normal process restart and replays exactly once', async ({ page }) => {
  await open(page); await material(page, '가상 재시작 원본'); const selected = await latest(page); await material(page, '가상 재시작 이후'); await visit(page); await choose(page, selected)
  let original: any, finish!: () => void; const handled = new Promise<void>(resolveDone => { finish = resolveDone })
  await page.route('**/api/restores', async route => { original = route.request().postDataJSON(); await route.fetch(); await route.abort('failed'); finish() })
  await page.getByRole('button', { name: '선택한 전체 상태로 복원 확정', exact: true }).click(); await handled; await expect(page.getByRole('button', { name: '같은 요청으로 결과 확인', exact: true })).toBeEnabled(); await page.unroute('**/api/restores')
  const stateAfter = await get(page, '/api/state'), pending = await page.evaluate(() => sessionStorage.getItem('sunchang.restore.pending.v1')); expect(JSON.parse(pending!)).toEqual(original)
  const savedInputs = await page.evaluate(() => Object.fromEntries(Object.entries(sessionStorage))); await page.goto('about:blank'); const [exit] = await closed; expect(exit).toBe(0); await launch()
  await page.addInitScript(values => { for (const [key, value] of Object.entries(values)) sessionStorage.setItem(key, value) }, savedInputs); await open(page); await visit(page)
  const waiting = page.waitForRequest(request => new URL(request.url()).pathname === '/api/restores' && request.method() === 'POST'); await page.getByRole('button', { name: '같은 요청으로 결과 확인', exact: true }).click(); expect((await waiting).postDataJSON()).toEqual(original); await expect(page.getByRole('heading', { name: '전체 상태 복원 완료', exact: true })).toBeVisible(); expect(await get(page, '/api/state')).toEqual(stateAfter)
})

test('old unprocessed material request keeps input and requires explicit review after another tab restores', async ({ page, context }) => {
  await open(page); await material(page, '가상 여러 탭 원본'); const selected = await latest(page); await material(page, '가상 이후 자료')
  await page.getByLabel('재료 이름', { exact: false }).fill('가상 미처리 보존 입력')
  let original: any, finish!: () => void; const handled = new Promise<void>(resolveDone => { finish = resolveDone })
  await page.route('**/api/materials', async route => { if (route.request().method() === 'GET') return route.continue(); original = route.request().postDataJSON(); await route.abort('failed'); finish() })
  await page.getByRole('button', { name: '재료 등록', exact: true }).click(); await handled; await page.unroute('**/api/materials')
  const other = await context.newPage(); await visit(other); await choose(other, selected); await restore(other)
  await expect(page.locator('.restore-banner')).toBeVisible(); await page.getByRole('button', { name: '같은 요청 재시도', exact: true }).click(); await expect(page.getByText(/복원으로 자료가 바뀌어 이 요청은 처리하지 않았습니다/)).toBeVisible(); await expect(page.getByLabel('재료 이름', { exact: false })).toHaveValue('가상 미처리 보존 입력')
  expect(JSON.parse((await page.evaluate(() => sessionStorage.getItem('sunchang.pending-material.v1')))!)).toEqual(original)
  await other.close()
})

test('late old success retains original processed request without reapplying it', async ({ page, context }) => {
  await open(page); await material(page, '가상 지연 전 원본'); const selected = await latest(page)
  await page.getByLabel('재료 이름', { exact: false }).fill('가상 늦게 도착하는 등록')
  let release!: () => void, fetched!: () => void, finished!: () => void, original: any
  const held = new Promise<void>(resolveDone => { release = resolveDone }), reached = new Promise<void>(resolveDone => { fetched = resolveDone }), handled = new Promise<void>(resolveDone => { finished = resolveDone })
  await page.route('**/api/materials', async route => { if (route.request().method() === 'GET') return route.continue(); original = route.request().postDataJSON(); const response = await route.fetch(); fetched(); await held; await route.fulfill({ response }); finished() })
  await page.getByRole('button', { name: '재료 등록', exact: true }).click(); await reached
  const other = await context.newPage(); await visit(other); await choose(other, selected); await restore(other); await expect(page.locator('.restore-banner')).toBeVisible(); release(); await handled; await page.unroute('**/api/materials')
  await expect(page.getByText(/복원 전에 처리된 요청입니다/)).toBeVisible(); await expect(page.getByLabel('재료 이름', { exact: false })).toHaveValue('가상 늦게 도착하는 등록'); expect(JSON.parse((await page.evaluate(() => sessionStorage.getItem('sunchang.pending-material.v1')))!)).toEqual(original); expect((await get(page, '/api/materials')).map((item: any) => item.name)).not.toContain('가상 늦게 도착하는 등록')
  await page.getByRole('navigation', { name: '주요 메뉴' }).getByRole('link', { name: '자료 복원', exact: true }).click(); await expect(page.getByRole('heading', { name: '보존한 입력·요청', exact: true })).toBeVisible()
  const opened = context.waitForEvent('page'); await page.getByRole('link', { name: '새 탭에서 현재 자료로 작업 시작', exact: true }).click(); const fresh = await opened
  await expect(fresh.locator('.overview-count')).toHaveText('1개'); expect(await fresh.evaluate(() => window.opener)).toBeNull(); expect(await fresh.evaluate(() => sessionStorage.getItem('sunchang.pending-material.v1'))).toBeNull(); expect(JSON.parse((await page.evaluate(() => sessionStorage.getItem('sunchang.pending-material.v1')))!)).toEqual(original); await fresh.close()
  await page.getByRole('navigation', { name: '주요 메뉴' }).getByRole('link', { name: '재료 관리', exact: true }).click()
  let writes = 0; page.on('request', request => { if (request.method() === 'POST' && new URL(request.url()).pathname === '/api/materials') writes++ })
  await page.getByRole('button', { name: '원요청 보관하고 새 재료 입력', exact: true }).click(); await expect(page.getByLabel('재료 이름', { exact: false })).toHaveValue(''); expect(writes).toBe(0)
  await page.getByLabel('재료 이름', { exact: false }).fill('가상 복원 뒤 별도 새입력'); const sending = page.waitForRequest(request => request.method() === 'POST' && new URL(request.url()).pathname === '/api/materials'); await page.getByRole('button', { name: '재료 등록', exact: true }).click(); const newRequest = (await sending).postDataJSON(); expect(newRequest.request_id).not.toBe(original.request_id); expect(newRequest.epoch).toBe((await get(page, '/api/state')).epoch); await expect(page.getByRole('link', { name: '등록한 재료 보기', exact: true })).toBeVisible()
  await page.getByRole('navigation', { name: '주요 메뉴' }).getByRole('link', { name: '자료 복원', exact: true }).click(); const retainedIndex = await page.locator('#retained-inputs textarea').evaluateAll(elements => elements.findIndex(element => (element as HTMLTextAreaElement).value.includes('가상 늦게 도착하는 등록'))); expect(retainedIndex).toBeGreaterThanOrEqual(0); const retained = page.locator('#retained-inputs details').nth(retainedIndex); await retained.locator('summary').click(); await expect(retained.locator('textarea')).toHaveValue(/가상 늦게 도착하는 등록/); await other.close()
})

test('restore list keysets, keyboard focus and full comparison remain readable in six layouts', async ({ page }, info) => {
  await open(page); for (let index = 0; index < 27; index++) await material(page, `가상 긴 복원 검토 품목 ${index + 1}`)
  await visit(page); await expect(page.locator('[data-restore-target]')).toHaveCount(25); await page.getByRole('button', { name: '다음 복원 지점 25개 보기', exact: true }).click(); await expect(page.locator('[data-restore-target]')).toHaveCount(2)
  await page.locator('[data-restore-target]').first().focus(); await page.keyboard.press('Enter'); const preview = page.getByRole('button', { name: '전체 영향 미리보기', exact: true }); await preview.focus(); await page.keyboard.press('Space'); await expect(page.getByRole('heading', { name: '선택한 복원 지점', exact: true })).toBeFocused(); await expect(page.locator('.restore-comparison tbody tr')).toHaveCount(15)
  const before = await get(page, '/api/state')
  const comparisonFits = () => page.locator('.restore-comparison .table-scroll').evaluate(region => {
    const boundary = region.getBoundingClientRect()
    return region.scrollWidth <= region.clientWidth && Array.from(region.querySelectorAll('th, td')).every(cell => {
      const rect = cell.getBoundingClientRect(); return rect.left >= boundary.left && rect.right <= boundary.right
    })
  })
  for (const width of [1440, 768, 390]) for (const theme of ['light', 'dark']) {
    await page.setViewportSize({ width, height: 1000 }); const current = await page.locator('html').getAttribute('data-theme'); if (current !== theme) await page.getByRole('button', { name: theme === 'dark' ? '다크 모드로 전환' : '라이트 모드로 전환', exact: true }).click()
    expect(await page.locator('html').getAttribute('data-theme')).toBe(theme); expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true)
    expect(await comparisonFits()).toBe(true)
    await page.screenshot({ path: info.outputPath(`restore-${width}-${theme}.png`), fullPage: true })
  }
  await page.setViewportSize({ width: 540, height: 1000 }); expect(await comparisonFits()).toBe(true)
  expect(await get(page, '/api/state')).toEqual(before)
})
