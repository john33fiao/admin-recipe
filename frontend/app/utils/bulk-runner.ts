import type { BulkReview, BulkItem, BulkRow } from '../types/bulk.ts'
import type { BusinessMutation, BusinessPreview, BusinessDetail } from '../types/business.ts'
import type { CatalogState, Material } from '../types/material.ts'
import type { RecipeMutation } from '../types/recipe.ts'
import { businessPending, confirmPending } from './business.ts'
import { failureAction } from './catalog.ts'
import { bulkUuid } from './bulk.ts'
export function applyBulkDraft(entry: BulkItem, row: BulkRow, detail: BusinessDetail, beforeChange: () => void): boolean {
  if (entry.result || entry.pending || entry.command.kind !== 'receipt' || !entry.rowIds.includes(row.id) || entry.draft?.id !== detail.item.id || detail.item.status !== 'draft' || detail.head.payload.kind !== 'receipt' || detail.head.payload.data.source.kind !== 'direct') return false
  beforeChange()
  const payload = JSON.parse(JSON.stringify(detail.head.payload)) as typeof detail.head.payload, data = payload.data
  if (data.source.kind !== 'direct') return false
  Object.assign(row.values, { material_id: data.source.material_id ?? '', quantity: data.quantity, unit: data.unit, amount: data.amount, receipt_date: data.receipt_date, note: data.note, stock_effect: data.stock_effect.kind, observation_version_id: '', included_quantity: '' })
  row.reference = data.source.material_id && data.source.material_version_id ? { material_id: data.source.material_id, material_version_id: data.source.material_version_id } : null
  row.selections = data.stock_effect.kind === 'reconciled' ? data.stock_effect.selections : []
  entry.command = { kind: 'receipt', payload }; entry.draft = { id: detail.item.id, head_id: detail.head.id }; entry.recovery = ''; entry.failure = ''
  return true
}
// Called only by an explicit re-review. Never replaces an unresolved request.
export function inspectBulkDraft(entry: BulkItem, detail: BusinessDetail): 'pending' | 'unchanged' | 'changed' | 'confirmed' | 'archived' {
  if (entry.pending) return 'pending'
  if (!entry.draft || entry.draft.id !== detail.item.id || entry.command.kind !== 'receipt') throw new Error('입고 초안 연결이 다릅니다.')
  if (detail.item.status === 'confirmed') {
    const confirmation = detail.confirmations.find(value => value.record_id === detail.item.id)
    const result = confirmation?.result
    if (!result || !('receipt_id' in result) || !bulkUuid(result.receipt_id)) throw new Error('이미 확정한 입고 결과를 확인할 수 없습니다.')
    entry.result = { id: result.receipt_id, href: `/receipts/${result.receipt_id}` }; entry.recovery = ''; entry.failure = '다른 화면에서 이미 확정한 입고 결과를 연결했습니다. 새 입고를 만들지 않았습니다. 표의 원입력과 실제 기록을 비교해 주세요.'; return 'confirmed'
  }
  if (detail.item.status === 'archived') { entry.recovery = 'review'; entry.failure = '이 입고 초안이 다른 화면에서 보관되었습니다. 원기록을 확인해 주세요. 자동으로 새 입고를 만들지 않습니다.'; return 'archived' }
  if (!sameBulkEffect(entry.command.payload, detail.head.payload)) { entry.recovery = 'review'; entry.failure = '이 입고 초안이 다른 화면에서 수정되었습니다. 기존 표를 보존했습니다. 현재 초안을 확인하고 적용할 내용을 선택해 주세요.'; return 'changed' }
  entry.draft.head_id = detail.head.id; entry.preview = null; entry.recovery = ''; entry.failure = ''; return 'unchanged'
}

