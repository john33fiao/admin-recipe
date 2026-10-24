import type { BusinessDetail, BusinessIssue, BusinessMutation, BusinessPayload, BusinessPending, BusinessPreview, BusinessStatus, BusinessSummary } from '~/types/business'
import type { CatalogState } from '~/types/material'
import { ApiError, apiRequest } from '~/utils/api'
import { businessPending, confirmPending, copyPayload, restoreBusinessPending, sameBusinessTarget, samePayload, validBusinessPayload } from '~/utils/business'
import { failureAction } from '~/utils/catalog'

export interface BusinessEditorInitial<T extends BusinessPayload = BusinessPayload> { payload: T; state: CatalogState; recordId: string | null; headId: string | null; status: BusinessStatus; corrects: string | null; issues: BusinessIssue[] }

export function useBusinessDraft<T extends BusinessPayload>(identity: string, initial: BusinessEditorInitial<T>, refresh: () => Promise<BusinessDetail | null>) {
  const key = `sunchang.business.${identity}`
  const pendingKey = `${key}.pending.v1`
  const draftKey = `${key}.draft.v1`
  const connection = useConnection()
  const pending = useState<BusinessPending | null>(`${key}.pending`, () => {
    try {
      const value = restoreBusinessPending(sessionStorage.getItem(pendingKey))
      if (value && value.record_id === initial.recordId && (value.action !== 'create' || value.body.corrects_confirmation_id === initial.corrects) && (!('payload' in value.body) || sameBusinessTarget(initial.payload, value.body.payload))) return value
    } catch { /* Invalid pending data cannot change the kind or target. */ }
    return null
  })
  const stored = useState<{ payload: T; state: CatalogState; headId: string | null } | null>(`${key}.stored`, () => {
    try {
      const value = JSON.parse(sessionStorage.getItem(draftKey) ?? 'null')
      if (validBusinessPayload(value?.payload) && sameBusinessTarget(initial.payload, value.payload) && typeof value.state?.epoch === 'string' && typeof value.state?.revision === 'string' && (value.headId === null || typeof value.headId === 'string')) return value
    } catch { /* Malformed tab content is not a write request. */ }
    return null
  })
  const payload = useState<T>(`${key}.payload`, () => pending.value && 'payload' in pending.value.body ? copyPayload(pending.value.body.payload) as T : copyPayload(stored.value?.payload ?? initial.payload))
  const baseline = useState<T>(`${key}.baseline`, () => copyPayload(initial.payload))
  const basis = useState<CatalogState>(`${key}.basis`, () => pending.value ? { epoch: pending.value.body.meta.epoch, revision: pending.value.body.meta.expected_revision } : stored.value?.state ?? initial.state)
  const headId = useState<string | null>(`${key}.head`, () => pending.value && 'expected_head_id' in pending.value.body ? pending.value.body.expected_head_id : stored.value?.headId ?? initial.headId)
  const status = useState<BusinessStatus>(`${key}.status`, () => initial.status)
  const issues = useState<BusinessIssue[]>(`${key}.issues`, () => initial.issues)
  const preview = useState<BusinessPreview | null>(`${key}.preview`, () => null)
  const busy = useState(`${key}.busy`, () => false)
  const failure = useState(`${key}.failure`, () => '')
  const recovery = useState<'edit' | 'refresh' | 'blocked' | 'retry' | ''>(`${key}.recovery`, () => '')
  const notice = useState(`${key}.notice`, () => '')
  const storageFailure = useState(`${key}.storage-failure`, () => '')
  const created = useState<BusinessSummary | null>(`${key}.created`, () => null)
  const completed = useState(`${key}.completed`, () => 0)
  const dirty = computed(() => !samePayload(payload.value, baseline.value))
  const locked = computed(() => busy.value || !!pending.value || status.value !== 'draft' || !!created.value)
  const canRetry = computed(() => !!pending.value && (!recovery.value || recovery.value === 'retry'))
  const canReview = computed(() => recovery.value === 'edit' || recovery.value === 'refresh')
  let applying = false
  let mounted = true
  onScopeDispose(() => { mounted = false })

  function synchronizeTerminal(latest: BusinessEditorInitial<T>) {
    if (latest.status === 'draft' || pending.value || busy.value) return
    status.value = latest.status
    baseline.value = copyPayload(latest.payload)
    basis.value = latest.state
    headId.value = latest.headId
    issues.value = latest.issues
    preview.value = null
  }

  function persistDraft() {
    if (applying || pending.value) return
    try {
      if (dirty.value) sessionStorage.setItem(draftKey, JSON.stringify({ payload: payload.value, state: basis.value, headId: headId.value }))
      else sessionStorage.removeItem(draftKey)
      storageFailure.value = ''
    } catch { storageFailure.value = '이 탭에 미저장 입력을 보관하지 못했습니다. 화면을 닫기 전에 임시 저장해 주세요.' }
  }
  watch(payload, () => { if (!applying) { preview.value = null; notice.value = '' }; persistDraft() }, { deep: true, flush: 'sync' })
  watch(completed, () => { void refresh() })

  function failed(error: unknown) {
    recovery.value = error instanceof ApiError ? error.status === 413 && (payload.value.kind === 'production' || payload.value.kind === 'stocktake' || payload.value.kind === 'receipt' && payload.value.data.stock_effect.kind === 'reconciled') ? 'edit' : failureAction(error.status, error.code) : 'retry'
    if (recovery.value === 'refresh') failure.value = '다른 변경으로 자료가 갱신되어 이 요청은 처리하지 않았습니다. 최신 기록을 확인하고 입력과 미리보기를 다시 검토해 주세요.'
    else if (recovery.value === 'blocked') failure.value = '이 요청 ID에 다른 처리 이력이 있어 재전송할 수 없습니다. 업무 기록에서 처리 결과를 확인해 주세요.'
    else if (recovery.value === 'edit') failure.value = error instanceof Error ? error.message : '입력을 확인해 주세요.'
    else failure.value = pending.value ? '처리 완료를 확인하지 못했습니다. 요청과 입력을 유지했습니다. 같은 요청으로 재시도해 주세요.' : '자료를 확인하지 못했습니다. 연결을 확인하고 다시 시도해 주세요.'
  }

  async function send(action: BusinessPending['action']): Promise<BusinessMutation | null> {
    if (busy.value || connection.value !== 'connected' || (recovery.value && recovery.value !== 'retry')) return null
    if (pending.value && pending.value.action !== action) return null
    if (!pending.value && (status.value !== 'draft' || created.value)) return null
    let request = pending.value
    if (!request) {
      if (action === 'confirm') {
        if (dirty.value || !preview.value?.can_confirm || !preview.value.effect || preview.value.issues.length || preview.value.head_id !== headId.value) return null
        request = confirmPending(preview.value, () => crypto.randomUUID())
      } else if (action === 'create') request = businessPending(action, null, null, basis.value, payload.value, initial.corrects, () => crypto.randomUUID())
      else if (initial.recordId && headId.value) request = businessPending(action, initial.recordId, headId.value, basis.value, payload.value, initial.corrects, () => crypto.randomUUID())
      else return null
    }
    try { sessionStorage.setItem(pendingKey, JSON.stringify(request)) } catch { storageFailure.value = '요청을 이 탭에 보관하지 못해 전송하지 않았습니다. 브라우저 저장공간을 확인해 주세요.'; return null }
    pending.value = request
    busy.value = true
    failure.value = ''
    notice.value = ''
    try {
      const largeWrite = 'payload' in request.body && (request.body.payload.kind === 'production' || request.body.payload.kind === 'stocktake') ? request.body.payload.kind : null
      const path = largeWrite && request.action === 'create' ? `/api/${largeWrite}-drafts` : largeWrite && request.action === 'revise' ? `/api/${largeWrite}-drafts/${encodeURIComponent(request.record_id!)}/revisions` : request.action === 'create' ? '/api/business/drafts' : `/api/business/${encodeURIComponent(request.record_id!)}/${{ revise: 'draft-revisions', confirm: 'confirm', archive: 'archive' }[request.action]}`
      const response = await apiRequest<BusinessMutation>(path, request.body)
      applying = true
      basis.value = response.state
      issues.value = response.result.issues
      if (request.action === 'create') {
        created.value = response.result.item
        payload.value = copyPayload(initial.payload)
        baseline.value = copyPayload(initial.payload)
      } else {
        // The server keeps a stored record's kind/action/target immutable.
        payload.value = copyPayload(response.result.head.payload) as T
        baseline.value = copyPayload(response.result.head.payload) as T
        headId.value = response.result.head.id
        status.value = response.result.head.status
      }
      try { sessionStorage.removeItem(draftKey); sessionStorage.removeItem(pendingKey) } catch { /* A stored immutable request remains safe to replay. */ }
      pending.value = null
      preview.value = null
      recovery.value = ''
      const confirmedPayload = response.result.head.payload
      const confirmed = confirmedPayload.kind === 'stocktake' ? '실사를 확정했습니다. 확인한 수량과 소모·배분 근거를 보관했습니다.' : confirmedPayload.kind === 'waste' ? '폐기·직원식을 확정했습니다. 실제 수량과 원가를 정산 기준에 반영했습니다.' : confirmedPayload.kind === 'catalog_price' ? '가격을 확정했습니다. 확정 기록과 가격을 함께 보관했습니다.' : confirmedPayload.kind === 'receipt' ? confirmedPayload.data.stock_effect.kind === 'reconciled' ? '입고와 보정을 확정했습니다. 실사 포함 수량을 다시 더하지 않고 차이를 보관했습니다.' : '입고를 확정했습니다. 실제 수량과 금액을 정산 기준에 반영했습니다.' : confirmedPayload.kind === 'production' ? '생산을 확정했습니다. 실제 투입과 결과, 원가 배분을 정산 기준에 반영했습니다.' : confirmedPayload.kind === 'sale' ? '판매를 확정했습니다. 당시 레시피와 원가를 보관했습니다. 재고는 차감하지 않았습니다.' : { place: '주문을 확정했습니다. 미입고 내역에서 확인할 수 있습니다.', revise: '주문 수정을 확정했습니다. 이전 주문 내용은 이력에 남아 있습니다.', cancel: '주문 취소를 확정했습니다. 보유 수량과 가격은 바뀌지 않았습니다.' }[confirmedPayload.data.action]
      notice.value = { create: '초안을 임시 저장했습니다. 앱을 다시 열어도 이어서 작성할 수 있습니다.', revise: '수정 내용을 임시 저장했습니다.', confirm: confirmed, archive: '초안을 보관했습니다. 이전 입력은 이력에 남아 있습니다.' }[request.action]
      completed.value += 1
      return response
    } catch (error) { failed(error); return null }
    finally { applying = false; busy.value = false }
  }

  async function showPreview() {
    if (!initial.recordId || !headId.value || locked.value || dirty.value || connection.value !== 'connected') return false
    busy.value = true
    failure.value = ''
    preview.value = null
    try {
      const result = await apiRequest<BusinessPreview>(`/api/business/${encodeURIComponent(initial.recordId)}/preview`, { expected_head_id: headId.value })
      preview.value = result
      issues.value = result.issues
      recovery.value = ''
      return result.can_confirm
    } catch (error) { failed(error); return false }
    finally { busy.value = false }
  }

  async function review() {
    if (!canReview.value || busy.value || connection.value !== 'connected') return false
    const requestId = pending.value?.body.meta.request_id
    busy.value = true
    try {
      const latest = initial.recordId ? await refresh() : null
      const state = latest?.state ?? (initial.recordId ? null : await apiRequest<CatalogState>('/api/state'))
      if (!state || pending.value?.body.meta.request_id !== requestId) return false
      sessionStorage.removeItem(pendingKey)
      basis.value = state
      if (latest) { headId.value = latest.head.id; status.value = latest.head.status; baseline.value = copyPayload(latest.head.payload) as T; issues.value = latest.issues }
      pending.value = null
      recovery.value = ''
      failure.value = ''
      preview.value = null
      notice.value = '최신 기록을 확인했습니다. 작성 중인 입력을 비교하고 필요한 내용을 임시 저장한 뒤 다시 미리보기해 주세요.'
      persistDraft()
      if (!mounted) completed.value += 1
      return true
    } catch { failure.value = '최신 기록이나 요청 보관 정보를 갱신하지 못했습니다. 다시 시도해 주세요.'; return false }
    finally { busy.value = false }
  }
  function resetNew() {
    if (initial.recordId || initial.corrects || busy.value || pending.value) return
    // A returned editor may have observed newer state than the previous create.
    // A reset in the same instance must also retain its newer create response.
    if (basis.value.epoch !== initial.state.epoch || BigInt(initial.state.revision) > BigInt(basis.value.revision)) basis.value = initial.state
    applying = true
    payload.value = copyPayload(initial.payload)
    baseline.value = copyPayload(initial.payload)
    applying = false
    created.value = null
    notice.value = ''
    issues.value = []
  }
  return { payload, status, issues, preview, busy, failure, recovery, notice, storageFailure, created, pending, dirty, locked, canRetry, canReview, send, showPreview, review, resetNew, synchronizeTerminal }
}
