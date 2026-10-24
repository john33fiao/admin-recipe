<script setup lang="ts">
import type { PurchaseReturnList } from '~/types/purchase-return'
import type { Material } from '~/types/material'
import type { ReceiptList } from '~/types/receipt'
import { apiRequest } from '~/utils/api'
import { displayDecimal } from '~/utils/catalog'
import { effectivePurchaseReturn } from '~/utils/purchase-return'
import { formatCreatedAt } from '~/utils/material'
const route = useRoute(), connection = useConnection()
const material = ref(typeof route.query.material === 'string' ? route.query.material : ''), receipt = ref(typeof route.query.receipt === 'string' ? route.query.receipt : '')
const materials = ref<Material[]>([]), receipts = ref<ReceiptList['items']>([]), list = ref<PurchaseReturnList | null>(null), loading = ref(false), failure = ref('')
let generation = 0
onScopeDispose(() => { generation += 1 })
async function load() {
  const current = ++generation
  if (connection.value !== 'connected') { loading.value = false; return }
  loading.value = true; failure.value = ''
  const query = new URLSearchParams()
  if (material.value) query.set('material_id', material.value)
  if (receipt.value) query.set('receipt_id', receipt.value)
  try {
    const [value, choices, sources] = await Promise.all([apiRequest<PurchaseReturnList>(`/api/purchase-returns?${query}`), apiRequest<Material[]>('/api/materials?archive=all'), apiRequest<ReceiptList>('/api/receipts')])
    if (current === generation) { list.value = value; materials.value = choices; receipts.value = sources.items }
  } catch { if (current === generation) failure.value = '구입 반품 기록을 불러오지 못했습니다. 다시 시도해 주세요.' }
  finally { if (current === generation) loading.value = false }
}
watch(() => route.query.material, value => { material.value = typeof value === 'string' ? value : '' })
watch(() => route.query.receipt, value => { receipt.value = typeof value === 'string' ? value : '' })
watch([connection, material, receipt], () => { list.value = null; void load() }, { immediate: true })
useHead({ title: '구입 반품 기록 | 순창한상' })
</script>
<template>
  <div class="page-heading"><div><h1>구입 반품 기록</h1><p>원입고 참고 근거와 실제 평균 가치 차감 내역을 확인합니다.</p></div><NuxtLink class="primary-link" :to="{ path: '/business', query: { kind: 'purchase_return', ...(receipt ? { receipt_id: receipt } : {}) } }">구입 반품 초안 작성</NuxtLink></div>
  <section class="order-list-panel" aria-labelledby="return-list-title" :aria-busy="loading"><div class="list-heading"><h2 id="return-list-title">확정한 구입 반품</h2><button class="text-button" type="button" :disabled="loading || connection !== 'connected'" @click="load">다시 불러오기</button></div>
    <div class="quantity-fields"><div class="field"><label for="return-material-filter">반품 품목 조회</label><select id="return-material-filter" v-model="material" :disabled="connection !== 'connected'"><option value="">모든 품목</option><option v-for="item in materials" :key="item.id" :value="item.id">{{ item.name }}{{ item.archived ? ' (보관됨)' : '' }}</option></select></div><div class="field"><label for="return-receipt-filter">원입고 조회</label><select id="return-receipt-filter" v-model="receipt" :disabled="connection !== 'connected'"><option value="">모든 원입고</option><option v-for="item in receipts" :key="item.id" :value="item.id">{{ item.material_name }} · {{ item.receipt_date }} · {{ item.id.slice(0, 8) }}</option></select></div></div>
    <p v-if="failure" class="notice warning" role="alert">{{ failure }}</p><div v-else-if="loading && !list" class="empty-state"><h3>반품 기록을 불러오고 있습니다</h3></div><div v-else-if="list && !list.items.length" class="empty-state"><h3>확정한 구입 반품이 없습니다</h3><p>원입고와 실제 반품 수량을 확인하고 초안을 작성하세요.</p></div>
    <ul v-else-if="list" class="business-list"><li v-for="entry in list.items" :key="entry.item.id"><NuxtLink :to="`/purchase-returns/${entry.item.id}`"><div><strong>{{ effectivePurchaseReturn(entry)?.effect.material_name ?? entry.item.effect.material_name }}</strong><span class="status-label">{{ entry.effective?.kind === 'void' ? '취소됨' : '구입 반품 확정' }}</span></div><p v-if="effectivePurchaseReturn(entry)">{{ displayDecimal(effectivePurchaseReturn(entry)!.effect.quantity) }}{{ effectivePurchaseReturn(entry)!.effect.unit }} · 실제 차감 {{ displayDecimal(effectivePurchaseReturn(entry)!.effect.deducted_value) }}원</p><p v-else>유효 효과 취소 · 원기록 보존</p><p>원입고 {{ entry.item.effect.source.reference.id.slice(0, 8) }}</p><time>{{ formatCreatedAt(entry.item.recorded_at) }} 최초 기록 · {{ entry.item.id.slice(0, 8) }}</time></NuxtLink></li></ul>
  </section>
</template>
