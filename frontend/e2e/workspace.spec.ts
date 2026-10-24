import { test, expect, type Page } from '@playwright/test'
import { spawn, type ChildProcess } from 'node:child_process'
import { once } from 'node:events'
import { randomUUID } from 'node:crypto'
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmdirSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve, sep } from 'node:path'

const binaryName = process.platform === 'win32' ? 'sunchang-hansang.exe' : 'sunchang-hansang'
let directory: string, binary: string, server: ChildProcess
let closed: Promise<[number | null, NodeJS.Signals | null]>, launchUrl: string

async function launch() {
  server = spawn(binary, ['--no-browser'], { cwd: directory, stdio: ['ignore', 'pipe', 'pipe'] })
  closed = once(server, 'close') as Promise<[number | null, NodeJS.Signals | null]>
  launchUrl = await new Promise<string>((resolveUrl, reject) => {
    let output = '', errors = ''
    const timer = setTimeout(() => reject(new Error(`workspace fixture start timed out: ${errors}`)), 15_000)
    server.stderr!.on('data', data => { errors += String(data) })
    server.once('exit', code => { clearTimeout(timer); reject(new Error(`workspace fixture exited ${code}: ${errors}`)) })
    server.stdout!.on('data', data => {
      output += String(data)
      const match = output.match(/http:\/\/127\.0\.0\.1:\d+\/#token=[a-f0-9]+/)
      if (match) { clearTimeout(timer); resolveUrl(match[0]) }
    })
  })
}

test.beforeEach(async () => {
  directory = mkdtempSync(join(tmpdir(), 'sunchang-workspace-'))
  binary = join(directory, binaryName)
  cpSync(resolve('..', 'target', 'release', binaryName), binary)
  await launch()
})
test.afterEach(async () => {
  if (server && server.exitCode === null && server.signalCode === null) server.kill()
  if (closed) await closed
  if (directory) {
    expect(resolve(directory).startsWith(`${resolve(tmpdir())}${sep}`)).toBe(true)
    rmSync(directory, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 })
  }
})
test.setTimeout(120_000)

