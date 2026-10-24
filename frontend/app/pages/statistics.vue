<script setup lang="ts">
import type { AppliedStatisticsRange } from '~/types/statistics'
import type { Material } from '~/types/material'
import { currentTimeZone, localDateRange, localDateText, rangeDateFields, rangeFromQuery } from '~/utils/statistics'
import { formatCreatedAt } from '~/utils/material'
import { apiRequest } from '~/utils/api'
const route = useRoute(), router = useRouter()
const range = shallowRef<AppliedStatisticsRange | null>(null)
const start = ref(localDateText()), last = ref(localDateText()), dateFailure = ref(''), dateInput = ref<HTMLInputElement | null>(null)
const pcZone = ref(currentTimeZone()), selectedMaterial = ref(typeof route.query.material === 'string' ? route.query.material : '')
const materials = shallowRef<Material[]>([]), materialFailure = ref('')
const { snapshot, failure, loading, load, connection } = useStatistics(range)
let materialGeneration = 0
onScopeDispose(() => { materialGeneration += 1 })
async function loadMaterials() {
  const token = ++materialGeneration
  if (connection.value !== 'connected') return
  try { const result = await apiRequest<Material[]>('/api/materials?archive=all'); if (token === materialGeneration) { materials.value = result; materialFailure.value = '' } }
  catch { if (token === materialGeneration) materialFailure.value = '품목 목록을 읽지 못했습니다.' }
}
async function readUrl() {
  try {
    const saved = rangeFromQuery(route.query)
    if (!saved) { const initial = localDateRange(localDateText(), localDateText()); await router.replace({ query: { ...route.query, ...initial } }); return }
    range.value = saved; const fields = rangeDateFields(saved); start.value = fields.start; last.value = fields.last; dateFailure.value = ''
  } catch (cause) { range.value = null; dateFailure.value = cause instanceof Error ? cause.message : '기간을 확인해 주세요.' }
}
async function query() {
  try {
    pcZone.value = currentTimeZone()
    const next = localDateRange(start.value, last.value)
    dateFailure.value = ''
    if (range.value && next.from === range.value.from && next.until === range.value.until && next.zone === range.value.zone) await load()
    else await router.push({ query: { ...route.query, ...next } })
  } catch (cause) { dateFailure.value = cause instanceof Error ? cause.message : '기간을 확인해 주세요.'; await nextTick(); dateInput.value?.focus() }
}
async function selectMaterial() { await router.replace({ query: { ...route.query, material: selectedMaterial.value || undefined } }) }
const oldResult = computed(() => snapshot.value && (!range.value || snapshot.value.range.from !== range.value.from || snapshot.value.range.until !== range.value.until || snapshot.value.range.zone !== range.value.zone || failure.value || loading.value || connection.value !== 'connected'))
watch([() => route.query.from, () => route.query.until, () => route.query.zone], () => { void readUrl() }, { immediate: true })
watch(() => route.query.material, value => { selectedMaterial.value = typeof value === 'string' ? value : '' })
watch(connection, () => { void loadMaterials() }, { immediate: true })
useHead({ title: '통계·이력 | 순창한상' })
</script>
<template>
  <div class="page-heading"><div><h1>통계·이력</h1><p>원래 확정한 시각의 기록에 현재 유효한 정정을 반영해 확인합니다.</p></div><NuxtLink to="/workspace">모아 입력으로</NuxtLink></div>
  <form class="statistics-period" aria-label="통계 조회 기간" novalidate @submit.prevent="query"><div class="field"><label for="statistics-start">시작 날짜</label><input id="statistics-start" ref="dateInput" v-model="start" type="date" :aria-invalid="!!dateFailure" aria-describedby="statistics-date-help statistics-date-error"></div><div class="field"><label for="statistics-last">마지막 날짜 (포함)</label><input id="statistics-last" v-model="last" type="date" :aria-invalid="!!dateFailure" aria-describedby="statistics-date-help statistics-date-error"></div><button class="primary-button" type="submit" :disabled="connection !== 'connected'">기간 조회</button><p id="statistics-date-help" class="field-hint">새 날짜 조회는 PC 시간대 {{ pcZone }}의 하루 시작부터 마지막 날짜의 다음날 시작 전까지입니다. 입고일이 아닌 원확정 시각으로 조회합니다.</p><p v-if="dateFailure" id="statistics-date-error" class="notice warning" role="alert">{{ dateFailure }}</p></form>
  <p v-if="range && range.zone !== pcZone" class="notice">적용된 표시 시간대는 {{ range.zone }}입니다. URL의 원래 절대 기간을 유지합니다. 날짜를 새로 조회하면 현재 PC 시간대를 사용합니다.</p>
  <p v-if="failure" class="notice warning" role="alert">{{ failure }}</p><p v-if="loading" role="status">기간의 기록을 조회하고 있습니다.</p>
  <nav v-if="snapshot" class="statistics-sections" aria-label="보고서 영역"><a href="#statistics-sales-title">판매·원가</a><a href="#statistics-observations-title">실사 소모·배분</a><a href="#statistics-operations-title">종류별 이력</a><a href="#statistics-connections-title">품목 연결</a></nav>
  <div v-if="snapshot" class="statistics-report" :aria-busy="loading"><section class="statistics-applied" aria-labelledby="statistics-applied-title"><h2 id="statistics-applied-title">{{ oldResult ? '이전에 조회한 결과' : '적용된 조회 기간' }}</h2><p>{{ formatCreatedAt(snapshot.range.from, snapshot.range.zone) }} 포함 — {{ formatCreatedAt(snapshot.range.until, snapshot.range.zone) }} 제외</p><p>{{ snapshot.range.zone }} · 조회 기록 {{ snapshot.report.state.revision }}</p><p class="field-hint">원확정 시각 기준 · 조회 시점의 현재 유효 정정 반영. 상세 화면은 이후 변경을 보여줄 수 있습니다. 아래 값은 이 조회 결과에 고정됩니다.</p><details><summary>정확한 절대 기간</summary><p class="material-id">시작 {{ snapshot.range.from }}<br>끝 {{ snapshot.range.until }}</p></details><button type="button" class="text-button" :disabled="!range || loading || connection !== 'connected'" @click="load">같은 절대 기간 다시 조회</button></section>
    <StatisticsSales :value="snapshot.report.sales" :zone="snapshot.range.zone" /><StatisticsObservations :value="snapshot.report.observations" :zone="snapshot.range.zone" /><StatisticsOperations :value="snapshot.report.operations" :zone="snapshot.range.zone" />
  </div>
  <section class="statistics-section" aria-labelledby="statistics-connections-title"><h2 id="statistics-connections-title">품목 연결 조회</h2><p class="field-hint">기간 보고서와 독립적으로 현재 배합을 조회합니다. 보관된 품목도 선택할 수 있습니다.</p><p v-if="materialFailure" class="notice warning" role="alert">{{ materialFailure }} <button class="text-button" type="button" @click="loadMaterials">품목 다시 불러오기</button></p><div class="field"><label for="statistics-material">연결을 볼 품목</label><select id="statistics-material" v-model="selectedMaterial" @change="selectMaterial"><option value="">품목 선택</option><option v-for="item in materials" :key="item.id" :value="item.id">{{ item.name }}{{ item.archived ? ' (보관됨)' : '' }} · {{ item.id.slice(0, 8) }}</option></select></div><RecipeConnectionsPanel v-if="selectedMaterial" :key="selectedMaterial" :material-id="selectedMaterial" /></section>
</template>
