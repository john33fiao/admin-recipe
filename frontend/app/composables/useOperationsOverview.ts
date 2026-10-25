import type { OperationsOverview } from '~/types/overview'
import { ApiError, apiRequest } from '~/utils/api'

export function useOperationsOverview() {
  const connection = useConnection(), snapshot = shallowRef<{ report: OperationsOverview; receivedAt: string } | null>(null)
  const loading = ref(false), failure = ref('')
  let generation = 0
  onScopeDispose(() => { generation += 1 })
  async function load() {
    const token = ++generation
    loading.value = false
    if (connection.value !== 'connected') return
    loading.value = true; failure.value = ''
    try {
      const report = await apiRequest<OperationsOverview>('/api/overview')
      if (token === generation) snapshot.value = { report, receivedAt: new Date().toISOString() }
    } catch (cause) { if (token === generation) failure.value = cause instanceof ApiError ? cause.message : '작업 시작 자료를 읽지 못했습니다. 다시 조회해 주세요.' }
    finally { if (token === generation) loading.value = false }
  }
  watch(connection, () => { void load() }, { immediate: true, flush: 'sync' })
  const oldResult = computed(() => !!snapshot.value && (loading.value || !!failure.value || connection.value !== 'connected'))
  return { snapshot, loading, failure, connection, oldResult, load }
}
