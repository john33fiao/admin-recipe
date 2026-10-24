<script setup lang="ts">
import type { ProductionDetail } from '~/types/production'
import { ApiError, apiRequest } from '~/utils/api'
import { formatCreatedAt } from '~/utils/material'
const route = useRoute()
const connection = useConnection()
const detail = ref<ProductionDetail | null>(null)
const failure = ref('')
const loading = ref(false)
let generation = 0
onScopeDispose(() => { generation += 1 })
async function load() {
  const current = ++generation
  if (connection.value !== 'connected') { loading.value = false; return }
  loading.value = true; failure.value = ''
  try { const value = await apiRequest<ProductionDetail>(`/api/productions/${encodeURIComponent(String(route.params.id))}`); if (current === generation) detail.value = value }
  catch (error) { if (current === generation) failure.value = error instanceof ApiError && error.status === 404 ? '이 생산 기록을 찾을 수 없습니다.' : '생산 기록을 불러오지 못했습니다. 다시 시도해 주세요.' }
  finally { if (current === generation) loading.value = false }
}
watch(() => route.params.id, () => { detail.value = null })
watch([connection, () => route.params.id], () => { void load() }, { immediate: true })
useHead({ title: computed(() => `${detail.value?.item.outputs[0]?.material_name ?? '생산 상세'} | 순창한상`) })
</script>
<template>
  <NuxtLink class="back-link" to="/productions">‹ 생산 기록</NuxtLink>
  <p v-if="failure" class="notice warning" role="alert">{{ failure }} <button class="text-button" type="button" :disabled="loading || connection !== 'connected'" @click="load">다시 불러오기</button></p>
  <div v-if="!detail && !failure" class="empty-state"><h1>생산 기록을 불러오고 있습니다</h1></div>
  <template v-if="detail"><div class="page-heading"><div><h1>{{ detail.item.outputs[0]?.material_name ?? '생산 기록' }}</h1><p>실제 생산 · 확정</p></div><span class="status-label">생산 확정</span></div>
    <section class="business-record-summary production-detail" aria-labelledby="production-detail-title"><h2 id="production-detail-title">확정한 실제 생산</h2><ProductionResult :inputs="detail.item.inputs" :outputs="detail.item.outputs" :materials="detail.materials" :total="detail.item.total_input_value"/><dl><div><dt>생산 메모</dt><dd class="preserve-lines">{{ detail.item.note || '없음' }}</dd></div><div><dt>기록 시각</dt><dd>{{ formatCreatedAt(detail.item.recorded_at) }} · 기록 {{ detail.item.seq }}</dd></div><div><dt>생산 ID</dt><dd class="material-id">{{ detail.item.id }}</dd></div></dl></section>
    <NuxtLink class="secondary-link" :to="`/business/${detail.item.business_record_id}`">생산 업무 기록 보기</NuxtLink>
  </template>
</template>
