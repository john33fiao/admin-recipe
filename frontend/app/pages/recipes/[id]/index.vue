<script setup lang="ts">
import type { RecipeDetail } from '~/types/recipe'
import { ApiError, apiRequest } from '~/utils/api'
import { recipeFromVersion } from '~/utils/recipe'
import { formatCreatedAt } from '~/utils/material'
const route = useRoute(), connection = useConnection()
const detail = ref<RecipeDetail | null>(null), failure = ref(''), loading = ref(false)
let generation = 0
onScopeDispose(() => { generation += 1 })
async function load(): Promise<RecipeDetail | null> {
  const token = ++generation, id = String(route.params.id)
  if (connection.value !== 'connected') return null
  loading.value = true
  try { const result = await apiRequest<RecipeDetail>(`/api/recipes/${encodeURIComponent(id)}`); if (token !== generation) return null; detail.value = result; failure.value = ''; return result }
  catch (cause) { if (token === generation) failure.value = cause instanceof ApiError && cause.status === 404 ? '레시피를 찾을 수 없습니다.' : '레시피를 읽지 못했습니다. 다시 확인해 주세요.'; return null }
  finally { if (token === generation) loading.value = false }
}
watch(() => route.params.id, () => { detail.value = null })
watch([connection, () => route.params.id], () => { void load() }, { immediate: true })
const initial = computed(() => detail.value ? { materialId: detail.value.item.id, owner: { mode: 'existing' as const, material_id: detail.value.item.id, expected_material_version_id: detail.value.item.version_id }, data: recipeFromVersion(detail.value.head), head: detail.value.head, state: detail.value.state } : null)
useHead({ title: computed(() => `${detail.value?.item.name ?? '레시피'} | 순창한상`) })
</script>
<template>
  <NuxtLink class="back-link" to="/recipes">‹ 레시피 목록</NuxtLink>
  <p v-if="failure" class="notice warning" role="alert">{{ failure }} <button type="button" class="text-button" :disabled="loading" @click="load">다시 불러오기</button></p>
  <template v-if="detail && initial"><div class="page-heading"><div><h1>{{ detail.item.name }}</h1><p v-if="detail.item.archived">보관된 품목입니다. 저장된 배합과 이력은 계속 조회할 수 있습니다.</p><p><NuxtLink :to="`/materials/${detail.item.id}`">현재 품목 정보와 사용 레시피</NuxtLink></p></div></div>
    <p v-if="detail.head.material_version_id !== detail.item.version_id" class="stale-note">배합 저장 당시 품목: {{ detail.head.material_name }} · {{ detail.head.material_unit }}. 현재 품목 정보와 구별해 보관합니다.</p>
    <div class="button-row"><NuxtLink class="secondary-link" :to="{ path: '/business', query: { kind: 'sale', material: detail.item.id } }">이 메뉴 판매 초안 작성</NuxtLink><NuxtLink :to="`/sales?material=${detail.item.id}`">이 메뉴의 판매 기록</NuxtLink></div>
    <MaterialProfilePanel :key="`profile-${detail.item.id}`" :material-id="detail.item.id" />
    <RecipeEditor :key="detail.item.id" :identity="detail.item.id" :initial="initial" :refresh="load" />
    <RecipeCostPanel :version="detail.head" />
    <section class="history-section"><h2>레시피 버전 이력</h2><ol class="history-list recipe-history"><li v-for="version in [...detail.versions].reverse()" :key="version.id"><NuxtLink :to="`/recipes/${detail.item.id}/versions/${version.id}`">{{ formatCreatedAt(version.recorded_at) }} · 기준 {{ version.basis_count }}개</NuxtLink><p class="material-id">기록 {{ version.seq }} · {{ version.id }}</p><p v-if="version.copied_from_version_id">독립 복제로 시작한 배합</p></li></ol></section>
  </template>
</template>
