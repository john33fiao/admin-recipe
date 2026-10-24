<script setup lang="ts">
import type { CatalogState, MaterialDetail } from '~/types/material'
import type { RecipeEditorInitial } from '~/composables/useRecipeEditor'
import { apiRequest } from '~/utils/api'
import { blankRecipe } from '~/utils/recipe'
const route = useRoute(), connection = useConnection()
const initial = ref<RecipeEditorInitial | null>(null), failure = ref('')
const identity = computed(() => `new.${typeof route.query.material === 'string' ? route.query.material : 'item'}`)
let generation = 0
onScopeDispose(() => { generation += 1 })
async function load() {
  const token = ++generation
  if (connection.value !== 'connected') return null
  try {
    const material = typeof route.query.material === 'string' ? await apiRequest<MaterialDetail>(`/api/materials/${encodeURIComponent(route.query.material)}`) : null
    const state = material ? { epoch: material.epoch, revision: material.revision } : await apiRequest<CatalogState>('/api/state')
    if (token === generation) { initial.value = { materialId: null, owner: material ? { mode: 'existing', material_id: material.id, expected_material_version_id: material.version_id } : { mode: 'new', name: '', unit: '개', kind: 'menu', description: '' }, data: blankRecipe(), head: null, state }; failure.value = '' }
  } catch { if (token === generation) failure.value = '새 레시피의 기준 자료를 읽지 못했습니다. 연결을 확인해 주세요.' }
  return null
}
watch([connection, identity], () => { void load() }, { immediate: true })
useHead({ title: '새 레시피 | 순창한상' })
</script>
<template>
  <NuxtLink class="back-link" to="/recipes">‹ 레시피 목록</NuxtLink><div class="page-heading"><div><h1>새 레시피</h1><p>새 품목을 만들거나 기존 품목에 배합을 연결합니다.</p></div></div>
  <p v-if="failure" class="notice warning" role="alert">{{ failure }} <button type="button" class="text-button" @click="load">다시 불러오기</button></p>
  <RecipeEditor v-if="initial" :key="identity" :identity="identity" :initial="initial" :refresh="load" />
</template>
