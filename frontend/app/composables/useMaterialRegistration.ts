import type { Material, PendingMaterial } from '~/types/material'
import { ApiError, apiRequest } from '~/utils/api'
import { prepareRequest, restorePending } from '~/utils/material'

const storageKey = 'sunchang.pending-material.v1'

export function useMaterialRegistration() {
  const connection = useConnection()
  const pending = useState<PendingMaterial | null>('pending-material', () => {
    try { return restorePending(sessionStorage.getItem(storageKey)) } catch { return null }
  })
  const name = useState('material-name', () => pending.value?.name ?? '')
  const unit = useState('material-unit', () => pending.value?.unit ?? 'g')
  const saving = useState('material-saving', () => false)
  const failure = useState('material-failure', () => '')
  const rejected = useState('material-rejected', () => false)
  const saved = useState<Material | null>('material-saved', () => null)

  async function save(): Promise<Material | null> {
    if (saving.value || connection.value !== 'connected') return null
    saving.value = true
    failure.value = ''
    saved.value = null
    rejected.value = false
    let request: PendingMaterial
    try {
      request = prepareRequest({ name: name.value, unit: unit.value }, pending.value, () => crypto.randomUUID())
      // Persist before sending so refresh cannot turn an uncertain result into a new write.
      sessionStorage.setItem(storageKey, JSON.stringify(request))
      pending.value = request
      name.value = request.name
      unit.value = request.unit
    } catch {
      failure.value = '입력 내용을 임시 보관하지 못해 등록을 보내지 않았습니다. 브라우저의 사이트 저장공간 허용 여부를 확인해 주세요.'
      saving.value = false
      return null
    }
    try {
      const material = await apiRequest<Material>('/api/materials', request)
      // A success response means the server has completed both data and change-log writes.
      saved.value = material
      pending.value = null
      name.value = ''
      unit.value = 'g'
      try { sessionStorage.removeItem(storageKey) } catch { /* Replaying the same key remains safe. */ }
      return material
    } catch (error) {
      if (error instanceof ApiError && [400, 409].includes(error.status)) {
        rejected.value = true
        failure.value = error.status === 409
          ? '이 등록 요청을 다른 내용으로 보낼 수 없습니다. 새 입력을 시작해 주세요.'
          : '등록 요청이 거부되었습니다. 입력 내용을 수정해 주세요.'
      } else {
        failure.value = '등록 완료를 확인하지 못했습니다. 자료가 저장되었더라도 변경 기록이 아직 남지 않았을 수 있습니다. 입력 내용을 유지했으니 연결을 확인한 뒤 같은 요청을 재시도해 주세요.'
      }
      return null
    } finally {
      saving.value = false
    }
  }

  function editRejected() {
    if (!rejected.value || saving.value) return
    try { sessionStorage.removeItem(storageKey) } catch { return }
    pending.value = null
    rejected.value = false
    failure.value = ''
  }

  return { name, unit, pending, saving, failure, saved, rejected, save, editRejected }
}
