import { test, expect, type Page } from '@playwright/test'
import { spawn, type ChildProcess } from 'node:child_process'
import { once } from 'node:events'
import { randomUUID } from 'node:crypto'
import { cpSync, mkdtempSync, readdirSync, rmSync, readFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve, sep } from 'node:path'

const binaryName = process.platform === 'win32' ? 'sunchang-hansang.exe' : 'sunchang-hansang'
let directory: string, server: ChildProcess, closed: Promise<[number | null, NodeJS.Signals | null]>, launchUrl: string
test.beforeEach(async () => {
  directory = mkdtempSync(join(tmpdir(), 'sunchang-bulk-'))
  const binary = join(directory, binaryName); cpSync(resolve('..', 'target', 'release', binaryName), binary)
  server = spawn(binary, ['--no-browser'], { cwd: directory, stdio: ['ignore', 'pipe', 'pipe'] }); closed = once(server, 'close') as typeof closed
  launchUrl = await new Promise<string>((resolveUrl, reject) => {
    let output = '', errors = ''; const timer = setTimeout(() => reject(Error(`bulk fixture start timed out: ${errors}`)), 20_000)
    server.stderr!.on('data', data => { errors += String(data) }); server.once('exit', code => { clearTimeout(timer); reject(Error(`bulk fixture exited ${code}: ${errors}`)) })
    server.stdout!.on('data', data => { output += String(data); const match = output.match(/http:\/\/127\.0\.0\.1:\d+\/#token=[a-f0-9]+/); if (match) { clearTimeout(timer); resolveUrl(match[0]) } })
  })
})
test.afterEach(async () => {
  if (server?.exitCode === null && server.signalCode === null) server.kill()
  if (closed) await closed
  if (directory) { expect(resolve(directory).startsWith(`${resolve(tmpdir())}${sep}`)).toBe(true); rmSync(directory, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 }) }
})
test.setTimeout(180_000)
async function open(page: Page) { await page.goto(launchUrl); await expect(page.getByLabel('재료 이름', { exact: false })).toBeEnabled(); await bulk(page) }
async function bulk(page: Page) { await page.goto(new URL('/bulk', launchUrl).toString()); await expect(page.getByRole('heading', { name: '일괄 입력', exact: true })).toBeVisible(); await expect(page.getByRole('button', { name: '등록 내용 확인', exact: true })).toBeEnabled() }
async function get(page: Page, path: string) { const response = await page.request.get(new URL(path, launchUrl).toString()); expect(response.ok(), await response.text()).toBe(true); return response.json() }
async function post(page: Page, path: string, data: unknown) { const response = await page.request.post(new URL(path, launchUrl).toString(), { headers: { Origin: new URL(launchUrl).origin }, data }); expect(response.status(), await response.text()).toBe(201); return response.json() }
async function meta(page: Page) { const state = await get(page, '/api/state'); return { request_id: randomUUID(), epoch: state.epoch, expected_revision: state.revision } }
async function material(page: Page, name: string) { return post(page, '/api/materials', { ...await meta(page), name, unit: 'g', kind: 'raw', description: '' }) }
async function input(page: Page, row: number, name: string, value: string) { await page.getByLabel(`${row}행 ${name}`, { exact: true }).fill(value) }
async function csv(page: Page, raw: string) { await page.getByLabel('UTF-8 CSV 업로드', { exact: true }).setInputFiles({ name: 'virtual.csv', mimeType: 'text/csv', buffer: Buffer.from(raw) }); await expect(page.getByText('확인 전에는 재료·메뉴·입고를 등록하지 않습니다.', { exact: true })).toBeVisible() }
async function prepare(page: Page) { await page.getByRole('button', { name: '등록 내용 확인', exact: true }).click(); await expect(page.getByText('아래 내용과 참조를 확인한 뒤 등록해 주세요.', { exact: true })).toBeVisible() }
const submit = (page: Page) => page.locator('.bulk-submit')
async function run(page: Page) { await expect(submit(page)).toBeEnabled(); await submit(page).click(); await expect(page.getByText('일괄 등록을 마쳤습니다. 아래 결과에서 실제 저장한 기록을 확인하세요.', { exact: true })).toBeVisible() }
async function downloadTemplate(page: Page, name: string) { const downloading = page.waitForEvent('download'); await page.getByRole('button', { name: `${name} CSV 양식 내려받기`, exact: true }).click(); return readFileSync((await (await downloading).path())!, 'utf8') }
const receiptHeader = '재료 ID,수량,단위,구입금액,입고일,메모,재고 반영,실사 버전 ID,포함 수량\n'
function journals() { return readdirSync(join(directory, 'db', 'snapshots')).filter(name => name.endsWith('.json')) }

test('CSV template and editable preview register only the corrected material strings', async ({ page }) => {
  await open(page)
  const template = await downloadTemplate(page, '재료'); expect(template).toContain('이름,단위,종류,설명'); expect(template).toContain('가상 재료,g,raw,')
  await csv(page, '이름,단위,종류,설명\n"가상, 업로드",g,raw,"첫 줄\n""둘째"" 줄"')
  await input(page, 1, '이름', '<b>가상 최종 수정</b>')
  expect(await get(page, '/api/materials')).toHaveLength(0)
  await prepare(page); expect(await get(page, '/api/materials')).toHaveLength(0)
  await input(page, 1, '설명', '=1+1\n수식 모양도 문자열')
  await expect(submit(page)).toBeDisabled(); await prepare(page); await run(page)
  const list = await get(page, '/api/materials'); expect(list).toHaveLength(1); expect(list[0].name).toBe('<b>가상 최종 수정</b>'); expect(list[0].description).toBe('=1+1\n수식 모양도 문자열'); expect(journals()).toHaveLength(1)
  await page.getByRole('link', { name: '등록한 재료 보기', exact: true }).click(); await expect(page.getByRole('heading', { name: '<b>가상 최종 수정</b>', exact: true })).toBeVisible()
})

test('direct rows, tab paste, keyboard, cancellation and reload preserve input in six layouts', async ({ page }) => {
  await open(page); await input(page, 1, '이름', '가상 직접 입력'); await page.getByRole('button', { name: '행 추가', exact: true }).click(); await input(page, 2, '이름', '삭제할 행'); await page.getByRole('button', { name: '2행 삭제', exact: true }).click(); await expect(page.locator('[data-bulk-row]')).toHaveCount(1)
  await page.locator('.bulk-paste summary').click(); await page.getByLabel('제목 없이 위 열 순서로 복사한 셀', { exact: true }).fill('가상 붙여넣기\tg\traw\t첫 설명\n가상 둘째\tkg\tprepared\t')
  await page.getByRole('button', { name: '붙여넣은 값으로 표 만들기', exact: true }).click(); await expect(page.locator('[data-bulk-row]')).toHaveCount(2)
  await page.getByLabel('1행 이름', { exact: true }).focus(); await page.keyboard.press('Tab'); await expect(page.getByLabel('1행 단위', { exact: true })).toBeFocused()
  await page.keyboard.press('Shift+Tab'); await expect(page.getByLabel('1행 이름', { exact: true })).toBeFocused()
  await page.reload(); await expect(page.getByLabel('1행 이름', { exact: true })).toHaveValue('가상 붙여넣기'); await prepare(page); await page.getByRole('button', { name: '확인 취소', exact: true }).click(); await expect(submit(page)).toBeDisabled(); expect(await get(page, '/api/materials')).toHaveLength(0)
  for (const [width,height] of [[1440,900],[768,1024],[390,844]] as const) {
    await page.setViewportSize({ width,height })
    for (const theme of ['light','dark'] as const) {
      if (await page.locator('html').getAttribute('data-theme') !== theme) await page.getByRole('button', { name: theme === 'dark' ? '다크 모드로 전환' : '라이트 모드로 전환' }).click()
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true)
      await expect(page.getByLabel('1행 이름', { exact: true })).toBeEditable()
      await page.screenshot({ path: test.info().outputPath(`bulk-${width}-${theme}.png`), fullPage: true })
    }
  }
})

test('menu groups preserve multiple components, zero quantity, basis count and absent versus zero selling price', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 배합 재료')
  await page.getByLabel('등록 대상', { exact: true }).selectOption('menu')
  const template = await downloadTemplate(page, '메뉴'); expect(template).toContain('메뉴 묶음,이름,단위,설명,기준 개수,판매가,구성 재료 ID,구성 수량,구성 단위,구성 메모'); expect(template).toContain('가상-A,가상 메뉴,개,,1,0')
  await csv(page, `메뉴 묶음,이름,단위,설명,기준 개수,판매가,구성 재료 ID,구성 수량,구성 단위,구성 메모\nA,가상 배치,개,,3,0,${raw.id},100,g,배합\nA,가상 배치,개,,3,0,${raw.id},0,g,적당량\nB,가상 무가격,개,,1,,,,,`)
  await prepare(page); expect((await get(page, '/api/recipes')).items).toHaveLength(0); await run(page)
  const recipes = (await get(page, '/api/recipes')).items; expect(recipes).toHaveLength(2)
  const first = await get(page, `/api/recipes/${recipes.find((row: any) => row.item.name === '가상 배치').item.id}`)
  expect(first.head.basis_count).toBe('3'); expect(first.head.selling_price).toBe('0'); expect(first.head.lines.map((row: any) => [row.material_id,row.material_version_id,row.input_quantity])).toEqual([[raw.id,raw.version_id,'100'],[raw.id,raw.version_id,'0']])
  const second = await get(page, `/api/recipes/${recipes.find((row: any) => row.item.name === '가상 무가격').item.id}`); expect(second.head.selling_price).toBeNull(); expect(second.head.lines).toHaveLength(0)
})

