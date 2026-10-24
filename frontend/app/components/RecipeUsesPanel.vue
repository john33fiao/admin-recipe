<script setup lang="ts">
import type { RecipeDetail, RecipeList } from '~/types/recipe'
import { ApiError, apiRequest } from '~/utils/api'
const props = defineProps<{ materialId: string; revision: string }>()
const connection = useConnection()
const items = ref<RecipeList['items']>([]), ownRecipe = ref<boolean | null>(null), failure = ref('')
let generation = 0
onScopeDispose(() => { generation += 1 })
async function load() {
  const token = ++generation, id = props.materialId
  if (connection.value !== 'connected') return
  try {
    const result = await apiRequest<RecipeList>(`/api/materials/${encodeURIComponent(id)}/recipe-uses`)
    if (token !== generation) return
    items.value = result.items; failure.value = ''
    try { await apiRequest<RecipeDetail>(`/api/recipes/${encodeURIComponent(id)}`); if (token === generation) ownRecipe.value = true }
    catch (cause) { if (token === generation) ownRecipe.value = cause instanceof ApiError && cause.status === 404 ? false : null }
  } catch { if (token === generation) failure.value = '사용 레시피를 읽지 못했습니다.' }
}
watch([connection, () => props.materialId, () => props.revision], () => { void load() }, { immediate: true })
</script>
<template>
  <section class="history-section"><h2>이 재료를 사용하는 레시피</h2><p class="field-hint">현재 배합에서 직접 참조하는 레시피입니다. 수량 0인 구성도 포함합니다.</p><p v-if="failure" class="notice warning" role="alert">{{ failure }} <button type="button" class="text-button" @click="load">다시 불러오기</button></p><ul v-if="items.length" class="history-list"><li v-for="entry in items" :key="entry.item.id"><NuxtLink :to="`/recipes/${entry.item.id}`">{{ entry.item.name }}</NuxtLink><span v-if="entry.item.archived" class="status-label">보관됨</span></li></ul><p v-else-if="!failure" class="stale-note">현재 직접 사용하는 레시피가 없습니다.</p><p class="business-entry"><NuxtLink v-if="ownRecipe" :to="`/recipes/${materialId}`">이 품목의 레시피 보기</NuxtLink><NuxtLink v-else-if="ownRecipe === false" :to="{ path: '/recipes/new', query: { material: materialId } }">이 품목에 레시피 연결</NuxtLink></p></section>
</template>
