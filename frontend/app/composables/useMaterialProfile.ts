import type { ShallowRef } from 'vue'
import type { ProfileBasis, ProfileDetail, ProfileFile, ProfileHistory, ProfileInput, ProfilePending, ProfilePhoto } from '~/types/profile'
import { ApiError, apiRequest } from '~/utils/api'
import { failureAction } from '~/utils/catalog'
import { blankProfile, copyProfile, describeProfileFile, normalizeProfile, profileBasisShape, profileFileShape, profileFromVersion, profileShape, removeProfilePhoto, restoreProfilePending, sameProfile, sameProfileFile, validateProfile } from '~/utils/profile'

// File bytes belong to this browser lifetime, never Nuxt or session JSON.
const profileFiles = new Map<string, ShallowRef<File | null>>()

export function useMaterialProfile(materialId: string) {
  const connection = useConnection(), key = `sunchang.profile.${materialId}`
  const pendingKey = `${key}.pending.v1`, draftKey = `${key}.draft.v1`
  const read = (storageKey: string) => { try { return sessionStorage.getItem(storageKey) } catch { return null } }
  const pending = useState<ProfilePending | null>(`${key}.pending`, () => restoreProfilePending(read(pendingKey), materialId))
  const restoreFailure = useState(`${key}.restore-failure`, () => !!read(pendingKey) && !pending.value ? '보관된 사진 요청을 확인할 수 없어 새 요청을 전송하지 않습니다. 이 탭의 기록을 유지한 채 저장 상태를 확인해 주세요.' : '')
  const restored = (() => {
    try {
      const value = JSON.parse(read(draftKey) ?? 'null')
      if (value && profileShape(value.input) && profileBasisShape(value.basis) && (value.file === null || profileFileShape(value.file))) return value as { input: ProfileInput; basis: ProfileBasis; file: ProfileFile | null }
    } catch { /* A corrupt tab draft is not a mutation. */ }
    return null
  })()
  const input = useState<ProfileInput>(`${key}.input`, () => pending.value ? copyProfile(pending.value.body) : restored?.input ?? blankProfile())
  const basis = useState<ProfileBasis | null>(`${key}.basis`, () => pending.value ? { state: { epoch: pending.value.body.meta.epoch, revision: pending.value.body.meta.expected_revision }, material_version_id: pending.value.body.expected_material_version_id, profile_version_id: pending.value.body.expected_profile_version_id } : restored?.basis ?? null)
  const fileDescriptor = useState<ProfileFile | null>(`${key}.file-descriptor`, () => pending.value?.file ?? restored?.file ?? null)
  if (!profileFiles.has(materialId)) profileFiles.set(materialId, shallowRef<File | null>(null))
  const file = profileFiles.get(materialId)!
  const current = useState<ProfileDetail | null>(`${key}.current`, () => null)
  const baseline = useState<ProfileInput | null>(`${key}.baseline`, () => null)
  const photos = useState<Record<string, ProfilePhoto>>(`${key}.photos`, () => ({}))
  const busy = useState(`${key}.busy`, () => false), reviewing = useState(`${key}.reviewing`, () => false), hashing = useState(`${key}.hashing`, () => false)
  const hashGeneration = useState(`${key}.hash-generation`, () => 0)
  const completed = useState(`${key}.completed`, () => 0)
  const failure = useState(`${key}.failure`, () => ''), notice = useState(`${key}.notice`, () => ''), storageFailure = useState(`${key}.storage-failure`, () => '')
  const recovery = useState<'' | 'edit' | 'refresh' | 'blocked' | 'retry'>(`${key}.recovery`, () => '')
  const errors = useState<Record<string, string>>(`${key}.errors`, () => ({}))
  const loading = ref(false), readFailure = ref('')
  const history = ref<ProfileHistory | null>(null), historyLoading = ref(false), historyFailure = ref('')
  const dirty = computed(() => baseline.value ? !sameProfile(input.value, baseline.value) : !!restored)
  const locked = computed(() => !!pending.value || busy.value || reviewing.value || hashing.value || !!restoreFailure.value)
  const canRetry = computed(() => !!pending.value && (!recovery.value || recovery.value === 'retry'))
  const canReview = computed(() => !!pending.value && (recovery.value === 'edit' || recovery.value === 'refresh'))
  const needsFile = computed(() => input.value.photos.some(photo => photo.source === 'upload') && !file.value)
  let active = true, generation = 0, historyGeneration = 0
  onScopeDispose(() => { active = false; generation++; historyGeneration++ })

  function remember(detail: ProfileDetail) { for (const photo of detail.version?.photos ?? []) photos.value[photo.id] = photo }
  function adopt(detail: ProfileDetail) {
    input.value = profileFromVersion(detail.version)
    baseline.value = copyProfile(input.value)
    basis.value = { state: { ...detail.state }, material_version_id: detail.material.version_id, profile_version_id: detail.version?.id ?? null }
    file.value = null; fileDescriptor.value = null
  }
  function persist() {
    if (pending.value || !basis.value) return
    try { sessionStorage.setItem(draftKey, JSON.stringify({ input: input.value, basis: basis.value, file: fileDescriptor.value })); storageFailure.value = '' }
    catch { storageFailure.value = '작성 중인 사진·구입처를 이 탭에 보관하지 못했습니다. 새로고침하면 사라질 수 있습니다.' }
  }
  watch([input, basis, fileDescriptor], persist, { deep: true, flush: 'sync' })

  async function load(): Promise<ProfileDetail | null> {
    const token = ++generation
    if (!active || connection.value !== 'connected') { loading.value = false; return null }
    loading.value = true
    try {
      const result = await apiRequest<ProfileDetail>(`/api/materials/${materialId}/profile`)
      if (!active || token !== generation) return null
      if (current.value?.state.epoch === result.state.epoch && BigInt(current.value.state.revision) > BigInt(result.state.revision)) return current.value
      const clean = !pending.value && (!dirty.value || (!baseline.value && sameProfile(input.value, profileFromVersion(result.version))))
      current.value = result; remember(result); readFailure.value = ''
      if (clean) adopt(result)
      else if (!baseline.value) baseline.value = profileFromVersion(result.version)
      const expected = basis.value?.profile_version_id
      if (expected && expected !== result.version?.id) {
        try { const original = await apiRequest<ProfileDetail>(`/api/materials/${materialId}/profiles/${expected}`); if (active && token === generation) remember(original) } catch { /* Original references remain visible as IDs if their view is unavailable. */ }
      }
      return result
    } catch { if (active && token === generation) readFailure.value = '사진·구입처를 읽지 못했습니다. 기존 입력은 유지했습니다.'; return null }
    finally { if (active && token === generation) loading.value = false }
  }
  async function loadHistory() {
    const token = ++historyGeneration
    if (connection.value !== 'connected') return
    historyLoading.value = true
    try { const result = await apiRequest<ProfileHistory>(`/api/materials/${materialId}/profiles`); if (active && token === historyGeneration) { history.value = result; historyFailure.value = '' } }
    catch { if (active && token === historyGeneration) historyFailure.value = '사진·구입처 이력을 읽지 못했습니다.' }
    finally { if (active && token === historyGeneration) historyLoading.value = false }
  }

  async function chooseFile(selected: File): Promise<boolean> {
    if (busy.value || reviewing.value || restoreFailure.value || (pending.value && !canRetry.value)) return false
    if (!pending.value && !input.value.photos.some(photo => photo.source === 'upload') && input.value.photos.length >= 20) { errors.value.file = '사진은 20장까지 연결할 수 있습니다.'; return false }
    const token = ++hashGeneration.value, requestId = pending.value?.body.meta.request_id ?? null
    hashing.value = true; errors.value.file = ''
    try {
      const descriptor = await describeProfileFile(selected)
      if (token !== hashGeneration.value || requestId !== (pending.value?.body.meta.request_id ?? null)) return false
      if (pending.value?.file && !sameProfileFile(pending.value.file, descriptor)) { errors.value.file = '원래 요청에 사용한 사진과 다릅니다. 같은 사진을 다시 선택해 주세요.'; return false }
      if (!pending.value && needsFile.value && fileDescriptor.value && !sameProfileFile(fileDescriptor.value, descriptor)) { errors.value.file = '보관된 입력의 사진과 다릅니다. 새 사진을 쓰려면 기존 새 사진의 연결을 먼저 해제해 주세요.'; return false }
      file.value = selected; fileDescriptor.value = descriptor
      if (!pending.value && !input.value.photos.some(photo => photo.source === 'upload')) { input.value.photos.push({ source: 'upload', caption: '' }); if (input.value.primary_position === null) input.value.primary_position = 0 }
      persist(); return true
    } catch (cause) { if (token === hashGeneration.value) errors.value.file = cause instanceof Error ? cause.message : '사진을 읽지 못했습니다.'; return false }
    finally { if (token === hashGeneration.value) hashing.value = false }
  }
  function removePhoto(index: number) {
    if (locked.value || index < 0 || index >= input.value.photos.length) return
    if (input.value.photos[index]?.source === 'upload') { file.value = null; fileDescriptor.value = null; hashGeneration.value++ }
    input.value = removeProfilePhoto(input.value, index)
    errors.value = {}; notice.value = '현재 목록에서 연결을 해제했습니다. 저장 전이며 이전 기록의 사진은 남습니다.'
  }

  async function send(): Promise<boolean> {
    if (busy.value || reviewing.value || hashing.value || restoreFailure.value || connection.value !== 'connected' || !basis.value || (pending.value && !canRetry.value)) return false
    let request = pending.value
    if (!request) {
      const data = normalizeProfile(input.value)
      errors.value = validateProfile(data)
      for (const [index, photo] of data.photos.entries()) if (photo.source === 'existing' && !current.value?.version?.photos.some(item => item.id === photo.photo_id)) errors.value[`photo-${index}`] = '현재 저장된 목록에서 제외된 사진입니다. 최신 사진과 비교한 뒤 이 연결을 해제해 주세요.'
      const upload = data.photos.some(photo => photo.source === 'upload')
      if (upload && (!file.value || !fileDescriptor.value)) errors.value.file = '선택했던 사진을 다시 선택해 주세요.'
      if (Object.values(errors.value).some(Boolean)) return false
      request = { material_id: materialId, body: { ...data, meta: { request_id: crypto.randomUUID(), epoch: basis.value.state.epoch, expected_revision: basis.value.state.revision }, expected_material_version_id: basis.value.material_version_id, expected_profile_version_id: basis.value.profile_version_id }, file: upload ? { ...fileDescriptor.value! } : null }
      if (new TextEncoder().encode(JSON.stringify(request.body)).length > 256 * 1024) { errors.value.form = '사진·구입처 입력 전체가 256KiB를 넘습니다. 설명이나 항목 수를 줄여 주세요.'; return false }
    }
    if (request.file && !file.value) { errors.value.file = '원래 요청에 사용한 사진을 다시 선택한 뒤 재시도해 주세요.'; return false }
    try { sessionStorage.setItem(pendingKey, JSON.stringify(request)) }
    catch { storageFailure.value = '요청을 이 탭에 보관하지 못해 전송하지 않았습니다. 입력과 사진을 유지했습니다.'; return false }
    pending.value = request; busy.value = true; failure.value = ''; notice.value = ''; errors.value = {}
    try {
      const body = new FormData()
      body.append('metadata', JSON.stringify(request.body))
      if (request.file) body.append('photo', file.value!)
      const result = await apiRequest<ProfileDetail>(`/api/materials/${materialId}/profiles`, body)
      const latest = current.value
      const newer = latest && (latest.state.epoch !== result.state.epoch || BigInt(latest.state.revision) > BigInt(result.state.revision))
      current.value = newer ? latest : result
      remember(result); adopt(current.value)
      try { sessionStorage.removeItem(pendingKey); sessionStorage.removeItem(draftKey) } catch { storageFailure.value = '저장은 완료했지만 탭의 이전 요청 정리를 확인하지 못했습니다. 남은 요청은 같은 내용으로 확인할 수 있습니다.' }
      pending.value = null; recovery.value = ''; notice.value = '사진·구입처를 저장했습니다.'
      completed.value++; return true
    } catch (cause) {
      recovery.value = cause instanceof ApiError ? (cause.status === 413 ? 'edit' : failureAction(cause.status, cause.code)) : 'retry'
      failure.value = recovery.value === 'refresh' ? '다른 변경으로 자료가 갱신되어 이 요청은 저장하지 않았습니다. 최신 사진·구입처와 비교해 주세요.' : recovery.value === 'blocked' ? '같은 요청 ID에 다른 처리 이력이 있어 재전송할 수 없습니다. 저장 기록을 확인해 주세요.' : recovery.value === 'edit' ? (cause instanceof Error ? cause.message : '사진과 입력을 확인해 주세요.') : '저장 완료를 확인하지 못했습니다. 원래 사진과 입력을 유지했습니다. 같은 요청으로 재시도해 주세요.'
      return false
    } finally { busy.value = false }
  }
  async function review(): Promise<boolean> {
    if (!canReview.value || busy.value || reviewing.value || !pending.value) return false
    const requestId = pending.value.body.meta.request_id
    reviewing.value = true
    try {
      const latest = await load()
      if (!latest || pending.value?.body.meta.request_id !== requestId) return false
      sessionStorage.removeItem(pendingKey)
      baseline.value = profileFromVersion(latest.version)
      basis.value = { state: { ...latest.state }, material_version_id: latest.material.version_id, profile_version_id: latest.version?.id ?? null }
      pending.value = null; recovery.value = ''; failure.value = ''; errors.value = {}
      notice.value = '최신 저장본을 확인했습니다. 작성한 사진·구입처는 유지했으니 비교한 뒤 다시 저장해 주세요.'
      persist(); return true
    } catch { failure.value = '최신 상태나 요청 보관 정보를 갱신하지 못했습니다. 원래 요청을 유지했습니다.'; return false }
    finally { reviewing.value = false }
  }
  watch(connection, () => { void load() }, { immediate: true })
  watch(completed, () => { void load(); if (history.value) void loadHistory() })
  return { input, basis, current, photos, file, fileDescriptor, pending, dirty, locked, busy, reviewing, hashing, loading, readFailure, restoreFailure, failure, notice, storageFailure, errors, canRetry, canReview, needsFile, history, historyLoading, historyFailure, load, loadHistory, chooseFile, removePhoto, send, review }
}
