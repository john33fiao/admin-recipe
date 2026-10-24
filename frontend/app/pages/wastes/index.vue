<script setup lang="ts">
import type { WasteList } from '~/types/waste'
import type { Material } from '~/types/material'
import { apiRequest } from '~/utils/api'
import { displayDecimal } from '~/utils/catalog'
import { formatCreatedAt } from '~/utils/material'
const route = useRoute()
const connection = useConnection()
const filter = ref(typeof route.query.material === 'string' ? route.query.material : '')
const materials = ref<Material[]>([])
const list = ref<WasteList | null>(null)
const loading = ref(false)
const failure = ref('')
let generation = 0
onScopeDispose(() => { generation += 1 })
async function load() {
  const current = ++generation
  if (connection.value !== 'connected') { loading.value = false; return }
  loading.value = true; failure.value = ''
  try {
    const [value, choices] = await Promise.all([apiRequest<WasteList>(`/api/wastes${filter.value ? `?material_id=${encodeURIComponent(filter.value)}` : ''}`), apiRequest<Material[]>('/api/materials?archive=all')])
    if (current === generation) { list.value = value; materials.value = choices }
  } catch { if (current === generation) failure.value = '폐기 · 직원식 기록을 불러오지 못했습니다. 다시 시도해 주세요.' }
  finally { if (current === generation) loading.value = false }
}
watch(() => route.query.material, value => { filter.value = typeof value === 'string' ? value : '' })
watch([connection, filter], () => { list.value = null; void load() }, { immediate: true })
useHead({ title: '폐기 · 직원식 기록 | 순창한상' })
</script>
<template>
  <div class="page-heading"><div><h1>폐기 · 직원식 기록</h1><p>명시 폐기와 직원식 수량·원가 반영 근거를 확인합니다.</p></div><NuxtLink class="primary-link" to="/business?kind=waste">폐기 · 직원식 초안 작성</NuxtLink></div>
  <section class="order-list-panel" aria-labelledby="waste-list-title" :aria-busy="loading"><div class="list-heading"><h2 id="waste-list-title">확정한 폐기 · 직원식</h2><button class="text-button" type="button" :disabled="loading || connection !== 'connected'" @click="load">다시 불러오기</button></div>
    <div class="field"><label for="waste-material-filter">품목 조회</label><select id="waste-material-filter" v-model="filter" :disabled="connection !== 'connected'"><option value="">모든 품목</option><option v-for="item in materials" :key="item.id" :value="item.id">{{ item.name }}{{ item.archived ? ' (보관됨)' : '' }} · {{ item.id.slice(0, 8) }}</option></select><p class="field-hint">선택한 품목의 확정 기록을 조회합니다.</p></div>
    <p v-if="failure" class="notice warning" role="alert">{{ failure }}</p><div v-else-if="loading && !list" class="empty-state"><h3>폐기 · 직원식 기록을 불러오고 있습니다</h3></div><div v-else-if="list && !list.items.length" class="empty-state"><h3>확정한 폐기 · 직원식 기록이 없습니다</h3><p>실제로 확인한 내용을 초안으로 작성하고 미리보기한 뒤 확정하세요.</p><NuxtLink to="/business">작성 중인 업무 확인</NuxtLink></div>
    <ul v-else-if="list" class="business-list waste-list"><li v-for="item in list.items" :key="item.id"><NuxtLink :to="`/wastes/${item.id}`"><div><strong>{{ item.effect.material_name }}</strong><span class="status-label">폐기 · 직원식 확정</span></div><p>{{ item.effect.reason === 'discard' ? '명시 폐기' : '직원식' }} · {{ displayDecimal(item.effect.quantity) }}{{ item.effect.unit }}</p><time>{{ formatCreatedAt(item.recorded_at) }} 기록 · {{ item.id.slice(0, 8) }}</time></NuxtLink></li></ul>
  </section>
</template>
