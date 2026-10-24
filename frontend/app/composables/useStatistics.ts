import type { AppliedStatisticsRange, StatisticsReport } from '~/types/statistics'
import { ApiError, apiRequest } from '~/utils/api'
import { statisticsPath } from '~/utils/statistics'

export function useStatistics(range: Ref<AppliedStatisticsRange | null>) {
  const connection = useConnection()
  const snapshot = shallowRef<{ report: StatisticsReport; range: AppliedStatisticsRange } | null>(null)
  const failure = ref(''), loading = ref(false), loaded = ref(false)
  let generation = 0
  onScopeDispose(() => { generation += 1 })
  async function load() {
    const token = ++generation, applied = range.value ? { ...range.value } : null
    loading.value = false
    if (!applied || connection.value !== 'connected') return
    loading.value = true; failure.value = ''
    try {
      const report = await apiRequest<StatisticsReport>(statisticsPath(applied))
      if (token !== generation) return
      snapshot.value = { report, range: applied }; loaded.value = true
    } catch (cause) {
      if (token !== generation) return
      failure.value = cause instanceof ApiError && cause.code === 'report_too_large' ? '조회 결과가 많습니다. 기간을 좁혀 다시 조회하세요. 이번 조회의 부분 합계는 표시하지 않습니다.' : cause instanceof ApiError ? cause.message : '통계·이력을 읽지 못했습니다. 연결을 확인하고 다시 조회해 주세요.'
    } finally { if (token === generation) loading.value = false }
  }
  watch([connection, range], () => { void load() }, { immediate: true, flush: 'sync' })
  return { snapshot, failure, loading, loaded, load, connection }
}