test('receipt errors and edited zero price preview cause no writes before normal confirmation', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 입고 수정'); const before = await get(page, '/api/state'), count = journals().length
  await page.getByLabel('등록 대상', { exact: true }).selectOption('receipt'); const template = await downloadTemplate(page, '입고'); expect(template).toContain(receiptHeader.trim()); expect(template).toContain('11111111-1111-4111-8111-111111111111,100,g,0,2026-10-20,가상 예제'); await csv(page, receiptHeader + `${raw.id},300,L,,2024-02-29,가상,unreflected,,`)
  await page.getByRole('button', { name: '등록 내용 확인', exact: true }).click(); await expect(page.getByLabel('1행 구입금액', { exact: true })).toHaveAttribute('aria-invalid', 'true'); await expect(submit(page)).toBeDisabled(); expect(await get(page, '/api/state')).toEqual(before)
  await input(page, 1, '수량', '250'); await input(page, 1, '단위', 'g'); await input(page, 1, '구입금액', '0'); await prepare(page); expect(await get(page, '/api/state')).toEqual(before); expect(journals()).toHaveLength(count)
  await run(page); const list = await get(page, '/api/receipts'); expect(list.items).toHaveLength(1); expect(list.items[0].input_quantity).toBe('250'); expect(list.items[0].amount).toBe('0'); expect(list.items[0].receipt_date).toBe('2024-02-29'); expect(list.items[0].recorded_at.startsWith('2024-02-29')).toBe(false)
  const holding = await get(page, `/api/holdings/${raw.id}`); expect(holding.settlement_basis.quantity).toBe('250'); expect(holding.settlement_basis.value).toBe('0')
})

