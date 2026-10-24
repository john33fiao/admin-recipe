import type { CatalogState, MaterialDetail } from '~/types/material'
import type { RecipeDataInput, RecipeDetail, RecipeMutation, RecipeOwnerInput, RecipePending, RecipeVersion } from '~/types/recipe'
import { ApiError, apiRequest } from '~/utils/api'
import { blankRecipe, copyRecipe, normalizeRecipe, ownerDraftShape, recipeFailure, recipeFromVersion, recipeShape, restoreRecipePending, sameRecipe, validateRecipe, validateRecipeOwner } from '~/utils/recipe'
import { normalizeCatalog, validateCatalogInput } from '~/utils/catalog'

export interface RecipeEditorInitial { materialId: string | null; owner: RecipeOwnerInput; data: RecipeDataInput; head: RecipeVersion | null; state: CatalogState }
export function useRecipeEditor(identity: string, initial: RecipeEditorInitial, refresh: () => Promise<RecipeDetail | null>) {
  const key = `sunchang.recipe.${identity}`
  const pendingKey = `${key}.pending.v1`, draftKey = `${key}.draft.v1`
  const connection = useConnection()
  const pending = useState<RecipePending | null>(`${key}.pending`, () => {
    try { const value = restoreRecipePending(sessionStorage.getItem(pendingKey)); return value?.material_id === initial.materialId ? value : null } catch { return null }
  })
  const stored = useState<{ owner: RecipeOwnerInput; data: RecipeDataInput; basis: CatalogState; head: RecipeVersion | null } | null>(`${key}.stored`, () => {
    try { const value = JSON.parse(sessionStorage.getItem(draftKey) ?? 'null'); if (recipeShape(value?.data) && ownerDraftShape(value.owner) && typeof value.basis?.epoch === 'string' && /^\d+$/.test(value.basis?.revision) && (value.head === null || (typeof value.head?.id === 'string' && value.head.material_id === initial.materialId))) return value } catch { /* A malformed draft is not a request. */ }
    return null
  })
  const owner = useState<RecipeOwnerInput>(`${key}.owner`, () => ({ ...(pending.value?.action === 'create' ? pending.value.body.owner : stored.value?.owner ?? initial.owner) }))
  const data = useState<RecipeDataInput>(`${key}.data`, () => copyRecipe(pending.value && 'data' in pending.value.body ? pending.value.body.data : stored.value?.data ?? initial.data))
  const baseline = useState(`${key}.baseline`, () => copyRecipe(initial.data))
  const head = useState<RecipeVersion | null>(`${key}.head`, () => stored.value?.head ?? initial.head)
  const basis = useState<CatalogState>(`${key}.basis`, () => pending.value ? { epoch: pending.value.body.meta.epoch, revision: pending.value.body.meta.expected_revision } : stored.value?.basis ?? initial.state)
  const busy = useState(`${key}.busy`, () => false)
  const failure = useState(`${key}.failure`, () => '')
  const recovery = useState<'edit' | 'refresh' | 'blocked' | 'retry' | ''>(`${key}.recovery`, () => '')
  const notice = useState(`${key}.notice`, () => '')
  const storageFailure = useState(`${key}.storage-failure`, () => '')
  const result = useState<RecipeMutation | null>(`${key}.result`, () => null)
  const completed = useState(`${key}.completed`, () => 0)
  const created = useState(`${key}.created`, () => false)
  const dirty = computed(() => !sameRecipe(data.value, baseline.value))
  const locked = computed(() => busy.value || !!pending.value || created.value)
  const canRetry = computed(() => !!pending.value && (!recovery.value || recovery.value === 'retry'))
  const canReview = computed(() => recovery.value === 'edit' || recovery.value === 'refresh')
  let applying = false
  function persist() {
    if (applying || pending.value) return
    try { sessionStorage.setItem(draftKey, JSON.stringify({ owner: owner.value, data: data.value, basis: basis.value, head: head.value })); storageFailure.value = '' } catch { storageFailure.value = '이 탭에 미저장 입력을 보관하지 못했습니다. 화면을 닫기 전에 저장해 주세요.' }
  }
  watch([data, owner], () => { if (!applying) notice.value = ''; persist() }, { deep: true, flush: 'sync' })
  watch(completed, () => { void refresh() })
  function synchronize(latest: RecipeEditorInitial) {
    if (locked.value || dirty.value || stored.value || recovery.value || !latest.head) return
    applying = true
    owner.value = { ...latest.owner }; data.value = copyRecipe(latest.data); baseline.value = copyRecipe(latest.data); basis.value = latest.state; head.value = latest.head
    applying = false
  }
  async function send(action: RecipePending['action']): Promise<RecipeMutation | null> {
    if (busy.value || created.value || connection.value !== 'connected' || (recovery.value && recovery.value !== 'retry') || (pending.value && pending.value.action !== action)) return null
    let request = pending.value
    if (!request) {
      if (action !== 'copy' && Object.keys(validateRecipe(data.value)).length) return null
      const meta = { request_id: crypto.randomUUID(), epoch: basis.value.epoch, expected_revision: basis.value.revision }
      if (action === 'create' && !initial.materialId) {
        if (Object.keys(validateRecipeOwner(owner.value)).length) return null
        const selected = owner.value.mode === 'new' ? { mode: 'new' as const, ...normalizeCatalog(owner.value) } : { ...owner.value }
        request = { action, material_id: null, body: { meta, owner: selected, data: normalizeRecipe(data.value) } }
      } else if (initial.materialId && head.value && owner.value.mode === 'existing') {
        const common = { meta, expected_material_version_id: owner.value.expected_material_version_id }
        request = action === 'copy' ? { action, material_id: initial.materialId, body: { ...common, source_recipe_version_id: head.value.id } } : { action: 'revise', material_id: initial.materialId, body: { ...common, expected_recipe_version_id: head.value.id, data: normalizeRecipe(data.value) } }
      } else return null
    }
    try { sessionStorage.setItem(pendingKey, JSON.stringify(request)) } catch { storageFailure.value = '요청을 보관하지 못해 전송하지 않았습니다. 브라우저 저장공간을 확인해 주세요.'; return null }
    pending.value = request; busy.value = true; failure.value = ''; notice.value = ''
    try {
      const path = request.action === 'create' ? '/api/recipes' : `/api/recipes/${encodeURIComponent(request.material_id)}/${request.action === 'copy' ? 'copies' : 'versions'}`
      const response = await apiRequest<RecipeMutation>(path, request.body)
      applying = true; basis.value = response.state; result.value = response
      if (request.action === 'create') created.value = true
      else if (request.action === 'revise') { data.value = recipeFromVersion(response.version); baseline.value = copyRecipe(data.value); head.value = response.version; owner.value = { mode: 'existing', material_id: response.item.id, expected_material_version_id: response.item.version_id } }
      // Persist explicitly: the sender's component may have unmounted while awaiting the reply.
      try {
        if (request.action === 'copy') sessionStorage.setItem(draftKey, JSON.stringify({ owner: owner.value, data: data.value, basis: basis.value, head: head.value }))
        else sessionStorage.removeItem(draftKey)
        sessionStorage.removeItem(pendingKey)
      } catch { /* Immutable pending remains safe to replay. */ }
      stored.value = null; pending.value = null; recovery.value = ''
      notice.value = request.action === 'copy' ? `${response.item.name} 이름으로 독립 복제했습니다.` : '레시피를 저장했습니다. 이전 구성은 버전 이력에서 확인할 수 있습니다.'
      completed.value += 1
      return response
    } catch (cause) {
      recovery.value = cause instanceof ApiError ? recipeFailure(cause.status, cause.code) : 'retry'
      failure.value = recovery.value === 'refresh' ? '다른 변경으로 이 요청은 저장하지 않았습니다. 최신 저장본을 확인하고 작성 중인 입력을 다시 검토해 주세요.' : recovery.value === 'blocked' ? '이 요청 ID에 다른 처리 이력이 있습니다. 자동으로 다시 저장하지 않습니다. 기록을 확인해 주세요.' : recovery.value === 'edit' ? (cause instanceof Error ? cause.message : '입력을 확인해 주세요.') : '완료를 확인하지 못했습니다. 요청과 입력을 유지했습니다. 같은 요청으로 재시도해 주세요.'
      return null
    } finally { applying = false; busy.value = false }
  }
  async function review() {
    if (!canReview.value || busy.value || !pending.value || connection.value !== 'connected') return false
    const requestId = pending.value.body.meta.request_id
    busy.value = true
    try {
      const latest = initial.materialId ? await refresh() : null
      if (initial.materialId && !latest) return false
      const material = !initial.materialId && owner.value.mode === 'existing' ? await apiRequest<MaterialDetail>(`/api/materials/${encodeURIComponent(owner.value.material_id)}`) : null
      const state = latest?.state ?? (material ? { epoch: material.epoch, revision: material.revision } : await apiRequest<CatalogState>('/api/state'))
      if (pending.value?.body.meta.request_id !== requestId) return false
      sessionStorage.removeItem(pendingKey)
      basis.value = state
      if (latest) { head.value = latest.head; baseline.value = recipeFromVersion(latest.head); owner.value = { mode: 'existing', material_id: latest.item.id, expected_material_version_id: latest.item.version_id } }
      else if (material) owner.value = { mode: 'existing', material_id: material.id, expected_material_version_id: material.version_id }
      pending.value = null; stored.value = null; recovery.value = ''; failure.value = ''; notice.value = '최신 저장본을 확인했습니다. 작성 중인 입력과 비교한 뒤 다시 저장해 주세요.'; persist()
      return true
    } catch { failure.value = '최신 자료나 보관된 요청을 갱신하지 못했습니다. 다시 시도해 주세요.'; return false }
    finally { busy.value = false }
  }
  function resetNew() {
    if (initial.materialId || pending.value || busy.value) return
    applying = true
    owner.value = { mode: 'new', name: '', unit: '개', kind: 'menu', description: '' }; data.value = blankRecipe(); baseline.value = blankRecipe(); head.value = null
    if (basis.value.epoch !== initial.state.epoch || BigInt(initial.state.revision) > BigInt(basis.value.revision)) basis.value = initial.state
    result.value = null; created.value = false; stored.value = null; notice.value = ''; applying = false; persist()
  }
  return { owner, data, head, basis, busy, pending, dirty, locked, canRetry, canReview, failure, notice, storageFailure, result, created, send, review, synchronize, resetNew }
}
