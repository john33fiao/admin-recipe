import type { Material } from '~/types/material'
import { apiRequest } from '~/utils/api'

// Filtered results belong to this page, never to the overview's active total.
export function useMaterialList() {
  const connection = useConnection()
  const query = ref('')
  const archive = ref<'active' | 'archived' | 'all'>('active')
  const materials = ref<Material[]>([])
  const loaded = ref(false)
  const loading = ref(false)
  const loadFailure = ref('')
  const available = computed(() => connection.value === 'connected')
  let requestVersion = 0
  async function loadMaterials() {
    const version = ++requestVersion
    if (!available.value) { loading.value = false; return }
    loading.value = true
    loadFailure.value = ''
    const search = query.value.trim()
    const suffix = search || archive.value !== 'active' ? `?${new URLSearchParams({ query: search, archive: archive.value })}` : ''
    try {
      const result = await apiRequest<Material[]>(`/api/materials${suffix}`)
      if (version !== requestVersion) return
      materials.value = result
      loaded.value = true
    } catch {
      if (version === requestVersion) loadFailure.value = '재료 목록을 불러오지 못했습니다. 연결을 확인한 뒤 다시 불러와 주세요.'
    } finally { if (version === requestVersion) loading.value = false }
  }
  watch(available, () => { void loadMaterials() }, { immediate: true })
  return { query, archive, materials, loaded, loading, loadFailure, available, loadMaterials }
}
