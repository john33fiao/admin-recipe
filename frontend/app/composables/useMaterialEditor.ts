import type { CatalogInput, MaterialDetail, MaterialMutation, MaterialPriceVersion, MutationResult, PriceInput } from '~/types/material'
import { ApiError, apiRequest } from '~/utils/api'
import { decimalInput, failureAction, normalizeCatalog, restoreMaterialMutation } from '~/utils/catalog'

export function useMaterialEditor(initial: MaterialDetail, refresh: () => Promise<MaterialDetail | null>) {
  const connection = useConnection()
  const key = `sunchang.material-editor.${initial.id}`
  const pendingKey = `${key}.pending.v1`
  const draftKey = `${key}.draft.v1`
  const pending = useState<MaterialMutation | null>(`${key}.pending`, () => {
    try { return restoreMaterialMutation(sessionStorage.getItem(pendingKey)) } catch { return null }
  })
  type Basis = { epoch: string; revision: string; version_id: string }
  const draft = useState<{ edit: CatalogInput; price: PriceInput; basis: Basis } | null>(`${key}.draft`, () => {
    try {
      const value = JSON.parse(sessionStorage.getItem(draftKey) ?? 'null')
      if (value && typeof value === 'object' && value.edit && value.price && value.basis) {
        const fields = ['name', 'unit', 'kind', 'description'].every(field => typeof value.edit[field] === 'string')
        const prices = ['amount', 'quantity', 'unit'].every(field => typeof value.price[field] === 'string')
        const meta = ['epoch', 'revision', 'version_id'].every(field => typeof value.basis[field] === 'string')
        if (fields && prices && meta) return value
      }
    } catch { /* Invalid tab content is not a request. */ }
    return null
  })
  // A pending request can outlive its page. Its inputs and revision must do so too.
  const edit = useState<CatalogInput>(`${key}.edit`, () => pending.value?.action === 'version'
    ? { name: pending.value.body.name, unit: pending.value.body.unit, kind: pending.value.body.kind, description: pending.value.body.description }
    : draft.value?.edit ?? { name: initial.name, unit: initial.unit, kind: initial.kind, description: initial.description })
  const price = useState<PriceInput>(`${key}.price`, () => pending.value?.action === 'price'
    ? { amount: pending.value.body.amount, quantity: pending.value.body.quantity, unit: pending.value.body.unit }
    : draft.value?.price ?? { amount: '', quantity: '1', unit: initial.unit })
  const basis = useState<Basis>(`${key}.basis`, () => pending.value
    ? { epoch: pending.value.body.epoch, revision: pending.value.body.expected_revision, version_id: pending.value.body.expected_version_id }
    : draft.value?.basis ?? { epoch: initial.epoch, revision: initial.revision, version_id: initial.version_id })
  const draftRestored = useState(`${key}.draft-restored`, () => !!draft.value)
  const saving = useState(`${key}.saving`, () => false)
  const reviewing = useState(`${key}.reviewing`, () => false)
  const failure = useState(`${key}.failure`, () => '')
  const recovery = useState<'edit' | 'refresh' | 'blocked' | 'retry' | ''>(`${key}.recovery`, () => '')
  const saved = useState(`${key}.saved`, () => '')
  const draftFailure = useState(`${key}.draft-failure`, () => '')
  const completed = useState(`${key}.completed`, () => 0)
  const locked = computed(() => !!pending.value || saving.value || reviewing.value)
  const canRetry = computed(() => !!pending.value && (!recovery.value || recovery.value === 'retry'))
  const canReview = computed(() => recovery.value === 'refresh' || recovery.value === 'edit')
  let completing = false

  function persistDraft() {
    if (completing || pending.value) return
    try {
      sessionStorage.setItem(draftKey, JSON.stringify({ edit: edit.value, price: price.value, basis: basis.value }))
      draftFailure.value = ''
    } catch { draftFailure.value = '작성 중인 내용을 이 탭에 보관하지 못했습니다. 새로고침하면 사라질 수 있습니다.' }
  }
  watch([edit, price], persistDraft, { deep: true, flush: 'sync' })
  // Vue disposes the old page's watcher; only the mounted editor refreshes its view.
  watch(completed, () => { void refresh() })

  async function send(action: MaterialMutation['action']): Promise<boolean> {
    if (saving.value || reviewing.value || connection.value !== 'connected' || (recovery.value && recovery.value !== 'retry')) return false
    if (pending.value && pending.value.action !== action) return false
    const meta = { request_id: crypto.randomUUID(), epoch: basis.value.epoch, expected_revision: basis.value.revision, expected_version_id: basis.value.version_id }
    let request: MaterialMutation
    if (pending.value) request = pending.value
    else if (action === 'version') request = { action, body: { ...normalizeCatalog(edit.value), ...meta } }
    else if (action === 'price') request = { action, body: { amount: decimalInput(price.value.amount, 'money')!, quantity: decimalInput(price.value.quantity, 'quantity')!, unit: price.value.unit.trim(), ...meta } }
    else request = { action, body: meta }
    try { sessionStorage.setItem(pendingKey, JSON.stringify(request)) } catch { failure.value = '요청을 이 탭에 보관하지 못해 전송하지 않았습니다. 브라우저 저장공간을 확인해 주세요.'; return false }
    pending.value = request
    saving.value = true
    failure.value = ''
    saved.value = ''
    try {
      const suffix = { version: 'versions', price: 'prices', archive: 'archive' }[request.action]
      const result = await apiRequest<MutationResult<MaterialDetail | MaterialPriceVersion>>(`/api/materials/${encodeURIComponent(initial.id)}/${suffix}`, request.body)
      // Use the successful request's revision before any subsequent edit.
      completing = true
      basis.value = { epoch: result.epoch, revision: result.revision, version_id: 'version_id' in result ? result.version_id : request.body.expected_version_id }
      if (request.action === 'version') edit.value = { name: request.body.name, unit: request.body.unit, kind: request.body.kind, description: request.body.description }
      if (request.action === 'price') price.value = { amount: '', quantity: '1', unit: edit.value.unit }
      // Preserve the other form's unfinished input, then clear the immutable key.
      try {
        sessionStorage.setItem(draftKey, JSON.stringify({ edit: edit.value, price: price.value, basis: basis.value }))
        sessionStorage.removeItem(pendingKey)
      } catch { /* A persisted immutable request is safe to replay. */ }
      pending.value = null
      recovery.value = ''
      draftRestored.value = false
      saved.value = { version: '재료 정보를 저장했습니다.', price: '사전 가격을 등록했습니다.', archive: '재료를 보관했습니다. 과거 기록은 계속 확인할 수 있습니다.' }[request.action]
      completed.value += 1
      return true
    } catch (error) {
      recovery.value = error instanceof ApiError ? failureAction(error.status, error.code) : 'retry'
      if (recovery.value === 'refresh') failure.value = '다른 변경으로 자료가 갱신되어 이 요청은 저장하지 않았습니다. 최신 상태를 확인한 뒤 입력을 다시 확인해 주세요.'
      else if (recovery.value === 'blocked') failure.value = '이 요청 ID에 다른 처리 이력이 있어 재전송할 수 없습니다. 변경 이력을 먼저 확인해 주세요.'
      else if (recovery.value === 'edit') failure.value = error instanceof Error ? error.message : '입력을 확인해 주세요.'
      else failure.value = '저장 완료를 확인하지 못했습니다. 입력과 요청 정보를 유지했습니다. 연결을 확인한 뒤 같은 요청을 재시도해 주세요.'
      return false
    } finally { completing = false; saving.value = false }
  }

  async function review() {
    if (!canReview.value || saving.value || reviewing.value || !pending.value) return false
    const requestId = pending.value.body.request_id
    reviewing.value = true
    try {
      const latest = await refresh()
      if (!latest || pending.value?.body.request_id !== requestId) return false
      sessionStorage.removeItem(pendingKey)
      basis.value = { epoch: latest.epoch, revision: latest.revision, version_id: latest.version_id }
      pending.value = null
      recovery.value = ''
      failure.value = ''
      saved.value = '최신 정보를 불러왔습니다. 작성 중인 내용과 비교한 뒤 다시 저장해 주세요.'
      persistDraft()
      return true
    } catch { failure.value = '최신 상태나 요청 보관 정보를 갱신하지 못했습니다. 연결과 브라우저 저장공간을 확인해 주세요.'; return false }
    finally { reviewing.value = false }
  }
  return { edit, price, pending, saving, reviewing, failure, recovery, saved, locked, canRetry, canReview, draftRestored, draftFailure, send, review }
}