test('lost material and receipt confirmation responses replay immutable requests after reload without duplicates', async ({ page }) => {
  await open(page); await csv(page, '이름,단위,종류,설명\n가상 재시도1,g,raw,\n가상 재시도2,g,raw,'); await prepare(page)
  const attempts: unknown[] = []
  await page.route('**/api/materials', async route => { if (route.request().method() !== 'POST') return route.continue(); attempts.push(route.request().postDataJSON()); if (attempts.length === 1) { await route.fetch(); await route.abort('failed') } else await route.continue() })
  await submit(page).click(); await expect(page.getByText('등록이 멈췄습니다.', { exact: false })).toBeVisible(); await page.reload(); await expect(submit(page)).toBeEnabled(); await run(page); expect(attempts[0]).toEqual(attempts[1]); expect(await get(page, '/api/materials')).toHaveLength(2); await page.unroute('**/api/materials')
  const raw = (await get(page, '/api/materials'))[0]
  await page.getByLabel('등록 대상', { exact: true }).selectOption('receipt'); await csv(page, receiptHeader + `${raw.id},10,g,0,2024-02-29,,unreflected,,`); await prepare(page)
  const confirmations: unknown[] = []
  await page.route('**/api/business/*/confirm', async route => { confirmations.push(route.request().postDataJSON()); if (confirmations.length === 1) { await route.fetch(); await route.abort('failed') } else await route.continue() })
  await submit(page).click(); await expect(page.getByText('등록이 멈췄습니다.', { exact: false })).toBeVisible(); await page.reload(); await expect(submit(page)).toBeEnabled(); await run(page)
  expect(confirmations[0]).toEqual(confirmations[1]); expect((await get(page, '/api/receipts')).items).toHaveLength(1); expect((await get(page, `/api/holdings/${raw.id}`)).settlement_basis.quantity).toBe('10')
})

