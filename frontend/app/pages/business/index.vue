<script setup lang="ts">
import type { BusinessDetail, BusinessList, BusinessPayload } from '~/types/business'
import type { MaterialDetail } from '~/types/material'
import { apiRequest } from '~/utils/api'
import { blankPriceDraft, businessStatuses, copyPayload } from '~/utils/business'
import { formatCreatedAt } from '~/utils/material'

const route = useRoute()
const connection = useConnection()
const filter = ref('draft')
const list = ref<BusinessList | null>(null)
const loading = ref(false)
const failure = ref('')
const seed = ref<BusinessPayload | null>(null)
const seedFailure = ref('')
const corrects = computed(() => typeof route.query.corrects === 'string' ? route.query.corrects : null)
const identity = computed(() => `new.${corrects.value ?? (typeof route.query.material === 'string' ? route.query.material : 'price')}`)
let listGeneration = 0
let seedGeneration = 0
async function load(): Promise<null> {
  const generation = ++listGeneration
  if (connection.value !== 'connected') return null
  loading.value = true
  failure.value = ''
  try { const result = await apiRequest<BusinessList>(`/api/business?status=${filter.value}`); if (generation === listGeneration) list.value = result }
  catch { if (generation === listGeneration) failure.value = '업무 목록을 불러오지 못했습니다. 다시 시도해 주세요.' }
  finally { if (generation === listGeneration) loading.value = false }
  return null
}
async function loadSeed() {
  const generation = ++seedGeneration
  if (connection.value !== 'connected') return
  seed.value = null
  seedFailure.value = ''
  try {
    let payload = blankPriceDraft()
    if (corrects.value) {
      if (typeof route.query.record !== 'string') throw new Error('missing-record')
      const detail = await apiRequest<BusinessDetail>(`/api/business/${encodeURIComponent(route.query.record)}`)
      if (detail.item.confirmation_id !== corrects.value || detail.item.effective_confirmation_id !== corrects.value) throw new Error('not-effective')
      payload = copyPayload(detail.head.payload)
      payload.data.note = ''
    } else if (typeof route.query.material === 'string') {
      const material = await apiRequest<MaterialDetail>(`/api/materials/${encodeURIComponent(route.query.material)}`)
      payload.data = { ...payload.data, material_id: material.id, material_version_id: material.version_id, unit: material.unit }
    }
    if (generation === seedGeneration) seed.value = payload
  } catch { if (generation === seedGeneration) seedFailure.value = '초안의 대상 기록을 확인하지 못했습니다. 최신 재료나 유효한 확정 기록에서 다시 열어 주세요.' }
}
watch([connection, filter], () => { void load() }, { immediate: true })
watch([connection, () => route.query.material, () => route.query.corrects, () => route.query.record], () => { void loadSeed() }, { immediate: true })
useHead({ title: '업무 기록 | 순창한상' })
</script>

<template>
  <div class="page-heading"><div><h1>업무 기록</h1><p>작성 중인 가격을 임시 저장하고, 확인한 내용을 확정하세요.</p></div></div>
  <p v-if="seedFailure" class="notice warning" role="alert">{{ seedFailure }} <NuxtLink to="/business">새 가격 초안 작성</NuxtLink></p>
  <div class="business-workspace">
    <BusinessPriceEditor v-if="seed && list" :key="identity" :identity="identity" :initial="{ payload: seed, state: list.state, recordId: null, headId: null, status: 'draft', corrects, issues: [] }" :refresh="load" />
    <section class="business-list-panel" aria-labelledby="business-list-title" :aria-busy="loading">
      <div class="list-heading"><h2 id="business-list-title">저장된 업무</h2><button class="text-button" type="button" :disabled="loading || connection !== 'connected'" @click="load">다시 불러오기</button></div>
      <div class="field"><label for="business-filter">업무 상태</label><select id="business-filter" v-model="filter" :disabled="connection !== 'connected'"><option value="draft">임시 저장</option><option value="confirmed">확정</option><option value="archived">보관</option><option value="all">전체</option></select></div>
      <p v-if="failure" class="notice warning" role="alert">{{ failure }}</p>
      <div v-if="!list" class="empty-state"><h3>업무 기록을 불러오고 있습니다</h3><p>앱에 연결되면 저장된 초안을 확인할 수 있습니다.</p></div>
      <div v-else-if="!list.items.length" class="empty-state"><h3>이 상태의 업무가 없습니다</h3><p>가격 초안을 임시 저장하면 여기에서 이어서 작성할 수 있습니다.</p></div>
      <ul v-else class="business-list"><li v-for="item in list.items" :key="item.id"><NuxtLink :to="`/business/${item.id}`"><div><strong>{{ item.material_name ?? '재료 확인 필요' }}</strong><span class="status-label">{{ businessStatuses[item.status] }}</span></div><p>{{ item.corrects_confirmation_id ? '가격 정정' : '사전 가격' }} · {{ item.id.slice(0, 8) }}</p><p v-if="item.issue_count" class="business-needs-input">입력 필요 {{ item.issue_count }}개</p><time>{{ formatCreatedAt(item.updated_at) }} 수정</time></NuxtLink></li></ul>
    </section>
  </div>
</template>
