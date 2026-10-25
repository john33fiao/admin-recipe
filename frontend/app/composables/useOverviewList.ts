import type { CatalogState } from '~/types/material'
import type { OperationsOverview, OverviewListResponse, OverviewTarget } from '~/types/overview'
import { ApiError, apiRequest } from '~/utils/api'
import { overviewListApi, overviewResponseMatches, sameOverviewScope } from '~/utils/overview'

export function useOverviewList(target: Ref<OverviewTarget | null>, initialState: CatalogState | null = null) {
  const connection = useConnection(), snapshot = shallowRef<{ response: OverviewListResponse; receivedAt: string } | null>(null)
  const loading = ref(false), failure = ref(''), stale = ref(false)
  let pinned = initialState ? { ...initialState } : null, generation = 0
  onScopeDispose(() => { generation += 1 })
  async function load(restart = false): Promise<boolean> {
    const token = ++generation, wanted = target.value ? { ...target.value, offset: restart ? '0' : target.value.offset } : null
    loading.value = false
    if (!wanted || connection.value !== 'connected' || (stale.value && !restart)) return false
    loading.value = true; failure.value = ''
    try {
      let basis = pinned
      if (!basis || restart) {
        const summary = await apiRequest<OperationsOverview>('/api/overview')
        if (token !== generation) return false
        basis = { ...summary.state }
      }
      const response = await apiRequest<OverviewListResponse>(overviewListApi(wanted, basis))
      if (token !== generation) return false
      if (!overviewResponseMatches(response, wanted, basis)) throw new Error('mismatched-overview-page')
      pinned = basis; stale.value = false
      snapshot.value = { response, receivedAt: new Date().toISOString() }
      return true
    } catch (cause) {
      if (token !== generation) return false
      stale.value = cause instanceof ApiError && cause.status === 409 && cause.code === 'stale_state'
      failure.value = stale.value ? '기록이 바뀌어 이 목록을 이어 볼 수 없습니다. 전체 자료를 다시 조회하고 첫 페이지에서 시작해 주세요.' : cause instanceof ApiError ? cause.message : '같은 조회의 목록을 확인하지 못했습니다. 다시 조회해 주세요.'
      return false
    } finally { if (token === generation) loading.value = false }
  }
  watch([connection, target], () => { void load() }, { immediate: true, flush: 'sync' })
  const oldResult = computed(() => !!snapshot.value && (loading.value || !!failure.value || connection.value !== 'connected' || !target.value || !sameOverviewScope(snapshot.value.response.scope, target.value) || String(snapshot.value.response.list.page.offset) !== target.value.offset))
  return { snapshot, loading, failure, stale, connection, oldResult, load: () => load(false), restart: () => load(true) }
}