test('late receipt inclusion editor prevents adding already observed quantity and rejects reused complete shares', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 실사 포함')
  const draft = await post(page, '/api/stocktake-drafts', { meta: await meta(page), payload: { kind: 'stocktake', version: 1, data: { rows: [{ material_id: raw.id, material_version_id: raw.version_id, quantity: '20', unit: 'g', initial_value: { kind: 'manual_amount', amount: '20' }, excess: null }], note: '' } }, corrects_confirmation_id: null })
  const confirmed = await post(page, `/api/business/${draft.result.item.id}/confirm`, { meta: await meta(page), expected_head_id: draft.result.head.id }), stock = await get(page, `/api/stocktakes/${confirmed.result.confirmation.result.stocktake_id}`)
  await page.getByLabel('등록 대상', { exact: true }).selectOption('receipt'); await csv(page, receiptHeader + `${raw.id},20,g,40,2024-02-29,,reconciled,,`)
  await page.getByRole('button', { name: '참조 목록 다시 불러오기', exact: true }).click(); await expect(page.getByRole('button', { name: '1행 실사 포함 근거 편집', exact: true })).toBeEnabled()
  await page.getByRole('button', { name: '1행 실사 포함 근거 편집', exact: true }).click(); await page.locator('#receipt-inclusion-0-observation_version_id').selectOption(stock.observations[0].version_id); await page.locator('#receipt-inclusion-0-source').selectOption('all'); await page.locator('#receipt-inclusion-0-included_quantity').fill('20')
  await prepare(page); await run(page); let holding = await get(page, `/api/holdings/${raw.id}`); expect(holding.settlement_basis.quantity).toBe('20'); expect(holding.settlement_basis.value).toBe('40')
  await page.getByRole('button', { name: '원입력 보존 후 빈 새 입력', exact: true }).click(); await csv(page, receiptHeader + `${raw.id},20,g,40,2024-02-29,,reconciled,${stock.observations[0].version_id},20`)
  await page.getByRole('button', { name: '등록 내용 확인', exact: true }).click(); await expect(submit(page)).toBeDisabled(); expect((await get(page, '/api/receipts')).items).toHaveLength(1); holding = await get(page, `/api/holdings/${raw.id}`); expect(holding.settlement_basis.quantity).toBe('20')
})

test('same-material later effects stop a partial batch and reupload links its completed results', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 두 입고'), contents = receiptHeader + `${raw.id},10,g,0,2024-02-29,,unreflected,,\n${raw.id},20,g,0,2024-02-29,,unreflected,,`
  await page.getByLabel('등록 대상', { exact: true }).selectOption('receipt'); await csv(page, contents); await prepare(page); await page.getByLabel('중복·같은 이름·순차 등록 안내를 확인했습니다', { exact: true }).check(); await submit(page).click()
  await expect(page.getByRole('heading', { name: '변경된 입고 효과', exact: true })).toBeVisible(); expect((await get(page, '/api/receipts')).items).toHaveLength(1); await expect(submit(page)).toBeDisabled()
  await page.getByRole('button', { name: '변경된 미리보기 확인', exact: true }).click(); await run(page); expect((await get(page, '/api/receipts')).items).toHaveLength(2); expect((await get(page, `/api/holdings/${raw.id}`)).settlement_basis.quantity).toBe('30')
  await page.getByLabel('UTF-8 CSV 업로드', { exact: true }).setInputFiles({ name: 'same-virtual.csv', mimeType: 'text/csv', buffer: Buffer.from(contents) }); await expect(page.getByRole('region', { name: '같은 내용 재업로드', exact: true })).toBeVisible(); await expect(page.locator('.bulk-results a[href^="/receipts/"]')).toHaveCount(2); expect((await get(page, '/api/receipts')).items).toHaveLength(2)
  await page.getByRole('button', { name: '같은 내용을 별도 새 입력으로 준비', exact: true }).click(); await expect(page.locator('.bulk-results')).toHaveCount(0); expect((await get(page, '/api/receipts')).items).toHaveLength(2)
})