async function open(page: Page) {
  await page.goto(launchUrl)
  await expect(page.getByLabel('재료 이름', { exact: false })).toBeEnabled()
}
async function visit(page: Page, path: string) { await page.goto(new URL(path, launchUrl).toString()) }
async function workspace(page: Page) {
  await visit(page, '/workspace')
  await expect(page.getByRole('heading', { name: '모아 입력', exact: true })).toBeVisible()
  await expect(page.getByRole('button', { name: '판매 추가', exact: true })).toBeEnabled()
}
async function get(page: Page, path: string) {
  const response = await page.request.get(new URL(path, launchUrl).toString())
  expect(response.ok(), await response.text()).toBe(true)
  return response.json()
}
async function post(page: Page, path: string, data: unknown) {
  const response = await page.request.post(new URL(path, launchUrl).toString(), { headers: { Origin: new URL(launchUrl).origin }, data })
  expect(response.status(), await response.text()).toBe(201)
  return response.json()
}
async function meta(page: Page) {
  const state = await get(page, '/api/state')
  return { request_id: randomUUID(), epoch: state.epoch, expected_revision: state.revision }
}
async function material(page: Page, name: string) {
  return post(page, '/api/materials', { ...await meta(page), name, unit: 'g', kind: 'raw', description: '가상 모아 입력' })
}
async function recipe(page: Page, raw: any, name: string, quantity = '0', sellingPrice: string | null = '1000') {
  return post(page, '/api/recipes', {
    meta: await meta(page), owner: { mode: 'new', name, unit: '개', kind: 'menu', description: '' },
    data: { basis_count: '1', reference_output: null, selling_price: sellingPrice, cooking_description: '', process: '', reference_time: '', lines: [{ material_id: raw.id, material_version_id: raw.version_id, quantity, unit: 'g', note: '' }] },
  })
}
async function apiDraft(page: Page, kind: string, data: unknown) {
  const path = kind === 'stocktake' ? '/api/stocktake-drafts' : kind === 'production' ? '/api/production-drafts' : '/api/business/drafts'
  return post(page, path, { meta: await meta(page), payload: { kind, version: 1, data }, corrects_confirmation_id: null })
}
async function apiConfirm(page: Page, saved: any) {
  return post(page, `/api/business/${saved.result.item.id}/confirm`, { meta: await meta(page), expected_head_id: saved.result.head.id })
}
function stockRow(item: any, quantity: string, amount?: string) {
  return { material_id: item.id, material_version_id: item.version_id, quantity, unit: 'g', initial_value: amount === undefined ? null : { kind: 'manual_amount', amount }, excess: null }
}
async function observe(page: Page, rows: any[]) {
  const saved = await apiConfirm(page, await apiDraft(page, 'stocktake', { rows, note: '가상 시작 관측' }))
  return get(page, `/api/stocktakes/${saved.result.confirmation.result.stocktake_id}`)
}
async function receipt(page: Page, item: any, quantity: string, amount: string) {
  return apiConfirm(page, await apiDraft(page, 'receipt', { source: { kind: 'direct', material_id: item.id, material_version_id: item.version_id }, quantity, unit: 'g', amount, receipt_date: '2024-02-29', note: '', stock_effect: { kind: 'unreflected' } }))
}
async function purchaseReturn(page: Page, receiptId: string, quantity: string) {
  const source = await get(page, `/api/receipts/${receiptId}/return-source`)
  return apiConfirm(page, await apiDraft(page, 'purchase_return', { receipt_id: receiptId, expected_receipt_version_id: source.current.reference.version_id, quantity, unit: 'g', note: '가상 실제 반품' }))
}
async function holding(page: Page, item: any) { return get(page, `/api/holdings/${item.id}`) }
function amounts(input: any) { return { quantity: input.settlement_basis?.quantity, value: input.settlement_basis?.value } }
function workButton(page: Page, identity: string) { return page.locator(`button[data-workspace-id="${identity}"]`) }
function workItem(page: Page, identity: string) { return page.locator('.workspace-item').filter({ has: workButton(page, identity) }) }
async function select(page: Page, identity: string) {
  await workButton(page, identity).click()
  await expect(workButton(page, identity)).toHaveAttribute('aria-pressed', 'true')
  await expect(page.locator('.workspace-editor-heading')).toBeVisible()
}
type WorkKind = 'sale' | 'production' | 'waste' | 'stocktake'
const addLabels: Record<WorkKind, string> = { sale: '판매 추가', production: '생산 추가', waste: '폐기 · 직원식 추가', stocktake: '실사 추가' }
async function add(page: Page, kind: WorkKind) {
  const existing = await page.locator('button[data-workspace-id]').evaluateAll(nodes => nodes.map(node => node.getAttribute('data-workspace-id')))
  await page.getByRole('button', { name: addLabels[kind], exact: true }).click()
  const selected = page.locator('button[data-workspace-id][aria-pressed="true"]')
  await expect(selected).toHaveCount(1)
  await expect.poll(async () => {
    const identity = await selected.getAttribute('data-workspace-id')
    return identity !== null && !existing.includes(identity)
  }).toBe(true)
  const identity = (await selected.getAttribute('data-workspace-id'))!
  expect(existing).not.toContain(identity)
  await expect(page.locator('.business-editor')).toHaveCount(1)
  return identity
}
async function saleFields(page: Page, menu: any, count: string, note = '') {
  await page.getByLabel('판매 메뉴', { exact: true }).selectOption(menu.item.id)
  await page.getByLabel('판매 개수', { exact: true }).fill(count)
  await page.getByLabel('판매 메모 (선택)', { exact: true }).fill(note)
}
async function productionRow(page: Page, side: '투입' | '결과', item: any, quantity: string) {
  await page.getByLabel(`${side} 1 품목`, { exact: true }).selectOption(item.id)
  await page.getByLabel(`${side} 1 실제 수량`, { exact: true }).fill(quantity)
  await page.getByLabel(`${side} 1 단위`, { exact: true }).fill('g')
}
async function stockFields(page: Page, item: any, quantity: string, index = 1) {
  await page.getByLabel(`실사 ${index} 품목`, { exact: true }).selectOption(item.id)
  await page.getByLabel(`실사 ${index} 실제 수량`, { exact: true }).fill(quantity)
  await page.getByLabel(`실사 ${index} 단위`, { exact: true }).fill('g')
}
async function wasteFields(page: Page, item: any, quantity: string) {
  await page.getByLabel('폐기 · 직원식 품목', { exact: true }).selectOption(item.id)
  await page.getByLabel('폐기 · 직원식 실제 수량', { exact: true }).fill(quantity)
  await page.getByLabel('폐기 · 직원식 단위', { exact: true }).fill('g')
}
const createPaths = ['/api/business/drafts', '/api/production-drafts', '/api/stocktake-drafts']
async function saveNew(page: Page) {
  const responsePromise = page.waitForResponse(response => response.request().method() === 'POST' && createPaths.some(path => new URL(response.url()).pathname === path))
  await page.getByRole('button', { name: '초안 임시 저장', exact: true }).click()
  const response = await responsePromise
  expect(response.status(), await response.text()).toBe(201)
  const saved = await response.json()
  await expect(workButton(page, saved.result.item.id)).toHaveAttribute('aria-pressed', 'true')
  await expect(page.getByRole('button', { name: '확정 전 미리보기', exact: true })).toBeVisible()
  await expect(page).toHaveURL(new URL('/workspace', launchUrl).toString())
  return saved
}
async function preview(page: Page) {
  const responsePromise = page.waitForResponse(response => response.request().method() === 'POST' && new URL(response.url()).pathname.endsWith('/preview'))
  await page.getByRole('button', { name: '확정 전 미리보기', exact: true }).click()
  const response = await responsePromise
  expect(response.status(), await response.text()).toBe(200)
  const checked = await response.json()
  await expect(page.getByRole('button', { name: '이 내용으로 확정', exact: true })).toBeEnabled()
  return checked
}
async function confirm(page: Page) {
  const responsePromise = page.waitForResponse(response => response.request().method() === 'POST' && new URL(response.url()).pathname.endsWith('/confirm'))
  await page.getByRole('button', { name: '이 내용으로 확정', exact: true }).click()
  const response = await responsePromise
  expect(response.status(), await response.text()).toBe(201)
  const done = await response.json()
  await expect(workItem(page, done.result.item.id).locator('.workspace-item-status')).toContainText('확정됨')
  return done
}
async function normalRestart(page: Page) {
  await page.goto('about:blank')
  let timer: ReturnType<typeof setTimeout> | undefined
  const [code, signal] = await Promise.race([
    closed,
    new Promise<never>((_, reject) => { timer = setTimeout(() => reject(new Error('workspace normal shutdown timed out')), 180_000) }),
  ]).finally(() => clearTimeout(timer))
  expect(code).toBe(0)
  expect(signal).toBeNull()
  await launch()
  await open(page)
}

