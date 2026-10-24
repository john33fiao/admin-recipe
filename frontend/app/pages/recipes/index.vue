<script setup lang="ts">
import type { RecipeList } from '~/types/recipe'
import { apiRequest } from '~/utils/api'
import { materialKinds } from '~/utils/catalog'
const connection = useConnection()
const query = ref(''), archive = ref('active'), items = ref<RecipeList['items']>([]), loading = ref(false), failure = ref('')
let generation = 0
onScopeDispose(() => { generation += 1 })
async function load() {
  const token = ++generation
  if (connection.value !== 'connected') return
  loading.value = true; failure.value = ''
  try { const result = await apiRequest<RecipeList>(`/api/recipes?query=${encodeURIComponent(query.value)}&archive=${archive.value}`); if (token === generation) items.value = result.items }
  catch { if (token === generation) failure.value = '레시피 목록을 불러오지 못했습니다. 다시 확인해 주세요.' }
  finally { if (token === generation) loading.value = false }
}
watch([connection, query, archive], () => { void load() }, { immediate: true })
useHead({ title: '레시피 | 순창한상' })
</script>
<template>
  <div class="page-heading"><div><h1>레시피</h1><p>구성 배합과 현재 가격으로 계산한 식재료비를 확인합니다.</p></div><NuxtLink class="secondary-link" to="/recipes/new">새 레시피 작성</NuxtLink></div>
  <section :aria-busy="loading"><div class="form-row"><div class="field"><label for="recipe-search">레시피 이름 검색</label><input id="recipe-search" v-model="query" type="search"></div><div class="field"><label for="recipe-archive">레시피 보관 상태</label><select id="recipe-archive" v-model="archive"><option value="active">사용 중</option><option value="all">보관 포함 전체</option><option value="archived">보관됨</option></select></div></div>
    <p v-if="failure" class="notice warning" role="alert">{{ failure }} <button type="button" class="text-button" @click="load">다시 불러오기</button></p>
    <p v-if="connection !== 'connected'" class="stale-note">마지막으로 확인한 목록입니다. 연결되면 다시 불러옵니다.</p>
    <ul v-if="items.length" class="recipe-list history-list"><li v-for="entry in items" :key="entry.item.id"><NuxtLink :to="`/recipes/${entry.item.id}`"><strong>{{ entry.item.name }}</strong></NuxtLink><p>{{ materialKinds[entry.item.kind] }} · {{ entry.item.unit }} · {{ entry.item.archived ? '보관됨' : '사용 중' }}</p><p class="material-id">{{ entry.item.id }}</p></li></ul>
    <div v-else-if="!loading && !failure" class="empty-state"><h2>표시할 레시피가 없습니다</h2><p>새 레시피를 작성하거나 검색 조건을 바꿔 보세요.</p></div>
  </section>
</template>
