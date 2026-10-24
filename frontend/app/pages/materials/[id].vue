<script setup lang="ts">
import type { MaterialDetail } from '~/types/material'
import { ApiError, apiRequest } from '~/utils/api'
import { formatCreatedAt } from '~/utils/material'
import { displayDecimal, materialKinds, priceSources, priceText } from '~/utils/catalog'

const route = useRoute()
const connection = useConnection()
const material = ref<MaterialDetail | null>(null)
const loading = ref(false)
const error = ref('')
const missing = ref(false)
let requestVersion = 0

async function load(): Promise<MaterialDetail | null> {
  const version = ++requestVersion
  if (connection.value !== 'connected') return null
  loading.value = true
  error.value = ''
  missing.value = false
  try {
    const result = await apiRequest<MaterialDetail>(`/api/materials/${encodeURIComponent(String(route.params.id))}`)
    if (version !== requestVersion) return null
    material.value = result
    return result
  } catch (cause) {
    if (version !== requestVersion) return null
    missing.value = cause instanceof ApiError && cause.status === 404
    error.value = missing.value ? '이 재료를 찾을 수 없습니다.' : '재료를 불러오지 못했습니다. 연결을 확인한 뒤 다시 시도해 주세요.'
    return null
  } finally {
    if (version === requestVersion) loading.value = false
  }
}
watch(() => route.params.id, () => { material.value = null })
watch([connection, () => route.params.id], () => { void load() }, { immediate: true })
useHead({ title: computed(() => `${material.value?.name ?? '재료 상세'} | 순창한상`) })
</script>

<template>
  <NuxtLink class="back-link" to="/"><span aria-hidden="true">‹</span> 재료 목록</NuxtLink>
  <section class="detail-panel" :aria-busy="loading">
    <template v-if="material">
      <div class="detail-heading"><span class="material-initial detail-initial" aria-hidden="true">{{ [...material.name][0] }}</span><div><p class="detail-kicker">재료 상세</p><h1>{{ material.name }}</h1></div></div>
      <p v-if="connection !== 'connected'" class="stale-note">마지막으로 확인한 정보입니다. 연결되면 다시 불러옵니다.</p>
      <dl class="detail-fields">
        <div><dt>종류</dt><dd>{{ materialKinds[material.kind] }} <span v-if="material.archived" class="status-label">보관됨</span></dd></div>
        <div><dt>관리 단위</dt><dd><span class="unit-tag">{{ material.unit }}</span></dd></div>
        <div><dt>{{ priceSources[material.price.source] }}</dt><dd>{{ priceText(material.price, material.canonical_unit) }}<p v-if="material.price.source === 'missing'" class="field-hint">가격을 등록하면 입고 전에도 원가 계산의 기준으로 사용할 수 있습니다.</p></dd></div>
        <div v-if="material.description"><dt>설명</dt><dd class="preserve-lines">{{ material.description }}</dd></div>
        <div><dt>등록 시각</dt><dd>{{ formatCreatedAt(material.created_at) }}</dd></div>
        <div><dt>재료 ID</dt><dd class="material-id">{{ material.id }}</dd></div>
      </dl>
    </template>
    <div v-else-if="!error" class="empty-state"><h1>{{ loading ? '재료를 불러오고 있습니다' : '연결을 기다리고 있습니다' }}</h1><p>앱에 연결되면 재료 정보를 확인할 수 있습니다.</p></div>
    <div v-if="error" class="notice warning" role="alert"><p>{{ error }}</p><button v-if="!missing" class="text-button" type="button" :disabled="connection !== 'connected' || loading" @click="load">다시 불러오기</button><NuxtLink v-else to="/">재료 목록에서 확인</NuxtLink></div>
  </section>
  <template v-if="material">
    <HoldingPanel :material-id="material.id" :revision="material.revision" />
    <MaterialEditor :key="material.id" :material="material" :refresh="load" />
    <p class="business-entry"><NuxtLink class="secondary-link" :to="{ path: '/business', query: { material: material.id } }">가격 초안 작성</NuxtLink><span>가격 입력을 임시 저장하고 나중에 확인한 뒤 확정할 수 있습니다.</span></p>
    <section class="history-section" aria-labelledby="price-history-title">
      <h2 id="price-history-title">가격 변경 이력</h2>
      <p v-if="!material.prices.length" class="stale-note">등록한 가격이 없습니다. 0원과 가격 없음은 구분합니다.</p>
      <ol v-else class="history-list"><li v-for="entry in [...material.prices].reverse()" :key="entry.id"><div><strong>{{ displayDecimal(entry.amount) }}원 / {{ displayDecimal(entry.input_quantity) }}{{ entry.input_unit }}</strong><span>{{ formatCreatedAt(entry.recorded_at) }}</span></div><p>기준 {{ displayDecimal(entry.quantity) }}{{ entry.unit }} · 품목 버전 {{ entry.material_version_id.slice(0, 8) }}</p><p class="material-id">기록 {{ entry.seq }} · {{ entry.id }}</p></li></ol>
    </section>
    <section class="history-section" aria-labelledby="material-history-title">
      <h2 id="material-history-title">정보 변경 이력</h2>
      <ol class="history-list"><li v-for="entry in [...material.versions].reverse()" :key="entry.id"><div><strong>{{ entry.name }}</strong><span>{{ formatCreatedAt(entry.recorded_at) }}</span></div><p>{{ materialKinds[entry.kind] }} · {{ entry.unit }} · {{ entry.archived ? '보관됨' : '사용 중' }}</p><p v-if="entry.description" class="preserve-lines">{{ entry.description }}</p><p class="material-id">{{ entry.seq ? `기록 ${entry.seq}` : '이전 자료에서 이어진 최초 버전' }} · {{ entry.id }}</p></li></ol>
    </section>
  </template>
</template>