test('workspace confirms sales production waste and selected stocktake with actual receipts and returns counted once', async ({ page }) => {
  await open(page)
  const raw = await material(page, '가상 모아 입력 원료'), output = await material(page, '가상 손질 결과')
  await observe(page, [stockRow(raw, '10000', '10000')])
  const menuA = await recipe(page, raw, '가상 모아 입력 메뉴 A'), menuB = await recipe(page, raw, '가상 모아 입력 메뉴 B')
  const incoming = await receipt(page, raw, '2000', '2000')
  await workspace(page)
  const sales: any[] = []
  for (const [menu, count] of [[menuA, '3'], [menuB, '1']] as const) {
    await add(page, 'sale'); await saleFields(page, menu, count); await saveNew(page); await preview(page); sales.push(await confirm(page))
  }
  expect(amounts(await holding(page, raw))).toEqual({ quantity: '12000', value: '12000' })
  await add(page, 'production'); await productionRow(page, '투입', raw, '2000'); await productionRow(page, '결과', output, '2000')
  await saveNew(page); await preview(page); await confirm(page)
  await purchaseReturn(page, incoming.result.confirmation.result.receipt_id, '1000')
  await workspace(page)
  await add(page, 'waste'); await wasteFields(page, raw, '1000'); await page.getByLabel('사용 구분', { exact: true }).selectOption('staff_meal')
  await saveNew(page); await preview(page); await confirm(page)
  expect(amounts(await holding(page, raw))).toEqual({ quantity: '8000', value: '9000' })
  await add(page, 'stocktake'); await stockFields(page, raw, '6000'); await saveNew(page)
  const checked = await preview(page), calculation = checked.effect.rows[0].calculation
  expect(calculation.consumption_quantity).toBe('2000'); expect(calculation.consumption_value).toBe('2250')
  for (const [sale, quantity, value] of [[sales[0], '1500', '1687.5'], [sales[1], '500', '562.5']] as const) {
    expect(calculation.allocation.candidates.find((candidate: any) => candidate.sale_id === sale.result.confirmation.result.sale_id)).toMatchObject({ quantity, value })
  }
  const done = await confirm(page)
  expect(amounts(await holding(page, raw))).toEqual({ quantity: '6000', value: '6750' })
  await expect(page.getByRole('link', { name: '실사와 배분 결과 보기', exact: true })).toHaveAttribute('href', `/stocktakes/${done.result.confirmation.result.stocktake_id}`)
  expect(Number(done.result.confirmation.seq)).toBeGreaterThan(0); expect(Number.isNaN(Date.parse(done.result.confirmation.recorded_at))).toBe(false)
  await page.getByRole('link', { name: '실사와 배분 결과 보기', exact: true }).click()
  await expect(page.locator('.stocktake-result')).toContainText('2,250')
})

