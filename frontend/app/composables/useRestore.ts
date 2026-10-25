import type { RestoreFile, RestorePreview, RestoreRequest, RestoreResult, RestoreSelection, RestoreSource, RestoreTarget, RestoreTargetPage } from '~/types/restore'
import { ApiError, apiRequest } from '~/utils/api'
import { announceRestore, currentAppState } from '~/utils/current-state'
import { describeRestoreFile, restoreFileDescriptor, restorePendingRequest, sameRestoreFile } from '~/utils/restore'
import { failureAction } from '~/utils/catalog'

const uploadFile = shallowRef<File | null>(null)
const pendingKey = 'sunchang.restore.pending.v1', uploadKey = 'sunchang.restore.import.v1'
const read = (key: string) => { try { return sessionStorage.getItem(key) } catch { return null } }
export function useRestore() {
  const connection = useConnection()
  const source = ref<RestoreSource>('journal'), page = ref<RestoreTargetPage | null>(null), loading = ref(false), listFailure = ref('')
  const pending = useState<RestoreRequest | null>('restore.pending', () => restorePendingRequest(read(pendingKey)))
  const selected = useState<RestoreSelection | null>('restore.selected', () => pending.value ? { target_id: pending.value.target_id, expected_sha256: pending.value.expected_sha256 } : null)
  const preview = useState<RestorePreview | null>('restore.preview', () => null), result = useState<RestoreResult | null>('restore.result', () => null)
  const busy = useState('restore.busy', () => false), failure = useState('restore.failure', () => ''), notice = useState('restore.notice', () => '')
  const recovery = useState<'' | 'retry' | 'edit' | 'refresh' | 'blocked'>('restore.recovery', () => '')
  const importDescriptor = useState<RestoreFile | null>('restore.import', () => restoreFileDescriptor(read(uploadKey)))
  const importing = useState('restore.importing', () => false), hashing = useState('restore.hashing', () => false), fileGeneration = useState('restore.file-generation', () => 0)
  const importFailure = useState('restore.import-failure', () => ''), imported = useState<RestoreTarget | null>('restore.imported', () => null)
  const malformedPending = !!read(pendingKey) && !pending.value
  const outdated = computed(() => !!preview.value && !!currentAppState.value && (preview.value.state.epoch !== currentAppState.value.epoch || preview.value.state.revision !== currentAppState.value.revision))
  const resultPast = computed(() => !!result.value && !!currentAppState.value && result.value.state.epoch !== currentAppState.value.epoch)
  const canConfirm = computed(() => !busy.value && !pending.value && !malformedPending && !result.value && !!preview.value?.can_restore && !outdated.value && connection.value === 'connected')
  const canRetry = computed(() => !!pending.value && (!recovery.value || recovery.value === 'retry'))
  const canReview = computed(() => !!pending.value && (recovery.value === 'edit' || recovery.value === 'refresh'))
  let listGeneration = 0, previewGeneration = 0, alive = true
  onScopeDispose(() => { alive = false; listGeneration++; previewGeneration++ })
  async function load(cursor?: string) {
    const request = ++listGeneration, kind = source.value
    if (connection.value !== 'connected') return
    loading.value = true; listFailure.value = ''
    try {
      const value = await apiRequest<RestoreTargetPage>(`/api/restore-targets?source=${kind}${cursor ? `&cursor=${encodeURIComponent(cursor)}` : ''}`)
      if (request !== listGeneration || kind !== source.value || !alive) return
      if (value.source !== kind) throw new Error('source-mismatch')
      page.value = value
    } catch (error) { if (request === listGeneration) listFailure.value = error instanceof Error ? error.message : '복원 지점을 불러오지 못했습니다.' }
    finally { if (request === listGeneration) loading.value = false }
  }
  function choose(target: RestoreTarget) {
    if (pending.value || busy.value || malformedPending) return
    previewGeneration++; selected.value = { target_id: target.target_id, expected_sha256: target.sha256 }; preview.value = null; result.value = null; failure.value = ''; notice.value = ''; recovery.value = ''
  }
  async function showPreview(review = false) {
    if (busy.value || !selected.value || malformedPending || connection.value !== 'connected' || pending.value && !(review && canReview.value)) return false
    const request = ++previewGeneration, target = { ...selected.value }, original = pending.value
    busy.value = true; failure.value = ''
    try {
      const value = await apiRequest<RestorePreview>('/api/restore/preview', target)
      if (!alive || request !== previewGeneration || selected.value?.target_id !== target.target_id || selected.value.expected_sha256 !== target.expected_sha256) return false
      if (value.target.target_id !== target.target_id || value.target.sha256 !== target.expected_sha256) throw new Error('선택한 복원 지점과 미리보기 결과가 일치하지 않습니다.')
      if (review && pending.value?.meta.request_id === original?.meta.request_id) {
        // Only a confirmed not-processed/edit response permits an explicit new review.
        if (original) sessionStorage.setItem(`sunchang.restore.reviewed.${original.meta.request_id}`, JSON.stringify(original))
        sessionStorage.removeItem(pendingKey); pending.value = null
      }
      preview.value = value; recovery.value = ''; result.value = null
      return value.can_restore
    } catch (error) { failure.value = error instanceof Error ? error.message : '미리보기를 확인하지 못했습니다.'; return false }
    finally { busy.value = false }
  }
  async function confirm() {
    if (busy.value || connection.value !== 'connected' || (!pending.value && !canConfirm.value) || pending.value && !canRetry.value) return false
    const request = pending.value ?? { ...selected.value!, meta: { request_id: crypto.randomUUID(), epoch: preview.value!.state.epoch, expected_revision: preview.value!.state.revision } }
    try { sessionStorage.setItem(pendingKey, JSON.stringify(request)) } catch { failure.value = '원요청을 이 탭에 보관하지 못해 복원을 보내지 않았습니다. 브라우저 저장공간을 확인해 주세요.'; return false }
    pending.value = request; busy.value = true; failure.value = ''; notice.value = ''
    try {
      const value = await apiRequest<RestoreResult>('/api/restores', request)
      result.value = value; preview.value = null; pending.value = null; recovery.value = ''
      try { sessionStorage.removeItem(pendingKey) } catch { /* A retained immutable request can be safely checked again. */ }
      notice.value = '선택한 전체 상태를 복원했습니다. 복원 직전 자료와 기존 기록·사진은 보존했습니다.'
      announceRestore()
      // A replay may return an earlier revision in this same generation.
      try { await apiRequest('/api/state') } catch { notice.value += ' 최신 자료 조회는 연결 후 다시 확인해 주세요.' }
      return true
    } catch (error) {
      recovery.value = error instanceof ApiError ? failureAction(error.status, error.code, error.requestStatus) : 'retry'
      failure.value = error instanceof ApiError && recovery.value !== 'retry' ? error.message : '복원 완료를 확인하지 못했습니다. 이미 적용되었을 수 있습니다. 원요청을 유지했으니 같은 요청으로 결과를 확인해 주세요.'
      return false
    } finally { busy.value = false }
  }
  function startAnother() {
    if (busy.value || pending.value && recovery.value !== 'blocked' || malformedPending) return
    try {
      if (pending.value) { sessionStorage.setItem(`sunchang.restore.retained.${pending.value.meta.request_id}`, JSON.stringify(pending.value)); sessionStorage.removeItem(pendingKey) }
    } catch { failure.value = '원요청을 보관하지 못해 새 검토를 시작하지 않았습니다.'; return }
    pending.value = null; selected.value = null; preview.value = null; result.value = null; recovery.value = ''; failure.value = ''; notice.value = '다른 복원 작업은 지점을 새로 선택하고 전체 영향을 확인한 뒤 확정해 주세요.'
  }
  async function selectFile(file: File | null) {
    if (!file || importing.value || hashing.value) return false
    const generation = ++fileGeneration.value; hashing.value = true; importFailure.value = ''
    try {
      const descriptor = await describeRestoreFile(file)
      if (generation !== fileGeneration.value) return false
      if (importDescriptor.value && !sameRestoreFile(importDescriptor.value, descriptor)) throw new Error('결과를 확인할 원래 JSON과 다른 파일입니다. 같은 내용을 가진 파일을 다시 선택해 주세요.')
      sessionStorage.setItem(uploadKey, JSON.stringify(descriptor)); importDescriptor.value = descriptor; uploadFile.value = file; imported.value = null
      return true
    } catch (error) { importFailure.value = error instanceof Error ? error.message : '파일을 확인하지 못했습니다.'; return false }
    finally { if (generation === fileGeneration.value) hashing.value = false }
  }
  async function importJson() {
    if (importing.value || hashing.value || !uploadFile.value || !importDescriptor.value || connection.value !== 'connected') return false
    importing.value = true; importFailure.value = ''
    try {
      const value = await apiRequest<RestoreTarget>('/api/restore-targets/import', uploadFile.value)
      if (value.sha256 !== importDescriptor.value.sha256 || value.target_id !== `import:${value.sha256}`) throw new Error('선택한 파일과 보존된 지점이 일치하지 않습니다.')
      imported.value = value; choose(value)
      try { sessionStorage.removeItem(uploadKey) } catch { /* Same bytes identify the same target. */ }
      importDescriptor.value = null; uploadFile.value = null
      if (alive) { source.value = 'import'; await load() }
      return true
    } catch (error) { importFailure.value = error instanceof ApiError ? error.message : '가져오기 결과를 확인하지 못했습니다. 같은 파일로 다시 확인할 수 있습니다.'; return false }
    finally { importing.value = false }
  }
  function changeFile() {
    if (importing.value || hashing.value) return
    try {
      if (importDescriptor.value) sessionStorage.setItem(`sunchang.restore.retained.import.${importDescriptor.value.sha256}`, JSON.stringify(importDescriptor.value))
      sessionStorage.removeItem(uploadKey)
    } catch { importFailure.value = '원파일 확인 정보를 보관하지 못해 선택을 유지합니다.'; return }
    importDescriptor.value = null; uploadFile.value = null; importFailure.value = ''; imported.value = null
  }
  watch([connection, source], () => { void load() }, { immediate: true })
  return { connection, source, page, loading, listFailure, selected, preview, result, busy, failure, notice, pending, recovery, outdated, resultPast, canConfirm, canRetry, canReview, malformedPending, importDescriptor, imported, importing, hashing, importFailure, uploadFile, load, choose, showPreview, confirm, startAnother, selectFile, importJson, changeFile }
}
