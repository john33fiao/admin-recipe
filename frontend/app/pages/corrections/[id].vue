<script setup lang="ts">
import type { CorrectionDetail } from '~/types/correction'
import { apiRequest } from '~/utils/api'
import { formatCreatedAt } from '~/utils/material'
const route = useRoute()
const connection = useConnection()
const detail = ref<CorrectionDetail | null>(null)
const failure = ref('')
const loading = ref(false)
let generation = 0
async function load() {
  const current = ++generation
  if (connection.value !== 'connected') return
  loading.value = true
  try { const result = await apiRequest<CorrectionDetail>(`/api/corrections/${encodeURIComponent(String(route.params.id))}`); if (current === generation) { detail.value = result; failure.value = '' } }
  catch { if (current === generation) failure.value = '정정 기록을 불러오지 못했습니다.' }
  finally { if (current === generation) loading.value = false }
}
watch([connection, () => route.params.id], () => { detail.value = null; void load() }, { immediate: true })
useHead({ title: '정정 기록 | 순창한상' })
</script>
<template>
  <NuxtLink class="back-link" to="/business">‹ 업무 기록</NuxtLink><p v-if="failure" class="notice warning" role="alert">{{ failure }} <button class="text-button" type="button" :disabled="loading || connection !== 'connected'" @click="load">다시 불러오기</button></p><p v-if="!detail && !failure">정정 기록을 불러오고 있습니다.</p>
  <template v-if="detail"><div class="page-heading"><div><h1>정정 확정 기록</h1><p>{{ formatCreatedAt(detail.item.recorded_at) }} · 실제 기록 {{ detail.item.seq }} · 원래 계산 위치 {{ detail.item.plan.target.logical_seq }}</p></div><span class="status-label">정정 확정</span></div><section class="business-record-summary"><h2>정정 이유</h2><p class="preserve-lines">{{ detail.item.reason }}</p><div class="button-row"><NuxtLink :to="`/business/${detail.item.business_record_id}`">이 정정 업무 보기</NuxtLink><NuxtLink :to="`/business/${detail.item.plan.target.business_record_id}`">대상 업무 기록 보기</NuxtLink></div></section><CorrectionPlanPanel :plan="detail.item.plan"/><details><summary>확정 후 생성된 참조</summary><ul class="history-list"><li v-for="entry in detail.item.resolved_refs" :key="`${entry.reference.entity}-${entry.reference.id}`"><p class="material-id">대상 {{ entry.reference.id }} · 확정 버전 {{ entry.version_id }}</p></li></ul></details><CorrectionHistoryPanel :confirmation-id="detail.item.target_confirmation_id"/><section class="history-section"><h2>보존된 정정 이력</h2><ol class="history-list"><li v-for="version in detail.versions" :key="version.id"><NuxtLink :to="`/corrections/${version.id}`">{{ formatCreatedAt(version.recorded_at) }} · 기록 {{ version.seq }}</NuxtLink><p class="preserve-lines">{{ version.reason }}</p></li></ol></section></template>
</template>
