<script setup lang="ts">
import type { WasteDetail } from '~/types/waste'
import { ApiError, apiRequest } from '~/utils/api'
import { formatCreatedAt } from '~/utils/material'
const route = useRoute()
const connection = useConnection()
const detail = ref<WasteDetail | null>(null)
const failure = ref('')
const loading = ref(false)
let generation = 0
onScopeDispose(() => { generation += 1 })
async function load() {
  const current = ++generation
  if (connection.value !== 'connected') { loading.value = false; return }
  loading.value = true; failure.value = ''
  try { const value = await apiRequest<WasteDetail>(`/api/wastes/${encodeURIComponent(String(route.params.id))}`); if (current === generation) detail.value = value }
  catch (error) { if (current === generation) failure.value = error instanceof ApiError && error.status === 404 ? '이 폐기 · 직원식 기록을 찾을 수 없습니다.' : '폐기 · 직원식 기록을 불러오지 못했습니다. 다시 시도해 주세요.' }
  finally { if (current === generation) loading.value = false }
}
watch(() => route.params.id, () => { detail.value = null })
watch([connection, () => route.params.id], () => { void load() }, { immediate: true })
useHead({ title: computed(() => `${detail.value?.item.effect.material_name ?? '폐기 · 직원식 상세'} | 순창한상`) })
</script>
<template>
  <NuxtLink class="back-link" to="/wastes">‹ 폐기 · 직원식 기록</NuxtLink>
  <p v-if="failure" class="notice warning" role="alert">{{ failure }} <button class="text-button" type="button" :disabled="loading || connection !== 'connected'" @click="load">다시 불러오기</button></p>
  <div v-if="!detail && !failure" class="empty-state"><h1>폐기 · 직원식 기록을 불러오고 있습니다</h1></div>
  <template v-if="detail"><div class="page-heading"><div><h1>{{ detail.item.effect.material_name ?? '폐기 · 직원식 기록' }}</h1><p>실제 폐기 · 직원식 · 확정</p></div><span class="status-label">폐기 · 직원식 확정</span></div>
    <section class="business-record-summary waste-detail" aria-labelledby="waste-detail-title"><h2 id="waste-detail-title">확정한 실제 폐기 · 직원식</h2><WasteResult :value="detail.item.effect"/><dl><div><dt>폐기 · 직원식 메모</dt><dd class="preserve-lines">{{ detail.item.note || '없음' }}</dd></div><div><dt>기록 시각</dt><dd>{{ formatCreatedAt(detail.item.recorded_at) }} · 기록 {{ detail.item.seq }}</dd></div><div><dt>폐기 · 직원식 ID</dt><dd class="material-id">{{ detail.item.id }}</dd></div></dl></section>
    <NuxtLink class="secondary-link" :to="`/business/${detail.item.business_record_id}`">폐기 · 직원식 업무 기록 보기</NuxtLink>
  </template>
</template>
