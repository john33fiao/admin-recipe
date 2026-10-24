<script setup lang="ts">
import type { Material } from '~/types/material'
import { ApiError, apiRequest } from '~/utils/api'
import { formatCreatedAt } from '~/utils/material'

const route = useRoute()
const connection = useConnection()
const material = ref<Material | null>(null)
const loading = ref(false)
const error = ref('')
const missing = ref(false)
let requestVersion = 0

async function load() {
  const version = ++requestVersion
  if (connection.value !== 'connected') return
  loading.value = true
  error.value = ''
  missing.value = false
  try {
    const result = await apiRequest<Material>(`/api/materials/${encodeURIComponent(String(route.params.id))}`)
    if (version === requestVersion) material.value = result
  } catch (cause) {
    if (version !== requestVersion) return
    missing.value = cause instanceof ApiError && cause.status === 404
    error.value = missing.value ? '이 재료를 찾을 수 없습니다.' : '재료를 불러오지 못했습니다. 연결을 확인한 뒤 다시 시도해 주세요.'
  } finally {
    if (version === requestVersion) loading.value = false
  }
}
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
        <div><dt>관리 단위</dt><dd><span class="unit-tag">{{ material.unit }}</span></dd></div>
        <div><dt>등록 시각</dt><dd>{{ formatCreatedAt(material.created_at) }}</dd></div>
        <div><dt>재료 ID</dt><dd class="material-id">{{ material.id }}</dd></div>
      </dl>
    </template>
    <div v-else-if="!error" class="empty-state"><h1>{{ loading ? '재료를 불러오고 있습니다' : '연결을 기다리고 있습니다' }}</h1><p>앱에 연결되면 재료 정보를 확인할 수 있습니다.</p></div>
    <div v-if="error" class="notice warning" role="alert"><p>{{ error }}</p><button v-if="!missing" class="text-button" type="button" :disabled="connection !== 'connected' || loading" @click="load">다시 불러오기</button><NuxtLink v-else to="/">재료 목록에서 확인</NuxtLink></div>
  </section>
</template>
