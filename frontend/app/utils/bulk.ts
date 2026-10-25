import type { BulkContext, BulkKind, BulkRow, BulkReview, BulkSession, BulkItem, BulkCommand } from '../types/bulk.ts'
import type { CatalogInput, Material, MaterialKind } from '../types/material.ts'
import { bulkColumns, bulkLimits } from './bulk-csv.ts'
import { decimalInput, normalizeCatalog, restoreCatalogRegistration, validateCatalogInput } from './catalog.ts'
import { blankRecipe, ownerShape, ownerDraftShape, recipeShape, restoreRecipePending, validateRecipe } from './recipe.ts'
import { validateReceiptDraft } from './receipt.ts'
import { restoreBusinessPending, validBusinessPayload } from './business.ts'

export const bulkUuid = (value: unknown): value is string => typeof value === 'string' && /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(value)
const referenceUuid = (value: unknown): value is string => typeof value === 'string' && /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(value)
const object = (v: unknown): v is Record<string, unknown> => !!v && typeof v === 'object' && !Array.isArray(v)
const exact = (v: Record<string, unknown>, keys: string[]) => Object.keys(v).length === keys.length && keys.every(key => Object.hasOwn(v, key))
const same = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b)
const stateShape = (v: unknown) => object(v) && exact(v, ['epoch', 'revision']) && bulkUuid(v.epoch) && typeof v.revision === 'string' && /^(0|[1-9]\d*)$/.test(v.revision)
export function setBulkCell(kind: BulkKind, row: BulkRow, field: string, value: string) {
  if (!bulkColumns[kind].some(column => column.key === field)) return
  if (field === 'material_id') {
    row.reference = null; row.selections = []
    if (kind === 'receipt') { row.values.observation_version_id = ''; row.values.included_quantity = '' }
  }
  row.values[field] = value
}
export function makeBulkRow(kind: BulkKind, values: Record<string, string> = {}, makeId: () => string = () => crypto.randomUUID()): BulkRow {
  const defaults = kind === 'material' ? { unit: 'g', kind: 'raw' } : kind === 'menu' ? { unit: '개', basis_count: '1' } : {}
  return { id: makeId(), values: Object.fromEntries(bulkColumns[kind].map(column => [column.key, values[column.key] ?? (defaults as Record<string, string>)[column.key] ?? ''])), reference: null, selections: [] }
}
export function bulkFingerprint(kind: BulkKind, rows: BulkRow[]) {
  return JSON.stringify([kind, rows.map(row => [bulkColumns[kind].map(column => row.values[column.key] ?? ''), row.selections])])
}
function canonical(unit: string) { return ({ kg: 'g', L: 'mL' } as Record<string, string>)[unit.trim()] ?? unit.trim() }
function compatible(material: Material, unit: string) { return canonical(unit) === material.canonical_unit }
function catalog(row: BulkRow, menu = false): CatalogInput {
  return normalizeCatalog({ name: row.values.name ?? '', unit: row.values.unit ?? '', kind: menu ? 'menu' : row.values.kind as MaterialKind, description: row.values.description ?? '' })
}
function item(row: BulkRow, command: BulkCommand, label: string): BulkItem {
  return { id: row.id, rowIds: [row.id], label, command, effect: null, preview: null, draft: null, result: null, pending: null, recovery: '', failure: '' }
}
export function reviewBulk(kind: BulkKind, rows: BulkRow[], context: BulkContext): BulkReview {
  const result: BulkReview = { state: { ...context.state }, items: [], errors: {}, warnings: [] }
  const error = (row: BulkRow, field: string, message: string) => { (result.errors[row.id] ??= {})[field] = message }
  if (!rows.length || rows.length > bulkLimits.rows) { result.warnings.push('1~200행을 준비해 주세요.'); return result }
  const reference = (row: BulkRow, unitField: string): Material | null => {
    const id = (row.values.material_id ?? '').trim()
    const found = context.materials.find(material => material.id === id)
    if (!found || found.archived) { error(row, 'material_id', '사용 중인 재료를 선택해 주세요. 이름이 같아도 ID로 구별합니다.'); return null }
    if (row.reference && (row.reference.material_id !== id || row.reference.material_version_id !== found.version_id)) { error(row, 'material_id', '선택한 재료 버전이 바뀌었습니다. 재료를 다시 선택해 변경을 확인해 주세요.'); return null }
    row.reference ??= { material_id: found.id, material_version_id: found.version_id }
    if (!compatible(found, row.values[unitField] ?? '')) error(row, unitField, `선택한 재료의 ${found.unit}와 호환되는 단위가 필요합니다.`)
    const quantity = decimalInput(row.values.quantity ?? '', 'quantity')
    if (quantity !== null && ['kg', 'L'].includes((row.values[unitField] ?? '').trim())) {
      const [whole, fraction = ''] = quantity.split('.')
      if ((BigInt(whole!) * 1_000_000n + BigInt(fraction.padEnd(6, '0'))) * 1000n > 9_000_000_000_000_000n) error(row, 'quantity', '단위 환산 결과가 허용 범위를 넘습니다.')
    }
    return found
  }
  const groups = new Map<string, BulkItem>()
  for (const row of rows) {
    if (kind === 'material') {
      const input = catalog(row)
      for (const [field, message] of Object.entries(validateCatalogInput(input))) error(row, field, message!)
      if (input.kind === 'unclassified') error(row, 'kind', '새 재료는 원물·손질재료·비축품·판매메뉴 중에서 종류를 선택해 주세요.')
      result.items.push(item(row, { kind, input }, input.name))
    } else if (kind === 'menu') {
      const owner = { mode: 'new' as const, ...catalog(row, true) }
      for (const [field, message] of Object.entries(validateCatalogInput(owner))) error(row, field, message!)
      const group = row.values.group!.trim()
      if (!group || [...group].length > 100 || /[\u0000-\u001f\u007f-\u009f]/u.test(group)) error(row, 'group', '메뉴 묶음을 1~100자로 입력해 주세요.')
      const data = { ...blankRecipe(), basis_count: row.values.basis_count!.trim(), selling_price: row.values.selling_price!.trim() === '' ? null : row.values.selling_price!.trim() }
      for (const [field, message] of Object.entries(validateRecipe(data))) error(row, field, message)
      let entry = groups.get(group)
      if (!entry) { entry = item(row, { kind, owner, data }, `${owner.name} · 묶음 ${group}`); groups.set(group, entry); result.items.push(entry) }
      else if (entry.command.kind === 'menu') {
        entry.rowIds.push(row.id)
        for (const field of ['name', 'unit', 'description', 'basis_count', 'selling_price']) {
          const original = rows.find(candidate => candidate.id === entry!.id)!
          if (row.values[field]!.trim() !== original.values[field]!.trim()) error(row, field, '같은 메뉴 묶음의 기본 정보는 모든 행에서 같아야 합니다.')
        }
      }
      if (row.values.material_id!.trim()) {
        const found = reference(row, 'component_unit')
        const line = { material_id: row.values.material_id!.trim(), material_version_id: row.reference?.material_version_id ?? '', quantity: row.values.quantity!.trim(), unit: row.values.component_unit!.trim(), note: row.values.note! }
        const errors = validateRecipe({ ...blankRecipe(), lines: [line] })
        for (const [field, message] of Object.entries(errors)) error(row, field === 'line-0-material' ? 'material_id' : field === 'line-0-unit' ? 'component_unit' : field.replace('line-0-', ''), message)
        if (found && entry.command.kind === 'menu') entry.command.data.lines.push(line)
      } else if (row.values.quantity!.trim() || row.values.component_unit!.trim() || row.values.note!.trim()) error(row, 'material_id', '구성 수량·단위·메모가 있으면 구성 재료를 선택해 주세요.')
      if (entry.command.kind === 'menu' && entry.command.data.lines.length > 100) error(row, 'material_id', '한 메뉴에 구성 재료를 100행까지 입력할 수 있습니다.')
    } else {
      const found = reference(row, 'unit')
      const quantity = decimalInput(row.values.quantity!, 'quantity')
      if (quantity === null || quantity === '0') error(row, 'quantity', '실제 입고 수량은 0보다 큰 숫자로 입력해 주세요.')
      if (decimalInput(row.values.amount!, 'money') === null) error(row, 'amount', '구입금액이 0원이면 0을 입력해 주세요. 빈 값은 확정할 수 없습니다.')
      const date = row.values.receipt_date!.trim()
      if (!/^\d{4}-\d{2}-\d{2}$/.test(date) || date.startsWith('0000') || !Number.isFinite(Date.parse(date + 'T00:00:00Z')) || new Date(date + 'T00:00:00Z').toISOString().slice(0, 10) !== date) error(row, 'receipt_date', '실제 입고일을 YYYY-MM-DD로 입력해 주세요.')
      let selections = row.selections.map(selection => ({ ...selection, source: selection.source ? JSON.parse(JSON.stringify(selection.source)) : null }))
      if (!selections.length && row.values.observation_version_id!.trim()) selections = [{ observation_version_id: row.values.observation_version_id!.trim(), included_quantity: row.values.included_quantity!.trim(), source: { kind: 'all_available' } }]
      const effect = row.values.stock_effect!.trim()
      if (!['unreflected', 'reconciled'].includes(effect)) error(row, 'stock_effect', '미반영 또는 실사 포함을 확인해 선택해 주세요.')
      if (effect === 'reconciled' && !selections.length) error(row, 'stock_effect', '실사에 포함된 수량과 근거를 선택해 주세요. 포함량 0이면 미반영을 선택하세요.')
      if (effect === 'unreflected' && (selections.length || row.values.included_quantity!.trim())) error(row, 'stock_effect', '실사 포함 근거가 남아 있습니다. 포함 여부를 확인하거나 근거를 지워 주세요.')
      const payload = { kind: 'receipt' as const, version: 1 as const, data: { source: { kind: 'direct' as const, material_id: found?.id ?? row.values.material_id!.trim(), material_version_id: row.reference?.material_version_id ?? null }, quantity: row.values.quantity!.trim(), unit: row.values.unit!.trim(), amount: row.values.amount!.trim(), receipt_date: date, note: row.values.note!, stock_effect: effect === 'reconciled' ? { kind: 'reconciled' as const, selections } : { kind: 'unreflected' as const } } }
      for (const [field, message] of Object.entries(validateReceiptDraft(payload))) error(row, field.startsWith('stock_effect') ? 'stock_effect' : field, message)
      result.items.push(item(row, { kind, payload }, found?.name ?? row.values.material_id!))
    }
  }
  const names = new Set<string>(), references = new Set<string>(), commands = new Set<string>()
  for (const entry of result.items) {
    const command = entry.command, key = JSON.stringify(command)
    if (commands.has(key)) result.warnings.push(`${entry.label}: 같은 내용이 여러 번 있습니다. 각각 별도 등록됩니다.`)
    commands.add(key)
    if (command.kind === 'receipt') {
      const id = command.payload.data.source.kind === 'direct' ? command.payload.data.source.material_id! : ''
      if (references.has(id)) result.warnings.push(`${entry.label}: 앞선 입고로 효과가 바뀌면 멈추고 새 미리보기를 확인받습니다.`)
      references.add(id)
    } else {
      const name = command.kind === 'material' ? command.input.name : command.owner.mode === 'new' ? command.owner.name : ''
      if (names.has(name) || context.materials.some(material => material.name === name)) result.warnings.push(`${name}: 같은 이름이 있습니다. 기존 항목에 합치지 않고 새 ID로 등록합니다.`)
      names.add(name)
    }
  }
  return result
}
const draftText = (value: unknown): value is string => typeof value === 'string' && value.length <= bulkLimits.bytes
function selectionShape(value: unknown): boolean {
  if (!object(value) || !exact(value, ['observation_version_id', 'source', 'included_quantity']) || !(value.observation_version_id === null || draftText(value.observation_version_id)) || !draftText(value.included_quantity)) return false
  if (value.source === null) return true
  const source = value.source
  if (!object(source)) return false
  if (source.kind === 'all_available') return exact(source, ['kind'])
  return source.kind === 'source' && exact(source, ['kind', 'source_key']) && object(source.source_key) && exact(source.source_key, ['kind', 'id', 'position']) && ['receipt_inflow', 'observation_initial', 'observation_excess'].includes(source.source_key.kind as string) && draftText(source.source_key.id) && source.source_key.position === 0
}
function invalidReceiptShape(value: unknown): boolean {
  if (!object(value) || !exact(value, ['kind', 'version', 'data']) || value.kind !== 'receipt' || value.version !== 1 || !object(value.data)) return false
  const data = value.data
  if (!exact(data, ['source', 'quantity', 'unit', 'amount', 'receipt_date', 'note', 'stock_effect']) || !['quantity', 'unit', 'amount', 'receipt_date', 'note'].every(key => draftText(data[key])) || !object(data.source) || !exact(data.source, ['kind', 'material_id', 'material_version_id']) || data.source.kind !== 'direct' || ![data.source.material_id, data.source.material_version_id].every(id => id === null || draftText(id)) || !object(data.stock_effect)) return false
  const effect = data.stock_effect
  if (effect.kind === 'needs_reconciliation') return exact(effect, ['kind', 'basis_id', 'included_quantity']) && (effect.basis_id === null || draftText(effect.basis_id)) && draftText(effect.included_quantity)
  return effect.kind === 'unreflected' ? exact(effect, ['kind']) : effect.kind === 'reconciled' && exact(effect, ['kind', 'selections']) && Array.isArray(effect.selections) && effect.selections.length <= 20 && effect.selections.every(selectionShape)
}
function commandShape(value: unknown, allowInvalid = false): value is BulkCommand {
  if (!object(value)) return false
  if (value.kind === 'material') {
    const input = value.input
    return exact(value, ['kind', 'input']) && object(input) && exact(input, ['name', 'unit', 'kind', 'description']) && ['name', 'unit', 'kind', 'description'].every(key => draftText(input[key])) && (allowInvalid || !Object.keys(validateCatalogInput(input as unknown as CatalogInput)).length)
  }
  if (value.kind === 'menu') return exact(value, ['kind', 'owner', 'data']) && (allowInvalid ? ownerDraftShape(value.owner) : ownerShape(value.owner)) && recipeShape(value.data) && (allowInvalid || !Object.keys(validateRecipe(value.data)).length)
  return value.kind === 'receipt' && exact(value, ['kind', 'payload']) && (allowInvalid ? invalidReceiptShape(value.payload) : validBusinessPayload(value.payload) && value.payload.kind === 'receipt')
}
export function restoreBulkSession(raw: string | null): BulkSession | null {
  try {
    if (!raw || raw.length > 6 * bulkLimits.bytes) return null
    const value: unknown = JSON.parse(raw)
    if (!object(value) || !exact(value, ['version', 'kind', 'rows', 'review', 'fingerprint', 'acceptedWarnings', 'duplicate']) || value.version !== 1 || !['material', 'menu', 'receipt'].includes(value.kind as string) || !Array.isArray(value.rows) || value.rows.length > bulkLimits.rows || typeof value.fingerprint !== 'string' || typeof value.acceptedWarnings !== 'boolean' || typeof value.duplicate !== 'boolean') return null
    const keys = bulkColumns[value.kind as BulkKind].map(column => column.key), rows = value.rows
    for (const row of rows) {
      if (!object(row) || !exact(row, ['id', 'values', 'reference', 'selections']) || !bulkUuid(row.id) || !object(row.values) || !exact(row.values, keys) || !Array.isArray(row.selections) || row.selections.length > 20) return null
      const fields = row.values
      if (!keys.every(key => draftText(fields[key]))) return null
      if (row.reference !== null && (!object(row.reference) || !exact(row.reference, ['material_id', 'material_version_id']) || !referenceUuid(row.reference.material_id) || !referenceUuid(row.reference.material_version_id))) return null
      if (!row.selections.every(selectionShape)) return null
    }
    if (new Set(rows.map(row => row.id)).size !== rows.length) return null
    if (value.review !== null) {
      const review = value.review
      if (!object(review) || !exact(review, ['state', 'items', 'errors', 'warnings']) || !stateShape(review.state) || !Array.isArray(review.items) || review.items.length > bulkLimits.rows || !object(review.errors) || !Array.isArray(review.warnings) || !review.warnings.every(w => typeof w === 'string')) return null
      for (const entry of review.items) {
        if (!object(entry) || !exact(entry, ['id', 'rowIds', 'label', 'command', 'effect', 'preview', 'draft', 'result', 'pending', 'recovery', 'failure']) || !bulkUuid(entry.id) || !Array.isArray(entry.rowIds) || !entry.rowIds.length || !entry.rowIds.every(id => rows.some(row => row.id === id)) || typeof entry.label !== 'string' || !commandShape(entry.command, Object.keys(review.errors).length > 0 && entry.pending === null && entry.result === null) || entry.command.kind !== value.kind || !['', 'retry', 'review', 'blocked', 'effect'].includes(entry.recovery as string) || typeof entry.failure !== 'string') return null
        if (entry.draft !== null && (!object(entry.draft) || !exact(entry.draft, ['id', 'head_id']) || !bulkUuid(entry.draft.id) || !bulkUuid(entry.draft.head_id))) return null
        if (entry.result !== null && (!object(entry.result) || !exact(entry.result, ['id', 'href']) || !bulkUuid(entry.result.id) || entry.result.href !== `/${value.kind === 'material' ? 'materials' : value.kind === 'menu' ? 'recipes' : 'receipts'}/${entry.result.id}`)) return null
        if (entry.preview !== null && (!object(entry.preview) || !bulkUuid(entry.preview.record_id) || !bulkUuid(entry.preview.head_id) || !stateShape(entry.preview.state) || typeof entry.preview.can_confirm !== 'boolean' || !Array.isArray(entry.preview.issues))) return null
        if (entry.pending !== null) {
          const pending = entry.pending
          if (!object(pending) || pending.kind !== entry.command.kind) return null
          if (pending.kind === 'material') {
            if (!exact(pending, ['kind', 'body']) || !object(pending.body) || !exact(pending.body, ['name', 'unit', 'kind', 'description', 'request_id', 'epoch', 'expected_revision'])) return null
            const request = restoreCatalogRegistration(JSON.stringify(pending.body))
            if (!request || !('kind' in request) || entry.command.kind !== 'material' || !same(entry.command.input, { name: request.name, unit: request.unit, kind: request.kind, description: request.description })) return null
          } else if (pending.kind === 'menu') {
            const request = restoreRecipePending(JSON.stringify(pending.request))
            if (!exact(pending, ['kind', 'request']) || request?.action !== 'create' || entry.command.kind !== 'menu' || !same(request.body.owner, entry.command.owner) || !same(request.body.data, entry.command.data)) return null
          } else {
            const request = restoreBusinessPending(JSON.stringify(pending.request))
            if (!exact(pending, ['kind', 'request']) || !request || !['create', 'confirm'].includes(request.action) || entry.command.kind !== 'receipt' || request.action === 'create' && !same(request.body.payload, entry.command.payload) || request.action === 'confirm' && (request.record_id !== (entry.draft as { id: string } | null)?.id || request.body.expected_head_id !== (entry.draft as { head_id: string } | null)?.head_id)) return null
          }
        }
      }
    }
    return value as unknown as BulkSession
  } catch { return null }
}