test('late success after another tab restores preserves the old bulk request without reapplying it', async ({ page, context }) => {
  await open(page); await material(page, '가상 복원 기준'); const target = (await get(page, '/api/restore-targets?source=journal')).rows[0]
  await input(page, 1, '이름', '가상 복원 전 늦은 등록'); await prepare(page)
  let release!: () => void, fetched!: () => void, original: unknown
  const held = new Promise<void>(resolveDone => { release = resolveDone }), reached = new Promise<void>(resolveDone => { fetched = resolveDone })
  await page.route('**/api/materials', async route => { if (route.request().method() !== 'POST') return route.continue(); original = route.request().postDataJSON(); const response = await route.fetch(); fetched(); await held; await route.fulfill({ response }) })
  await submit(page).click(); await reached
  const other = await context.newPage(); await other.goto(new URL('/restore', launchUrl).toString()); await expect(other.locator(`[data-restore-target="${target.target_id}"]`)).toBeVisible(); await other.locator(`[data-restore-target="${target.target_id}"]`).click(); await other.getByRole('button', { name: '전체 영향 미리보기', exact: true }).click(); await expect(other.getByRole('button', { name: '선택한 전체 상태로 복원 확정', exact: true })).toBeEnabled(); await other.getByRole('button', { name: '선택한 전체 상태로 복원 확정', exact: true }).click(); await expect(other.getByRole('heading', { name: '전체 상태 복원 완료', exact: true })).toBeVisible(); await expect(page.locator('.restore-banner')).toBeVisible(); release()
  await expect(page.getByText('복원 전에 처리된 요청입니다.', { exact: false })).toBeVisible(); await expect(submit(page)).toBeDisabled(); const preserved = JSON.parse((await page.evaluate(() => sessionStorage.getItem('sunchang.bulk.session.v1')))!)
  expect(preserved.review.items[0].pending.body).toEqual(original); expect((await get(page, '/api/materials')).map((item: any) => item.name)).toEqual(['가상 복원 기준'])
  await page.getByRole('navigation', { name: '주요 메뉴' }).getByRole('link', { name: '자료 복원', exact: true }).click(); await expect(page.getByRole('heading', { name: '보존한 입력·요청', exact: true })).toBeVisible(); const texts = await page.locator('#retained-inputs textarea').evaluateAll(elements => elements.map(element => (element as HTMLTextAreaElement).value)); expect(texts.some(value => value.includes('가상 복원 전 늦은 등록'))).toBe(true); await other.close()
})

test('lost response followed by restore can explicitly preserve the old request and start an empty new batch', async ({ page, context }) => {
  await open(page); await material(page, '가상 복원 기준'); const target = (await get(page, '/api/restore-targets?source=journal')).rows[0]
  await input(page, 1, '이름', '가상 응답 유실 보존'); await prepare(page)
  const requests: any[] = []
  await page.route('**/api/materials', async route => { if (route.request().method() !== 'POST') return route.continue(); requests.push(route.request().postDataJSON()); if (requests.length === 1) { await route.fetch(); await route.abort('failed') } else await route.continue() })
  await submit(page).click(); await expect(page.getByText('등록이 멈췄습니다.', { exact: false })).toBeVisible()
  const other = await context.newPage(); await other.goto(new URL('/restore', launchUrl).toString()); await other.locator(`[data-restore-target="${target.target_id}"]`).click(); await other.getByRole('button', { name: '전체 영향 미리보기', exact: true }).click(); await expect(other.getByRole('button', { name: '선택한 전체 상태로 복원 확정', exact: true })).toBeEnabled(); await other.getByRole('button', { name: '선택한 전체 상태로 복원 확정', exact: true }).click(); await expect(other.getByRole('heading', { name: '전체 상태 복원 완료', exact: true })).toBeVisible(); await expect(page.locator('.restore-banner')).toBeVisible()
  await expect(submit(page)).toBeDisabled(); await page.getByRole('button', { name: '원입력 보존 후 빈 새 입력', exact: true }).click(); await expect(page.getByLabel('1행 이름', { exact: true })).toHaveValue(''); expect(requests).toHaveLength(1)
  const retained = await page.evaluate(() => Object.keys(sessionStorage).filter(key => key.startsWith('sunchang.bulk.retained.')).map(key => sessionStorage.getItem(key)!))
  expect(retained.some(raw => raw.includes('가상 응답 유실 보존') && raw.includes(requests[0].request_id))).toBe(true)
  await input(page, 1, '이름', '가상 명시적 새 입력'); await prepare(page); await run(page); expect(requests).toHaveLength(2); expect(requests[1].request_id).not.toBe(requests[0].request_id); expect(requests[1].epoch).not.toBe(requests[0].epoch); expect((await get(page, '/api/materials')).map((row: any) => row.name)).toEqual(expect.arrayContaining(['가상 복원 기준','가상 명시적 새 입력'])); await other.close()
})

