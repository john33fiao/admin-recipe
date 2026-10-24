<script setup lang="ts">
import type { RecipeConnections } from '~/types/statistics'
import type { RecipeList } from '~/types/recipe'
import { ApiError, apiRequest } from '~/utils/api'
import { displayDecimal } from '~/utils/catalog'
const props = defineProps<{ materialId: string; revision?: string }>()
const connection = useConnection(), heading = useId()
const value = shallowRef<RecipeConnections | null>(null), uses = shallowRef<RecipeList | null>(null)
const loading = ref(false), failure = ref(''), usesFailure = ref(''), absent = ref(false)
const differentStates = computed(() => value.value && uses.value && (value.value.state.epoch !== uses.value.state.epoch || value.value.state.revision !== uses.value.state.revision))
let generation = 0
onScopeDispose(() => { generation += 1 })
async function load() {
  const token = ++generation, id = props.materialId
  loading.value = false
  if (connection.value !== 'connected' || !id) return
  loading.value = true; failure.value = ''; usesFailure.value = ''; absent.value = false; value.value = null; uses.value = null
  await Promise.all([
    (async () => { try { const result = await apiRequest<RecipeConnections>(`/api/recipes/${encodeURIComponent(id)}/connections`); if (token === generation) value.value = result }
      catch (cause) { if (token !== generation) return; if (cause instanceof ApiError && cause.status === 404) absent.value = true; else failure.value = cause instanceof ApiError && cause.code === 'connections_too_large' ? '연결 경로가 많아 전체를 표시할 수 없습니다. 직접 연결한 품목의 레시피에서 나누어 확인하세요.' : '현재 연결을 읽지 못했습니다. 다시 불러와 주세요.' } })(),
    (async () => { try { const result = await apiRequest<RecipeList>(`/api/materials/${encodeURIComponent(id)}/recipe-uses`); if (token === generation) uses.value = result }
      catch { if (token === generation) usesFailure.value = '직접 사용하는 레시피를 읽지 못했습니다.' } })(),
  ])
  if (token === generation) loading.value = false
}
watch([connection, () => props.materialId, () => props.revision], () => { value.value = null; uses.value = null; absent.value = false; void load() }, { immediate: true, flush: 'sync' })
</script>
<template>
  <section class="statistics-section recipe-connections" :aria-labelledby="heading" :aria-busy="loading"><div class="list-heading"><h2 :id="heading">현재 품목 연결</h2><button type="button" class="text-button" :disabled="loading || connection !== 'connected'" @click="load">연결 다시 조회</button></div><p class="field-hint">선택 품목과 하위 품목의 현재 레시피를 연결합니다. 판매 당시 전체 중첩 배합이 아니며 경로의 원가·소모를 합산하지 않습니다. 수량 0인 구성도 포함합니다.</p>
    <p v-if="loading" role="status">현재 연결을 불러오고 있습니다.</p><p v-if="failure" class="notice warning" role="alert">{{ failure }}</p><p v-if="absent">이 품목의 현재 레시피가 없습니다. 직접 사용하는 레시피는 아래에서 확인할 수 있습니다.</p>
    <template v-if="value"><p><NuxtLink :to="`/recipes/${value.item.id}/versions/${value.root_recipe_version_id}`">{{ value.item.name }}의 조회한 배합 버전</NuxtLink><span v-if="value.item.archived"> · 보관된 품목</span></p><p class="field-hint">연결 조회 기록 {{ value.state.revision }} · 통계와 별도로 읽은 현재 연결입니다.</p><h3>직접 구성 {{ value.direct.length }}행</h3><p v-if="!value.direct.length">직접 구성 품목이 없습니다.</p><ul class="history-list"><li v-for="step in value.direct" :key="step.line.id"><NuxtLink :to="`/materials/${step.line.material_id}`">{{ step.line.material_name }}</NuxtLink> · {{ displayDecimal(step.basis_count) }}개 기준 {{ displayDecimal(step.line.input_quantity) }}{{ step.line.input_unit }}<p v-if="step.child_recipe_version_id"><NuxtLink :to="`/recipes/${step.line.material_id}/versions/${step.child_recipe_version_id}`">하위 품목의 조회한 현재 배합</NuxtLink></p><details><summary>보존된 구성 품목 버전</summary><span class="material-id">{{ step.line.material_version_id }}</span></details></li></ul><h3>간접 연결 {{ value.paths.length }}경로</h3><p v-if="!value.paths.length">두 단계 이상의 간접 경로가 없습니다.</p><ol class="statistics-records"><li v-for="(path, index) in value.paths" :key="index"><details><summary>{{ path.steps.map(step => step.line.material_name).join(' → ') }}</summary><ol class="history-list"><li v-for="(step, position) in path.steps" :key="`${step.recipe_version_id}-${step.line.id}-${position}`"><NuxtLink :to="`/recipes/${step.recipe_material_id}/versions/${step.recipe_version_id}`">{{ position + 1 }}단계 배합</NuxtLink> · <NuxtLink :to="`/materials/${step.line.material_id}`">{{ step.line.material_name }}</NuxtLink><p>{{ displayDecimal(step.basis_count) }}개 기준 구성 원입력 {{ displayDecimal(step.line.input_quantity) }}{{ step.line.input_unit }}</p><p class="material-id">구성 품목 버전 {{ step.line.material_version_id }}</p></li></ol></details></li></ol></template>
    <h3>이 품목을 직접 사용하는 현재 레시피</h3><p v-if="usesFailure" class="notice warning" role="alert">{{ usesFailure }}</p><template v-if="uses"><p class="field-hint">직접 사용 목록 조회 기록 {{ uses.state.revision }}</p><p v-if="differentStates" class="notice">서로 다른 시점에 읽은 연결입니다. 연결 다시 조회를 눌러 함께 확인해 주세요.</p><p v-if="!uses.items.length">현재 직접 사용하는 레시피가 없습니다.</p><ul class="history-list"><li v-for="entry in uses.items" :key="entry.item.id"><NuxtLink :to="`/recipes/${entry.item.id}/versions/${entry.recipe_version_id}`">{{ entry.item.name }}</NuxtLink><span v-if="entry.item.archived"> · 보관됨</span></li></ul></template>
  </section>
</template>