test('workspace permits shortage and zero recipe sales and keeps blank observations distinct from measured zero', async ({ page }) => {
  await open(page)
  const raw = await material(page, '가상 부족 판매 원료'), unchecked = await material(page, '가상 미확인 유지 품목')
  await observe(page, [stockRow(raw, '0', '0'), stockRow(unchecked, '5', '5')])
  const shortageMenu = await recipe(page, raw, '가상 부족 상태 판매 메뉴', '100'), zeroMenu = await recipe(page, raw, '가상 구성 영 메뉴', '0', null)
  const before = await holding(page, unchecked)
  await workspace(page)
  await add(page, 'sale'); await saleFields(page, shortageMenu, '50'); await saveNew(page); await preview(page); await confirm(page)
  await add(page, 'sale'); await saleFields(page, zeroMenu, '0'); await saveNew(page)
  const zeroPreview = await preview(page)
  expect(zeroPreview.effect.sale.count).toBe('0'); expect(zeroPreview.effect.sale.amount).toBeNull()
  await confirm(page)
  expect(amounts(await holding(page, raw))).toEqual({ quantity: '0', value: '0' })
  await add(page, 'stocktake'); await stockFields(page, unchecked, '')
  await page.getByRole('button', { name: '실사 품목 추가', exact: true }).click(); await stockFields(page, raw, '0', 2)
  await saveNew(page); const checked = await preview(page)
  expect(checked.effect.unchecked_positions).toEqual([0]); expect(checked.effect.rows).toHaveLength(1)
  await confirm(page)
  expect((await holding(page, unchecked)).observation).toEqual(before.observation)
  expect(amounts(await holding(page, unchecked))).toEqual({ quantity: '5', value: '5' })
})