export function sameBulkEffect(left: unknown, right: unknown): boolean {
  if (left === right) return true
  if (Array.isArray(left) || Array.isArray(right)) return Array.isArray(left) && Array.isArray(right) && left.length === right.length && left.every((value, index) => sameBulkEffect(value, right[index]))
  if (!left || !right || typeof left !== 'object' || typeof right !== 'object') return false
  const a = left as Record<string, unknown>, b = right as Record<string, unknown>
  return Object.keys(a).length === Object.keys(b).length && Object.keys(a).every(key => Object.hasOwn(b, key) && sameBulkEffect(a[key], b[key]))
}
export async function runBulk(review: BulkReview, request: <T>(path: string, body?: unknown) => Promise<T>, persist: () => void, makeId: () => string = () => crypto.randomUUID()): Promise<boolean> {
  if (Object.keys(review.errors).length || !review.items.length) return false
  for (const entry of review.items) {
    if (entry.result) continue
    if (['blocked', 'review', 'effect'].includes(entry.recovery)) return false
    try {
      entry.failure = ''
      const meta = () => ({ request_id: makeId(), epoch: review.state.epoch, expected_revision: review.state.revision })
      const command = entry.command
      if (command.kind === 'material') {
        entry.pending ??= { kind: 'material', body: { ...command.input, ...meta() } }
        if (entry.pending.kind !== 'material') throw new Error('요청 종류를 확인할 수 없습니다.')
        persist()
        const response = await request<Material & CatalogState>('/api/materials', entry.pending.body)
        if (!bulkUuid(response.id)) throw new Error('등록 결과를 확인할 수 없습니다.')
        entry.result = { id: response.id, href: `/materials/${response.id}` }; review.state = { epoch: response.epoch, revision: response.revision }
      } else if (command.kind === 'menu') {
        entry.pending ??= { kind: 'menu', request: { action: 'create', material_id: null, body: { meta: meta(), owner: command.owner, data: command.data } } }
        if (entry.pending.kind !== 'menu') throw new Error('요청 종류를 확인할 수 없습니다.')
        persist()
        const response = await request<RecipeMutation>('/api/recipes', entry.pending.request.body)
        if (!bulkUuid(response.item.id)) throw new Error('등록 결과를 확인할 수 없습니다.')
        entry.result = { id: response.item.id, href: `/recipes/${response.item.id}` }; review.state = response.state
      } else {
        if (!entry.draft) {
          entry.pending ??= { kind: 'receipt', request: businessPending('create', null, null, review.state, command.payload, null, makeId) }
          if (entry.pending.kind !== 'receipt' || entry.pending.request.action !== 'create') throw new Error('입고 원요청을 확인할 수 없습니다.')
          persist()
          const response = await request<BusinessMutation>('/api/business/drafts', entry.pending.request.body)
          if (!bulkUuid(response.result.item.id) || !bulkUuid(response.result.head.id)) throw new Error('입고 초안 결과를 확인할 수 없습니다.')
          entry.draft = { id: response.result.item.id, head_id: response.result.head.id }; review.state = response.state
          entry.pending = null; persist()
        }
        if (!entry.pending) {
          const preview = await request<BusinessPreview>(`/api/business/${entry.draft.id}/preview`, { expected_head_id: entry.draft.head_id })
          if (preview.state.epoch !== review.state.epoch) throw Object.assign(new Error('자료가 복원되었습니다. 원래 입력을 보존했습니다. 현재 자료에서 새로 검토해 주세요.'), { status: 409, code: 'stale_generation', requestStatus: 'not_processed' })
          entry.preview = preview
          if (!preview.can_confirm || !preview.effect || preview.issues.length) {
            entry.recovery = 'review'; entry.failure = preview.issues.map(issue => issue.message).join(' ') || '입고 내용을 다시 확인해 주세요.'; persist(); return false
          }
          if (!sameBulkEffect(entry.effect, preview.effect)) {
            entry.recovery = 'effect'; entry.failure = '입고 효과가 확인한 내용과 달라졌습니다. 아래 새 미리보기를 확인한 뒤 이어서 등록해 주세요.'; persist(); return false
          }
          entry.pending = { kind: 'receipt', request: confirmPending(preview, makeId) }
        }
        if (entry.pending.kind !== 'receipt' || entry.pending.request.action !== 'confirm') throw new Error('확정 원요청을 확인할 수 없습니다.')
        persist()
        const response = await request<BusinessMutation>(`/api/business/${entry.draft.id}/confirm`, entry.pending.request.body)
        const result = response.result.confirmation?.result
        if (!result || !('receipt_id' in result) || !bulkUuid(result.receipt_id)) throw new Error('입고 확정 결과를 확인할 수 없습니다.')
        entry.result = { id: result.receipt_id, href: `/receipts/${result.receipt_id}` }; review.state = response.state
      }
      entry.pending = null; entry.recovery = ''; entry.failure = ''; persist()
    } catch (error) {
      const failure = error as { status?: number; code?: string; requestStatus?: string; message?: string }
      const action = failureAction(failure.status ?? 0, failure.code, failure.requestStatus)
      entry.recovery = action === 'blocked' ? 'blocked' : action === 'edit' || action === 'refresh' ? 'review' : 'retry'
      if (!entry.pending && entry.draft && failure.status === 409 && failure.code !== 'stale_generation') entry.recovery = 'review'
      entry.failure = failure.code === 'stale_generation' ? failure.requestStatus === 'already_processed' ? '복원 전에 처리된 요청입니다. 현재 자료에 다시 적용하지 않습니다. 보존한 원입력과 실제 목록을 확인해 주세요.' : '자료가 복원되어 이 요청은 처리하지 않았습니다. 원입력을 보존하고 현재 자료에서 다시 검토해 주세요.' : action === 'blocked' ? '요청 ID에 다른 처리 이력이 있습니다. 원요청을 보존했습니다. 실제 결과를 확인해 주세요.' : action === 'edit' || action === 'refresh' ? failure.message || '최신 자료와 입력을 다시 확인해 주세요.' : '완료를 확인하지 못했습니다. 입력과 같은 요청을 보존했습니다. 연결·탭 저장공간을 확인한 뒤 같은 요청으로 이어서 시도해 주세요.'
      try { persist() } catch { /* The in-memory immutable request must remain available. */ }
      return false
    }
  }
  return true
}
