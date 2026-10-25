import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { spawn } from 'node:child_process'
import { once } from 'node:events'
import { cpSync, mkdirSync, readFileSync, writeFileSync, statSync, readdirSync } from 'node:fs'
import { resolve, join } from 'node:path'
import { randomUUID, createHash } from 'node:crypto'

export const root = resolve(import.meta.dirname, '../..')
const require = createRequire(join(root, 'frontend/package.json'))
export const { chromium, expect } = require('@playwright/test')
export const sha = path => createHash('sha256').update(readFileSync(path)).digest('hex')
export const save = (path, value) => writeFileSync(path, JSON.stringify(value, null, 2) + '\n')
export function fileSizeTree(path) {
  let bytes = 0, files = 0
  for (const entry of readdirSync(path, { withFileTypes: true })) {
    const next = join(path, entry.name)
    if (entry.isDirectory()) { const result = fileSizeTree(next); bytes += result.bytes; files += result.files }
    else if (entry.isFile()) { bytes += statSync(next).size; files++ }
  }
  return { files, bytes }
}
export class App {
  constructor(directory, browser, { copy = true, transcript = true } = {}) {
    this.directory = directory; this.browser = browser; this.transcript = transcript; this.entries = []; this.calls = []
    mkdirSync(directory, { recursive: true })
    this.binary = join(directory, 'sunchang-hansang')
    if (copy) cpSync(join(root, 'target/release/sunchang-hansang'), this.binary)
  }
  async launch() {
    const start = performance.now()
    this.server = spawn(this.binary, ['--no-browser'], { cwd: this.directory, stdio: ['ignore', 'pipe', 'pipe'] })
    this.closed = once(this.server, 'close'); this.output = ''; this.errors = ''
    // A spawn error may happen before launch awaits process closure.
    void this.closed.catch(() => {})
    this.launchUrl = await new Promise((resolveUrl, reject) => {
      const timer = setTimeout(() => reject(Error('startup timeout: ' + this.errors.slice(-1000))), 60000)
      this.server.once('error', error => { clearTimeout(timer); reject(error) })
      this.server.stderr.on('data', data => { this.errors += String(data) })
      this.server.stdout.on('data', data => {
        this.output += String(data)
        const match = this.output.match(/http:\/\/127\.0\.0\.1:\d+\/#token=[a-f0-9]+/)
        if (match) { clearTimeout(timer); resolveUrl(match[0]) }
      })
      this.server.once('exit', code => { clearTimeout(timer); reject(Error(`startup exit ${code}: ${this.errors.slice(-1000)}`)) })
    })
    this.startupMs = performance.now() - start
    this.context = await this.browser.newContext()
    this.page = await this.context.newPage()
    await this.page.goto(this.launchUrl)
    await expect(this.page.getByLabel('재료 이름', { exact: false })).toBeEnabled()
    await this.get('/api/state')
    return this
  }
  async request(method, path, data, expected = method === 'GET' ? 200 : 201) {
    const started = performance.now()
    const response = await this.page.request.fetch(new URL(path, this.launchUrl).toString(), {
      method, headers: { Origin: new URL(this.launchUrl).origin }, ...(data === undefined ? {} : { data }), timeout: 120000,
    })
    const text = await response.text()
    let value; try { value = JSON.parse(text) } catch { value = text }
    const call = { method, path, status: response.status(), ms: performance.now() - started }
    this.calls.push(call)
    if (this.transcript) this.entries.push({ ...call, input: data, output: value })
    assert.equal(response.status(), expected, `${method} ${path}: ${JSON.stringify(value).slice(0, 1500)}`)
    return value
  }
  get(path) { return this.request('GET', path) }
  post(path, data, expected = 201) { return this.request('POST', path, data, expected) }
  async meta() { const state = await this.get('/api/state'); return { request_id: randomUUID(), epoch: state.epoch, expected_revision: state.revision } }
  async material(name, unit = 'g', kind = 'raw') {
    return this.post('/api/materials', { ...await this.meta(), name, unit, kind, description: '' })
  }
  async price(item, amount, quantity = '1', unit = item.unit) {
    return this.post(`/api/materials/${item.id}/prices`, { ...await this.meta(), expected_version_id: item.version_id, amount, quantity, unit })
  }
  async recipe(name, lines, basis = '1', price = '1000', unit = '개') {
    return this.post('/api/recipes', { meta: await this.meta(), owner: { mode: 'new', name, unit, kind: 'menu', description: '' }, data: {
      basis_count: basis, reference_output: null, selling_price: price, cooking_description: '', process: '', reference_time: '', lines,
    } })
  }
  async draft(kind, data) {
    const paths = { stocktake: '/api/stocktake-drafts', production: '/api/production-drafts', correction: '/api/corrections/drafts' }
    return this.post(paths[kind] ?? '/api/business/drafts', { meta: await this.meta(), payload: { kind, version: 1, data }, corrects_confirmation_id: null })
  }
  async confirm(draft, { retry = false } = {}) {
    const id = draft.result.item.id, head = draft.result.head.id
    const preview = await this.post(`/api/business/${id}/preview`, { expected_head_id: head }, 200)
    assert.equal(preview.can_confirm, true, JSON.stringify(preview))
    const input = { meta: await this.meta(), expected_head_id: head }
    const result = await this.post(`/api/business/${id}/confirm`, input)
    if (retry) assert.deepEqual(await this.post(`/api/business/${id}/confirm`, input), result)
    return result
  }
  async business(kind, data, options) { return this.confirm(await this.draft(kind, data), options) }
  receipt(item, quantity, amount, date, options) {
    return this.business('receipt', { source: { kind: 'direct', material_id: item.id, material_version_id: item.version_id }, quantity, unit: item.unit, amount, receipt_date: date, note: '', stock_effect: { kind: 'unreflected' } }, options)
  }
  sale(recipe, count, options) {
    return this.business('sale', { material_id: recipe.item.id, material_version_id: recipe.item.version_id, recipe_version_id: recipe.version.id, count, note: '' }, options)
  }
  stocktake(rows, options) { return this.business('stocktake', { rows, note: '' }, options) }
  row(item, quantity) { return { material_id: item.id, material_version_id: item.version_id, quantity, unit: item.unit, initial_value: null, excess: null } }
  async screenshot(path, route) {
    if (route) await this.page.goto(new URL(route, this.launchUrl).toString())
    await this.page.locator('main').waitFor()
    await this.page.waitForLoadState('networkidle')
    if (route?.startsWith('/stocktakes/')) await expect(this.page.locator('.stocktake-result').first()).toBeVisible()
    if (route?.startsWith('/statistics')) await expect(this.page.locator('#statistics-sales-title')).toBeVisible()
    await this.page.screenshot({ path, fullPage: true })
  }
  async stop({ normal = false } = {}) {
    if (!this.server || this.server.exitCode !== null) return
    await this.page.goto('about:blank')
    if (!normal) this.server.kill('SIGINT')
    let timer
    const [code, signal] = await Promise.race([this.closed, new Promise((_, reject) => { timer = setTimeout(() => reject(Error('shutdown timeout')), 180000) })]).finally(() => clearTimeout(timer))
    assert.equal(code, 0); assert.equal(signal, null)
    assert(this.output.includes('순창한상을 안전하게 종료했습니다.'))
    await this.context.close()
    return { mode: normal ? 'heartbeat-120s-grace' : 'macOS-SIGINT-graceful', code, signal }
  }
  async cleanup() {
    // Failure cleanup is separate from the successful shutdown assertion above.
    // It must also work when launch failed before a page/context was available.
    const running = () => this.server?.pid && this.server.exitCode === null && this.server.signalCode === null
    if (!running()) return { mode: 'already-closed' }
    const wait = async milliseconds => {
      let timer
      await Promise.race([this.closed, new Promise((_, reject) => { timer = setTimeout(() => reject(Error('cleanup timeout')), milliseconds) })]).finally(() => clearTimeout(timer))
    }
    let mode = 'failure-SIGINT'
    this.server.kill('SIGINT')
    try { await wait(5000) }
    catch {
      if (running()) { mode = 'failure-SIGKILL'; this.server.kill('SIGKILL'); await wait(5000) }
    }
    return { mode, code: this.server.exitCode, signal: this.server.signalCode }
  }
  saveEvidence(path) { save(path, { binary_sha256: sha(this.binary), startup_ms: this.startupMs, calls: this.calls, transcript: this.entries, stderr: this.errors }) }
}