test('four unfinished database drafts survive actual normal shutdown and restart without a closing ledger', async ({ page }) => {
  test.setTimeout(280_000)
  await open(page)
  const raw = await material(page, '가상 초안 원료'), output = await material(page, '가상 초안 결과')
  await observe(page, [stockRow(raw, '10', '10'), stockRow(output, '5', '5')])
  const menu = await recipe(page, raw, '가상 초안 메뉴')
  await workspace(page)
  await add(page, 'sale'); await saleFields(page, menu, '', '가상 미완성 판매'); const sale = await saveNew(page)
  await add(page, 'production'); await productionRow(page, '투입', raw, '1.'); await productionRow(page, '결과', output, '2'); const production = await saveNew(page)
  await add(page, 'waste'); await wasteFields(page, raw, ''); const waste = await saveNew(page)
  await add(page, 'stocktake'); await stockFields(page, raw, ''); await page.getByRole('button', { name: '실사 품목 추가', exact: true }).click(); await stockFields(page, output, '0', 2); const stock = await saveNew(page)
  const before = await get(page, '/api/business?status=draft')
  expect(before.items).toHaveLength(4)
  await normalRestart(page); await workspace(page)
  for (const saved of [sale, production, waste, stock]) {
    await expect(workItem(page, saved.result.item.id).locator('.workspace-item-status')).toContainText('임시 저장됨')
    expect((await get(page, `/api/business/${saved.result.item.id}`)).head.payload).toEqual(saved.result.head.payload)
  }
  await select(page, sale.result.item.id); await expect(page.getByLabel('판매 개수', { exact: true })).toHaveValue('')
  await select(page, production.result.item.id); await expect(page.getByLabel('투입 1 실제 수량', { exact: true })).toHaveValue('1.')
  await select(page, waste.result.item.id); await expect(page.getByLabel('폐기 · 직원식 실제 수량', { exact: true })).toHaveValue('')
  await select(page, stock.result.item.id); await expect(page.getByLabel('실사 1 실제 수량', { exact: true })).toHaveValue(''); await expect(page.getByLabel('실사 2 실제 수량', { exact: true })).toHaveValue('0')
  expect(amounts(await holding(page, raw))).toEqual({ quantity: '10', value: '10' })
  expect((await get(page, '/api/business?status=draft')).items.map((item: any) => item.id).sort()).toEqual(before.items.map((item: any) => item.id).sort())
})

test('partial saved uncertain and local entries stay distinct and lost create replays its exact request after reload', async ({ page }) => {
  await open(page)
  const raw = await material(page, '가상 독립 요청 원료'), menu = await recipe(page, raw, '가상 동일 메뉴')
  await workspace(page)
  await add(page, 'sale'); await saleFields(page, menu, '1', '가상 저장 A'); const savedA = await saveNew(page)
  const localB = await add(page, 'sale'); await saleFields(page, menu, '1', '가상 결과 미확인 B')
  const bodies: any[] = []
  await page.route('**/api/business/drafts', async route => {
    bodies.push(route.request().postDataJSON())
    if (bodies.length === 1) { await route.fetch(); await route.abort('failed') } else await route.continue()
  })
  await page.getByRole('button', { name: '초안 임시 저장', exact: true }).click()
  await expect(page.getByRole('button', { name: '같은 업무 요청 재시도', exact: true })).toBeVisible()
  await expect(workItem(page, localB).locator('.workspace-item-status')).toContainText('처리 결과 확인 필요')
  const localC = await add(page, 'waste'); await wasteFields(page, raw, '2')
  await expect(workItem(page, savedA.result.item.id).locator('.workspace-item-status')).toContainText('임시 저장됨')
  await expect(workItem(page, localC).locator('.workspace-item-status')).toContainText('이 탭에만 입력')
  expect((await get(page, '/api/business?status=draft')).items).toHaveLength(2)
  await page.reload(); await select(page, localB)
  await expect(page.getByLabel('판매 메모 (선택)', { exact: true })).toHaveValue('가상 결과 미확인 B')
  const accepted = page.waitForResponse(response => response.request().method() === 'POST' && new URL(response.url()).pathname === '/api/business/drafts')
  await page.getByRole('button', { name: '같은 업무 요청 재시도', exact: true }).click()
  const response = await accepted; expect(response.status()).toBe(201)
  const savedB = await response.json(); expect(bodies).toHaveLength(2); expect(bodies[1]).toEqual(bodies[0])
  await expect(workButton(page, savedB.result.item.id)).toHaveAttribute('aria-pressed', 'true')
  expect(savedB.result.item.id).not.toBe(savedA.result.item.id)
  expect((await get(page, '/api/business?status=draft')).items).toHaveLength(2)
  await select(page, localC); await expect(page.getByLabel('폐기 · 직원식 실제 수량', { exact: true })).toHaveValue('2')
  await expect(workItem(page, localC).locator('.workspace-item-status')).toContainText('이 탭에만 입력')
})

