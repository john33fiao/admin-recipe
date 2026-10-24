<script setup lang="ts">
import type { PurchaseReturnDetail } from '~/types/purchase-return'
import { ApiError, apiRequest } from '~/utils/api'
import { effectivePurchaseReturn } from '~/utils/purchase-return'
import { formatCreatedAt } from '~/utils/material'
const route = useRoute(), connection = useConnection()
const detail = ref<PurchaseReturnDetail | null>(null), failure = ref(''), loading = ref(false)
const current = computed(() => detail.value ? effectivePurchaseReturn(detail.value) : null)
let generation = 0
onScopeDispose(() => { generation += 1 })
async function load() {
  const request = ++generation
  if (connection.value !== 'connected') { loading.value = false; return }
  loading.value = true; failure.value = ''
  try { const value = await apiRequest<PurchaseReturnDetail>(`/api/purchase-returns/${encodeURIComponent(String(route.params.id))}`); if (request === generation) detail.value = value }
  catch (error) { if (request === generation) failure.value = error instanceof ApiError && error.status === 404 ? '이 구입 반품 기록을 찾을 수 없습니다.' : '반품 기록을 불러오지 못했습니다. 다시 시도해 주세요.' }
  finally { if (request === generation) loading.value = false }
}
watch(() => route.params.id, () => { detail.value = null })
watch([connection, () => route.params.id], () => { void load() }, { immediate: true })
useHead({ title: computed(() => `${current.value?.effect.material_name ?? detail.value?.item.effect.material_name ?? '구입 반품 상세'} | 순창한상`) })
</script>
<template>
  <NuxtLink class="back-link" to="/purchase-returns">‹ 구입 반품 기록</NuxtLink><p v-if="failure" class="notice warning" role="alert">{{ failure }} <button class="text-button" type="button" :disabled="loading || connection !== 'connected'" @click="load">다시 불러오기</button></p><div v-if="!detail && !failure" class="empty-state"><h1>구입 반품 기록을 불러오고 있습니다</h1></div>
  <template v-if="detail"><div class="page-heading"><div><h1>{{ current?.effect.material_name ?? detail.item.effect.material_name }}</h1><p>구입 반품 · {{ current ? '현재 유효' : '취소됨' }}</p></div><span class="status-label">{{ current ? '반품 확정' : '유효 효과 취소' }}</span></div>
    <section class="business-record-summary purchase-return-detail" aria-label="현재 유효한 구입 반품"><h2>현재 유효한 구입 반품</h2><template v-if="current"><PurchaseReturnResult :value="current.effect"/><p class="preserve-lines">반품 메모: {{ current.note || '없음' }}</p></template><p v-else class="notice warning">이 반품의 유효 효과가 취소되었습니다. 수량 0의 반품으로 바꾸지 않으며 원기록과 정정 이력은 보존합니다.</p><dl class="return-fields"><div><dt>최초 기록 시각</dt><dd>{{ formatCreatedAt(detail.item.recorded_at) }} · 기록 {{ detail.item.seq }}</dd></div><div><dt>반품 ID</dt><dd class="material-id">{{ detail.item.id }}</dd></div></dl></section>
    <details v-if="detail.effective" class="history-section"><summary>보존된 최초 반품 기록</summary><PurchaseReturnResult :value="detail.item.effect"/><p class="preserve-lines">{{ detail.item.note || '메모 없음' }}</p></details>
    <p class="button-row"><NuxtLink class="secondary-link" :to="`/business/${detail.item.business_record_id}`">반품 업무 기록 보기</NuxtLink><NuxtLink class="secondary-link" :to="{ path: '/business', query: { kind: 'purchase_return', receipt_id: current?.effect.source.reference.id ?? detail.item.effect.source.reference.id } }">같은 원입고의 새 반품 초안</NuxtLink></p><CorrectionHistoryPanel :confirmation-id="detail.item.confirmation_id"/>
  </template>
</template>
