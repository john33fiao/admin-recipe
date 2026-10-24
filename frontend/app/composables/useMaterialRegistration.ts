import type { CatalogState, Material, MaterialKind, MutationResult, RegistrationRequest } from '~/types/material'
import { ApiError, apiRequest } from '~/utils/api'
import { failureAction, prepareCatalogRegistration, restoreCatalogRegistration } from '~/utils/catalog'
import { restoreCatalogDraft } from '~/utils/draft'

const storageKey = 'sunchang.pending-material.v1'
const draftStorageKey = 'sunchang.draft-material.v1'

export function useMaterialRegistration() {
  const connection = useConnection()
  const pending = useState<RegistrationRequest | null>('pending-material', () => {
    try { return restoreCatalogRegistration(sessionStorage.getItem(storageKey)) } catch { return null }
  })
  const draft = useState('material-draft', () => {
    try { return restoreCatalogDraft(sessionStorage.getItem(draftStorageKey)) } catch { return null }
  })
  const name = useState('material-name', () => pending.value?.name ?? draft.value?.name ?? '')
  const unit = useState('material-unit', () => pending.value?.unit ?? draft.value?.unit ?? 'g')
  const kind = useState<MaterialKind>('material-kind', () => pending.value ? ('kind' in pending.value ? pending.value.kind : 'unclassified') : draft.value?.kind ?? 'raw')
  const description = useState('material-description', () => pending.value ? ('description' in pending.value ? pending.value.description : '') : draft.value?.description ?? '')
  const draftRestored = useState('material-draft-restored', () => !pending.value && !!draft.value)
  const draftFailure = useState('material-draft-failure', () => '')
  const saving = useState('material-saving', () => false)
  const reviewing = useState('material-reviewing', () => false)
  const failure = useState('material-failure', () => '')
  const recovery = useState<'edit' | 'refresh' | 'blocked' | 'retry' | ''>('material-recovery', () => '')
  const rejected = computed(() => ['edit', 'refresh', 'blocked'].includes(recovery.value))
  const saved = useState<Material | null>('material-saved', () => null)
  const state = useState<CatalogState | null>('material-registration-state', () => null)

  watch([name, unit, kind, description], () => {
    if (pending.value) return
    try {
      if (name.value === '' && unit.value === 'g' && kind.value === 'raw' && description.value === '') sessionStorage.removeItem(draftStorageKey)
      else sessionStorage.setItem(draftStorageKey, JSON.stringify({ name: name.value, unit: unit.value, kind: kind.value, description: description.value }))
      draftFailure.value = ''
    } catch { draftFailure.value = '입력 내용을 이 탭에 보관하지 못했습니다. 새로고침하면 사라질 수 있습니다.' }
  }, { flush: 'sync' })

  async function save(): Promise<Material | null> {
    if (saving.value || reviewing.value || connection.value !== 'connected' || rejected.value) return null
    saving.value = true
    failure.value = ''
    saved.value = null
    try {
      if (!pending.value && !state.value) state.value = await apiRequest<CatalogState>('/api/state')
      const request = pending.value ?? prepareCatalogRegistration({ name: name.value, unit: unit.value, kind: kind.value, description: description.value }, state.value!, null, () => crypto.randomUUID())
      try {
        sessionStorage.setItem(storageKey, JSON.stringify(request))
      } catch {
        failure.value = '입력 내용을 임시 보관하지 못해 등록을 보내지 않았습니다. 브라우저의 사이트 저장공간 허용 여부를 확인해 주세요.'
        return null
      }
      pending.value = request
      name.value = request.name
      unit.value = request.unit
      const material = await apiRequest<MutationResult<Material>>('/api/materials', request)
      saved.value = material
      if (material.epoch && material.revision) state.value = { epoch: material.epoch, revision: material.revision }
      else state.value = null
      // Completion must work after the form watchers have unmounted.
      try { sessionStorage.removeItem(draftStorageKey); sessionStorage.removeItem(storageKey) } catch { /* The immutable request remains safe to replay. */ }
      draftRestored.value = false
      draftFailure.value = ''
      name.value = ''
      unit.value = 'g'
      kind.value = 'raw'
      description.value = ''
      pending.value = null
      recovery.value = ''
      return material
    } catch (error) {
      recovery.value = error instanceof ApiError ? failureAction(error.status, error.code) : 'retry'
      if (recovery.value === 'refresh') failure.value = '다른 변경으로 자료가 갱신되어 이 등록은 저장하지 않았습니다. 최신 상태를 확인한 뒤 입력을 다시 확인해 주세요.'
      else if (recovery.value === 'blocked') failure.value = '이 요청 ID에 다른 처리 이력이 있어 재전송할 수 없습니다. 목록에서 저장 결과를 먼저 확인해 주세요.'
      else if (recovery.value === 'edit') failure.value = error instanceof Error ? error.message : '입력 내용을 확인해 주세요.'
      else failure.value = '등록 완료를 확인하지 못했습니다. 자료가 저장되었더라도 변경 기록이 아직 남지 않았을 수 있습니다. 입력 내용을 유지했으니 연결을 확인한 뒤 같은 요청을 재시도해 주세요.'
      return null
    } finally { saving.value = false }
  }

  async function editRejected() {
    if (!rejected.value || recovery.value === 'blocked' || saving.value || reviewing.value || !pending.value) return false
    const requestId = pending.value.request_id
    reviewing.value = true
    try {
      const latest = await apiRequest<CatalogState>('/api/state')
      if (pending.value?.request_id !== requestId) return false
      sessionStorage.removeItem(storageKey)
      state.value = latest
      pending.value = null
      recovery.value = ''
      failure.value = ''
      return true
    } catch { failure.value = '최신 상태를 확인하지 못했습니다. 연결을 확인하고 다시 시도해 주세요.'; return false }
    finally { reviewing.value = false }
  }
  return { name, unit, kind, description, pending, saving, reviewing, failure, saved, rejected, recovery, draftRestored, draftFailure, save, editRejected }
}