test('externally revised or confirmed receipt drafts are reviewed without stale-head loops or duplicate receipts', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 초안 외부 수정')
  await page.getByLabel('등록 대상', { exact: true }).selectOption('receipt'); await csv(page, receiptHeader + `${raw.id},10,g,0,2024-02-29,,unreflected,,\n${raw.id},20,g,0,2024-02-29,,unreflected,,`); await prepare(page); await page.getByLabel('중복·같은 이름·순차 등록 안내를 확인했습니다', { exact: true }).check(); await submit(page).click(); await expect(page.getByRole('heading', { name: '변경된 입고 효과', exact: true })).toBeVisible()
  const stored = JSON.parse((await page.evaluate(() => sessionStorage.getItem('sunchang.bulk.session.v1')))!), draftId = stored.review.items[1].draft.id
  const draft = await get(page, `/api/business/${draftId}`)
  await post(page, `/api/business/${draftId}/draft-revisions`, { meta: await meta(page), expected_head_id: draft.head.id, payload: { ...draft.head.payload, data: { ...draft.head.payload.data, amount: '5' } } })
  await page.getByRole('button', { name: '변경된 미리보기 확인', exact: true }).click(); await submit(page).click(); await expect(page.getByRole('button', { name: '최신 자료 확인 후 입력 재검토', exact: true })).toBeVisible(); await page.getByRole('button', { name: '최신 자료 확인 후 입력 재검토', exact: true }).click(); await expect(page.getByRole('button', { name: '현재 초안 내용으로 표 갱신', exact: true })).toBeVisible(); await expect(page.getByLabel('2행 구입금액', { exact: true })).toHaveValue('0')
  await page.getByRole('button', { name: '현재 초안 내용으로 표 갱신', exact: true }).click(); await expect(page.getByLabel('2행 구입금액', { exact: true })).toHaveValue('5'); await prepare(page); await run(page); expect((await get(page, '/api/receipts')).items).toHaveLength(2); expect((await get(page, `/api/receipts/${JSON.parse((await page.evaluate(() => sessionStorage.getItem('sunchang.bulk.session.v1')))!).review.items[1].result.id}`)).item.amount).toBe('5')
  await page.getByRole('button', { name: '원입력 보존 후 빈 새 입력', exact: true }).click(); await csv(page, receiptHeader + `${raw.id},1,g,0,2024-02-29,,unreflected,,`); await prepare(page)
  await page.route('**/api/business/*/preview', route => route.abort('failed')); await submit(page).click(); await expect(page.getByText('등록이 멈췄습니다.', { exact: false })).toBeVisible(); await page.unroute('**/api/business/*/preview')
  const paused = JSON.parse((await page.evaluate(() => sessionStorage.getItem('sunchang.bulk.session.v1')))!), externalId = paused.review.items[0].draft.id, external = await get(page, `/api/business/${externalId}`)
  await post(page, `/api/business/${externalId}/confirm`, { meta: await meta(page), expected_head_id: external.head.id }); await submit(page).click(); await page.getByRole('button', { name: '최신 자료 확인 후 입력 재검토', exact: true }).click(); await expect(page.getByText('다른 화면에서 이미 확정한 입고 결과를 연결했습니다.', { exact: false })).toBeVisible(); await expect(page.locator('.bulk-results a[href^="/receipts/"]')).toHaveCount(1); expect((await get(page, '/api/receipts')).items).toHaveLength(3)
})