test('uncertain JSON confirmation retains one effect and blocks dependent stocktake preview until same request recovery', async ({ page }) => {
  await open(page)
  const raw = await material(page, '가상 발행 장애 원료'); await receipt(page, raw, '10', '100')
  const menu = await recipe(page, raw, '가상 발행 장애 메뉴')
  await workspace(page)
  await add(page, 'sale'); await saleFields(page, menu, '0'); await saveNew(page); await preview(page)
  const doneA = await confirm(page)
  await add(page, 'waste'); await wasteFields(page, raw, '1'); const savedB = await saveNew(page)
  await add(page, 'stocktake'); await stockFields(page, raw, '8'); const savedC = await saveNew(page)
  await select(page, savedB.result.item.id); await preview(page)
  const next = BigInt((await get(page, '/api/state')).revision) + 1n
  const blocked = join(directory, 'db', 'snapshots', `${next.toString().padStart(20, '0')}.json`)
  mkdirSync(blocked)
  const requests: any[] = []
  await page.route(`**/api/business/${savedB.result.item.id}/confirm`, async route => { requests.push(route.request().postDataJSON()); await route.continue() })
  const failed = page.waitForResponse(response => new URL(response.url()).pathname === `/api/business/${savedB.result.item.id}/confirm`)
  await page.getByRole('button', { name: '이 내용으로 확정', exact: true }).click()
  expect((await failed).status()).toBe(503)
  await expect(page.getByRole('button', { name: '같은 업무 요청 재시도', exact: true })).toBeVisible()
  await select(page, savedC.result.item.id)
  await expect(page.getByText('다른 업무의 처리 결과를 먼저 확인한 뒤 실사를 미리보기해 주세요. 임시 저장과 입력은 계속할 수 있습니다.', { exact: true })).toBeVisible()
  await expect(page.getByRole('button', { name: '확정 전 미리보기', exact: true })).toBeDisabled()
  expect((await get(page, `/api/business/${doneA.result.item.id}`)).confirmations).toHaveLength(1)
  rmdirSync(blocked)
  await select(page, savedB.result.item.id); await page.getByRole('button', { name: '같은 업무 요청 재시도', exact: true }).click()
  await expect(workItem(page, savedB.result.item.id).locator('.workspace-item-status')).toContainText('확정됨')
  expect(requests).toHaveLength(2); expect(requests[1]).toEqual(requests[0])
  expect((await get(page, `/api/business/${savedB.result.item.id}`)).confirmations).toHaveLength(1)
  expect(JSON.parse(readFileSync(blocked, 'utf8')).request_id).toBe(requests[0].meta.request_id)
  expect(amounts(await holding(page, raw))).toEqual({ quantity: '9', value: '100' })
  await select(page, savedC.result.item.id)
  const checked = await preview(page)
  expect(checked.effect.rows[0].calculation.consumption_quantity).toBe('1')
  expect((await get(page, `/api/business/${savedC.result.item.id}`)).confirmations).toHaveLength(0)
})

test('external state changes require explicit review and saved workspace drafts share identity with business detail', async ({ page, context }) => {
  await open(page)
  const raw = await material(page, '가상 다른 탭 원료'); await observe(page, [stockRow(raw, '10', '10')])
  const menu = await recipe(page, raw, '가상 왕복 메뉴')
  await workspace(page); await add(page, 'stocktake'); await stockFields(page, raw, '6'); const saved = await saveNew(page)
  const old = await preview(page); expect(old.effect.rows[0].calculation.consumption_quantity).toBe('4')
  const other = await context.newPage(); await other.goto(launchUrl); await expect(other.getByLabel('재료 이름', { exact: false })).toBeEnabled()
  await receipt(other, raw, '2', '2'); await other.close()
  const failed = page.waitForResponse(response => new URL(response.url()).pathname === `/api/business/${saved.result.item.id}/confirm`)
  await page.getByRole('button', { name: '이 내용으로 확정', exact: true }).click()
  const rejected = await failed; expect(rejected.status()).toBe(409); expect((await rejected.json()).code).toBe('stale_state')
  const rejectedBody = rejected.request().postDataJSON()
  await expect(page.getByLabel('실사 1 실제 수량', { exact: true })).toHaveValue('6')
  await page.getByRole('button', { name: '최신 기록 확인 후 입력 검토', exact: true }).click()
  const current = await preview(page); expect(current.effect.rows[0].calculation.consumption_quantity).toBe('6')
  const accepted = page.waitForResponse(response => new URL(response.url()).pathname === `/api/business/${saved.result.item.id}/confirm`)
  await confirm(page); const acceptedBody = (await accepted).request().postDataJSON()
  expect(acceptedBody.meta.request_id).not.toBe(rejectedBody.meta.request_id)
  expect(acceptedBody.meta.expected_revision).not.toBe(rejectedBody.meta.expected_revision)
  await add(page, 'sale'); await saleFields(page, menu, '1'); const sale = await saveNew(page)
  await page.getByLabel('판매 개수', { exact: true }).fill('2')
  await page.getByRole('link', { name: '원업무 기록 보기', exact: true }).click()
  await expect(page).toHaveURL(new URL(`/business/${sale.result.item.id}`, launchUrl).toString())
  await expect(page.getByLabel('판매 개수', { exact: true })).toHaveValue('2')
  await page.getByRole('button', { name: '수정 내용 임시 저장', exact: true }).click()
  await expect(page.getByText('수정 내용을 임시 저장했습니다.', { exact: true })).toBeVisible()
  await workspace(page); await select(page, sale.result.item.id)
  await expect(page.getByLabel('판매 개수', { exact: true })).toHaveValue('2')
  expect((await get(page, `/api/business/${sale.result.item.id}`)).head.payload.data.count).toBe('2')
})

