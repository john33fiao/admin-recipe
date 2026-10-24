<script setup lang="ts">
import type { StocktakeDetail } from '~/types/stocktake'
import { ApiError, apiRequest } from '~/utils/api'
import { formatCreatedAt } from '~/utils/material'
const route = useRoute()
const connection = useConnection()
const detail = ref<StocktakeDetail | null>(null)
const failure = ref('')
const loading = ref(false)
let generation = 0
onScopeDispose(() => { generation += 1 })
async function load() {
  const current = ++generation
  if (connection.value !== 'connected') { loading.value = false; return }
  loading.value = true; failure.value = ''
  try { const value = await apiRequest<StocktakeDetail>(`/api/stocktakes/${encodeURIComponent(String(route.params.id))}`); if (current === generation) detail.value = value }
  catch (error) { if (current === generation) failure.value = error instanceof ApiError && error.status === 404 ? '이 실사 기록을 찾을 수 없습니다.' : '실사 기록을 불러오지 못했습니다. 다시 시도해 주세요.' }
  finally { if (current === generation) loading.value = false }
}
watch(() => route.params.id, () => { detail.value = null })
watch([connection, () => route.params.id], () => { void load() }, { immediate: true })
useHead({ title: computed(() => `${detail.value?.observations[0]?.effect.material_name ?? '실사 상세'} | 순창한상`) })
</script>
<template>
  <NuxtLink class="back-link" to="/stocktakes">‹ 실사 기록</NuxtLink>
  <p v-if="failure" class="notice warning" role="alert">{{ failure }} <button class="text-button" type="button" :disabled="loading || connection !== 'connected'" @click="load">다시 불러오기</button></p>
  <div v-if="!detail && !failure" class="empty-state"><h1>실사 기록을 불러오고 있습니다</h1></div>
  <template v-if="detail"><div class="page-heading"><div><h1>{{ detail.observations[0]?.effect.material_name ?? '실사 기록' }}</h1><p>실사 · 확정</p></div><span class="status-label">실사 확정</span></div>
    <section class="business-record-summary stocktake-detail" aria-labelledby="stocktake-detail-title"><h2 id="stocktake-detail-title">확정한 실사</h2><StocktakeResult :rows="detail.observations.map(value => value.effect)"/><details><summary>실사 관측과 배분 근거</summary><ul class="history-list"><li v-for="row in detail.observations" :key="row.version_id"><p class="material-id">관측 {{ row.id }} · 버전 {{ row.version_id }}</p><p v-if="row.consumption_version_id" class="material-id">소모 배분 {{ row.consumption_version_id }}</p></li></ul></details><dl><div><dt>실사 메모</dt><dd class="preserve-lines">{{ detail.item.note || '없음' }}</dd></div><div><dt>기록 시각</dt><dd>{{ formatCreatedAt(detail.item.recorded_at) }} · 기록 {{ detail.item.seq }}</dd></div><div><dt>실사 ID</dt><dd class="material-id">{{ detail.item.id }}</dd></div></dl></section>
    <NuxtLink class="secondary-link" :to="`/business/${detail.item.business_record_id}`">실사 업무 기록 보기</NuxtLink>
  </template>
</template>