test('manual correction after an external draft review clears the old action and freezes the completed result', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 초안 수동 수정')
  await page.getByLabel('등록 대상', { exact: true }).selectOption('receipt'); await csv(page, receiptHeader + `${raw.id},10,g,0,2024-02-29,,unreflected,,\n${raw.id},20,g,0,2024-02-29,,unreflected,,`); await prepare(page); await page.getByLabel('중복·같은 이름·순차 등록 안내를 확인했습니다', { exact: true }).check(); await submit(page).click(); await expect(page.getByRole('heading', { name: '변경된 입고 효과', exact: true })).toBeVisible()
  const partial = JSON.parse((await page.evaluate(() => sessionStorage.getItem('sunchang.bulk.session.v1')))!), oldDraftId = partial.review.items[1].draft.id, oldDraft = await get(page, `/api/business/${oldDraftId}`)
  await post(page, `/api/business/${oldDraftId}/draft-revisions`, { meta: await meta(page), expected_head_id: oldDraft.head.id, payload: { ...oldDraft.head.payload, data: { ...oldDraft.head.payload.data, amount: '5' } } })
  await page.getByRole('button', { name: '변경된 미리보기 확인', exact: true }).click(); await submit(page).click(); await page.getByRole('button', { name: '최신 자료 확인 후 입력 재검토', exact: true }).click(); await expect(page.getByRole('button', { name: '현재 초안 내용으로 표 갱신', exact: true })).toBeVisible()
  await input(page, 2, '구입금액', '7'); await prepare(page); await expect(page.getByRole('button', { name: '현재 초안 내용으로 표 갱신', exact: true })).toHaveCount(0); await run(page)
  const completed = JSON.parse((await page.evaluate(() => sessionStorage.getItem('sunchang.bulk.session.v1')))!), item = completed.review.items[1], receipt = await get(page, `/api/receipts/${item.result.id}`)
  expect(item.draft.id).not.toBe(oldDraftId); expect(item.command.payload.data.amount).toBe('7'); expect(completed.rows[1].values.amount).toBe('7'); expect(receipt.item.amount).toBe('7'); await expect(page.getByLabel('2행 구입금액', { exact: true })).not.toBeEditable(); await expect(page.getByRole('button', { name: '현재 초안 내용으로 표 갱신', exact: true })).toHaveCount(0)
  const preserved = await get(page, `/api/business/${oldDraftId}`); expect(preserved.item.status).toBe('draft'); expect(preserved.head.payload.data.amount).toBe('5'); expect((await get(page, '/api/receipts')).items).toHaveLength(2)
})

test('an explicitly applied incomplete external receipt survives reload and can be corrected without automatic writes', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 미완성 초안 보존')
  await page.getByLabel('등록 대상', { exact: true }).selectOption('receipt'); await csv(page, receiptHeader + `${raw.id},10,g,0,2024-02-29,,unreflected,,\n${raw.id},20,g,0,2024-02-29,,unreflected,,`); await prepare(page); await page.getByLabel('중복·같은 이름·순차 등록 안내를 확인했습니다', { exact: true }).check(); await submit(page).click(); await expect(page.getByRole('heading', { name: '변경된 입고 효과', exact: true })).toBeVisible()
  const partial = JSON.parse((await page.evaluate(() => sessionStorage.getItem('sunchang.bulk.session.v1')))!), draftId = partial.review.items[1].draft.id, draft = await get(page, `/api/business/${draftId}`)
  const effect = { kind: 'needs_reconciliation', basis_id: null, included_quantity: '' }
  await post(page, `/api/business/${draftId}/draft-revisions`, { meta: await meta(page), expected_head_id: draft.head.id, payload: { ...draft.head.payload, data: { ...draft.head.payload.data, note: '가상 확인 필요 원문', stock_effect: effect } } })
  await page.getByRole('button', { name: '변경된 미리보기 확인', exact: true }).click(); await submit(page).click(); await page.getByRole('button', { name: '최신 자료 확인 후 입력 재검토', exact: true }).click(); await page.getByRole('button', { name: '현재 초안 내용으로 표 갱신', exact: true }).click()
  let writes = 0
  await page.route('**/api/business/**', route => { if (route.request().method() === 'POST' && !new URL(route.request().url()).pathname.endsWith('/preview')) writes++; return route.continue() })
  await page.reload(); await expect(page.getByLabel('2행 재고 반영', { exact: true })).toHaveValue('needs_reconciliation'); await expect(page.getByLabel('2행 메모', { exact: true })).toHaveValue('가상 확인 필요 원문'); await expect(page.getByText('보관한 입력을 안전하게 읽지 못했습니다.', { exact: false })).toHaveCount(0); await expect(submit(page)).toBeDisabled()
  const recovered = JSON.parse((await page.evaluate(() => sessionStorage.getItem('sunchang.bulk.session.v1')))!)
  expect(recovered.review.items[1].command.payload.data.stock_effect).toEqual(effect); expect(recovered.review.items[1].pending).toBeNull(); expect(recovered.review.items[0].result).toEqual(partial.review.items[0].result); expect(writes).toBe(0); expect((await get(page, '/api/receipts')).items).toHaveLength(1)
  await page.getByLabel('2행 재고 반영', { exact: true }).selectOption('unreflected'); await prepare(page); await run(page); expect(writes).toBe(2); expect((await get(page, '/api/receipts')).items).toHaveLength(2); const original = await get(page, `/api/business/${draftId}`); expect(original.item.status).toBe('draft'); expect(original.head.payload.data.stock_effect).toEqual(effect)
})