test('same-kind local editors remain independent when a late create completes and preserve focus with six layouts', async ({ page }) => {
  await open(page)
  const raw = await material(page, '가상 서로 독립인 긴 이름의 원료'), menu = await recipe(page, raw, '가상 여러 입력의 선택과 저장 상태를 구분하는 긴 메뉴 이름')
  await workspace(page)
  const localA = await add(page, 'sale'); await saleFields(page, menu, '3', '가상 늦은 저장 A')
  const localB = await add(page, 'sale'); await saleFields(page, menu, '5', '가상 현재 입력 B')
  await select(page, localA); await expect(page.getByLabel('판매 개수', { exact: true })).toHaveValue('3')
  let release!: () => void, arrived = false, accepted: any
  const gate = new Promise<void>(resolveGate => { release = resolveGate })
  const completions: Promise<void>[] = []
  await page.route('**/api/business/drafts', route => {
    const completion = (async () => { const response = await route.fetch(); accepted = await response.json(); arrived = true; await gate; await route.fulfill({ response }) })()
    completions.push(completion); return completion
  })
  try {
    await page.getByRole('button', { name: '초안 임시 저장', exact: true }).click()
    await expect.poll(() => arrived).toBe(true)
    await select(page, localB); await expect(page.getByLabel('판매 개수', { exact: true })).toHaveValue('5')
  } finally { release(); await Promise.all(completions); await page.unroute('**/api/business/drafts') }
  await expect(workButton(page, localB)).toHaveAttribute('aria-pressed', 'true')
  await expect(page.getByLabel('판매 메모 (선택)', { exact: true })).toHaveValue('가상 현재 입력 B')
  await expect(workItem(page, accepted.result.item.id).locator('.workspace-item-status')).toContainText('임시 저장됨')
  expect((await get(page, `/api/business/${accepted.result.item.id}`)).head.payload.data.count).toBe('3')
  await page.getByRole('button', { name: '작업 목록으로', exact: true }).click()
  await expect(workButton(page, localB)).toBeFocused()
  await workButton(page, accepted.result.item.id).focus(); await page.keyboard.press('Enter')
  await expect(page.locator('.workspace-editor-heading')).toBeFocused()
  await expect(page.getByLabel('판매 개수', { exact: true })).toHaveValue('3')
  expect(await page.locator('[id]').evaluateAll(nodes => { const ids = nodes.map(node => node.id); return ids.filter((id, index) => ids.indexOf(id) !== index) })).toEqual([])
  for (const [width, height] of [[1440, 900], [768, 1024], [390, 844]] as const) {
    await page.setViewportSize({ width, height })
    for (const theme of ['light', 'dark'] as const) {
      if (await page.locator('html').getAttribute('data-theme') !== theme) await page.getByRole('button', { name: theme === 'dark' ? '다크 모드로 전환' : '라이트 모드로 전환' }).click()
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true)
      await page.screenshot({ path: test.info().outputPath(`workspace-${width}-${theme}.png`), fullPage: true })
    }
  }
})
