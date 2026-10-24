import type { Material } from '~/types/material'
import { apiRequest } from '~/utils/api'

export function useMaterials() {
  const connection = useConnection()
  const materials = useState<Material[]>('materials', () => [])
  const loaded = useState('materials-loaded', () => false)
  const loading = useState('materials-loading', () => false)
  const loadFailure = useState('materials-load-failure', () => '')
  const available = computed(() => connection.value === 'connected')

  async function loadMaterials() {
    if (!available.value || loading.value) return
    loading.value = true
    loadFailure.value = ''
    try {
      materials.value = await apiRequest<Material[]>('/api/materials')
      loaded.value = true
    } catch {
      loadFailure.value = '재료 목록을 불러오지 못했습니다. 연결을 확인한 뒤 다시 불러와 주세요.'
    } finally {
      loading.value = false
    }
  }

  watch(available, connected => { if (connected) void loadMaterials() }, { immediate: true })
  return { materials, loaded, loading, loadFailure, available, loadMaterials }
}