test('an externally order-linked receipt explains the unsupported table action and continues in its preserved draft', async ({ page }) => {
  await open(page); const raw = await material(page, '가상 외부 주문 초안')
  await page.getByLabel('등록 대상', { exact: true }).selectOption('receipt'); await csv(page, receiptHeader + `${raw.id},10,g,0,2024-02-29,,unreflected,,\n${raw.id},20,g,0,2024-02-29,,unreflected,,`); await prepare(page); await page.getByLabel('중복·같은 이름·순차 등록 안내를 확인했습니다', { exact: true }).check(); await submit(page).click(); await expect(page.getByRole('heading', { name: '변경된 입고 효과', exact: true })).toBeVisible()
  const partial = JSON.parse((await page.evaluate(() => sessionStorage.getItem('sunchang.bulk.session.v1')))!), draftId = partial.review.items[1].draft.id, draft = await get(page, `/api/business/${draftId}`)
  const source = { kind: 'order', order_id: null, expected_order_version_id: null, material_version_id: null }
  await post(page, `/api/business/${draftId}/draft-revisions`, { meta: await meta(page), expected_head_id: draft.head.id, payload: { ...draft.head.payload, data: { ...draft.head.payload.data, source, note: '가상 주문 초안 원문' } } })
  await page.getByRole('button', { name: '변경된 미리보기 확인', exact: true }).click(); await submit(page).click(); await page.getByRole('button', { name: '최신 자료 확인 후 입력 재검토', exact: true }).click()
  await expect(page.getByText('발주에 연결된 입고 초안은 이 표에 적용할 수 없습니다. 초안 화면에서 계속해 주세요.', { exact: true })).toBeVisible()
  await expect(page.getByRole('button', { name: '현재 초안 내용으로 표 갱신', exact: true })).toHaveCount(0)
  const retained = JSON.parse((await page.evaluate(() => sessionStorage.getItem('sunchang.bulk.session.v1')))!)
  expect(retained.rows[1].values).toEqual(partial.rows[1].values); expect(retained.review.items[1].command).toEqual(partial.review.items[1].command); expect(retained.review.items[0].result).toEqual(partial.review.items[0].result); expect((await get(page, '/api/receipts')).items).toHaveLength(1)
  await page.screenshot({ path: test.info().outputPath('bulk-order-draft-guidance.png'), fullPage: true })
  const link = page.getByRole('link', { name: '초안·이력 보기', exact: true }); await expect(link).toHaveAttribute('href', `/business/${draftId}`); await link.click(); await expect(page.getByLabel('입고 방식', { exact: true })).toHaveValue('order'); await expect(page.getByLabel('메모 (선택)', { exact: true })).toHaveValue('가상 주문 초안 원문')
  const original = await get(page, `/api/business/${draftId}`); expect(original.item.status).toBe('draft'); expect(original.head.payload.data.source).toEqual(source); expect((await get(page, '/api/receipts')).items).toHaveLength(1)
})
