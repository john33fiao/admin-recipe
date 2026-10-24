<script setup lang="ts">
import type { RecipeVersion } from '~/types/recipe'
import { apiRequest } from '~/utils/api'
import { displayDecimal } from '~/utils/catalog'
import { formatCreatedAt } from '~/utils/material'
const route = useRoute(), connection = useConnection()
const value = ref<RecipeVersion | null>(null), failure = ref('')
let generation = 0
onScopeDispose(() => { generation += 1 })
async function load() {
  const token = ++generation
  if (connection.value !== 'connected') return
  try { const result = await apiRequest<RecipeVersion>(`/api/recipes/${encodeURIComponent(String(route.params.id))}/versions/${encodeURIComponent(String(route.params.version))}`); if (token === generation) { value.value = result; failure.value = '' } }
  catch { if (token === generation) failure.value = '이 레시피 버전을 읽지 못했습니다.' }
}
watch([connection, () => route.params.id, () => route.params.version], () => { value.value = null; void load() }, { immediate: true })
useHead({ title: '레시피 과거 버전 | 순창한상' })
</script>
<template>
  <NuxtLink class="back-link" :to="`/recipes/${route.params.id}`">‹ 현재 레시피</NuxtLink>
  <p v-if="failure" class="notice warning" role="alert">{{ failure }} <button class="text-button" type="button" @click="load">다시 불러오기</button></p>
  <template v-if="value"><div class="page-heading"><div><h1>{{ value.material_name }}</h1><p>저장된 과거 버전 · {{ formatCreatedAt(value.recorded_at) }}</p></div></div>
    <section class="detail-panel"><dl class="detail-fields"><div><dt>기준 개수</dt><dd>{{ displayDecimal(value.basis_count) }}개</dd></div><div v-if="value.reference_output"><dt>참고 생산량</dt><dd>{{ value.reference_output.input_quantity }}{{ value.reference_output.input_unit }} · 실제 생산 기록과 구분합니다.</dd></div><div><dt>판매가</dt><dd>{{ value.selling_price === null ? '미입력' : `${displayDecimal(value.selling_price)}원` }}</dd></div><div><dt>조리 설명</dt><dd class="preserve-lines">{{ value.cooking_description || '미입력' }}</dd></div><div><dt>조리 공정</dt><dd class="preserve-lines">{{ value.process || '미입력' }}</dd></div><div><dt>참고 시간</dt><dd>{{ value.reference_time || '미입력' }}</dd></div></dl>
      <h2>당시 구성</h2><ul class="history-list"><li v-for="line in value.lines" :key="line.id"><NuxtLink :to="`/materials/${line.material_id}`">{{ line.material_name }}의 현재 품목 정보</NuxtLink><p>기준 전체 {{ line.input_quantity }}{{ line.input_unit }}</p><p class="preserve-lines">{{ line.note }}</p><p class="material-id">당시 품목 버전 {{ line.material_version_id }}</p></li></ul><p v-if="!value.lines.length" class="stale-note">구성 재료 없이 저장한 배합입니다.</p><p class="material-id">{{ value.id }}</p>
    </section><RecipeCostPanel :version="value" past />
  </template>
</template>
