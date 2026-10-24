<script setup lang="ts">
import type { ReceiptInclusionCandidates, ReceiptInclusionSelection } from '~/types/reconciliation'
import { apiRequest } from '~/utils/api'
import { displayDecimal } from '~/utils/catalog'
import { formatCreatedAt } from '~/utils/material'
import { blankReceiptSelection, receiptSourceLabels, shareText, sourceKeyText } from '~/utils/receipt'
const rows = defineModel<ReceiptInclusionSelection[]>({ required: true })
const props = defineProps<{ materialId: string | null | undefined; locked: boolean; available: boolean; issue: (field: string) => string | undefined }>()
const candidates = ref<ReceiptInclusionCandidates | null>(null)
const loading = ref(false)
const failure = ref('')
let generation = 0
let active = true
onScopeDispose(() => { active = false; generation += 1 })
async function load() {
  const current = ++generation
  const material = props.materialId
  loading.value = false
  if (!material || !props.available) return
  loading.value = true
  failure.value = ''
  try {
    const result = await apiRequest<ReceiptInclusionCandidates>(`/api/materials/${encodeURIComponent(material)}/receipt-inclusion-bases`)
    if (active && current === generation && material === props.materialId) candidates.value = result
  } catch { if (active && current === generation) failure.value = '선택할 실사 근거를 불러오지 못했습니다. 작성한 입력을 유지했습니다.' }
  finally { if (active && current === generation) loading.value = false }
}
watch(() => props.materialId, () => { candidates.value = null })
watch([() => props.materialId, () => props.available], () => { void load() }, { immediate: true })
const observation = (row: ReceiptInclusionSelection) => candidates.value?.items.find(item => item.observation_version_id === row.observation_version_id)
const sourceValue = (row: ReceiptInclusionSelection) => row.source?.kind === 'all_available' ? 'all' : row.source?.kind === 'source' ? sourceKeyText(row.source.source_key) : ''
const selectedSource = (row: ReceiptInclusionSelection) => observation(row)?.sources.find(item => sourceKeyText(item.source_key) === sourceValue(row))
const rowIssue = (index: number, field: string) => props.issue(`stock_effect.selections.${index}.${field}`) ?? props.issue(`selections.${index}.${field}`)
function changeObservation(row: ReceiptInclusionSelection, event: Event) {
  if (props.locked || !rows.value.includes(row)) return
  row.observation_version_id = (event.target as HTMLSelectElement).value || null
  row.source = null
}
function changeSource(row: ReceiptInclusionSelection, event: Event) {
  if (props.locked || !rows.value.includes(row)) return
  const value = (event.target as HTMLSelectElement).value
  const source = observation(row)?.sources.find(item => sourceKeyText(item.source_key) === value)
  row.source = value === 'all' ? { kind: 'all_available' } : source ? { kind: 'source', source_key: { ...source.source_key } } : null
}
async function focusRow(index: number) {
  await nextTick()
  document.getElementById(index >= 0 && index < rows.value.length ? `receipt-inclusion-${index}-observation_version_id` : 'receipt-inclusion-add')?.focus()
}
function add() {
  if (props.locked || rows.value.length >= 20) return
  rows.value.push(blankReceiptSelection())
  void focusRow(rows.value.length - 1)
}
function remove(index: number) {
  if (props.locked) return
  rows.value.splice(index, 1)
  void focusRow(Math.min(index, rows.value.length - 1))
}
</script>
<template>
  <section class="receipt-inclusions" aria-labelledby="receipt-inclusions-title" :aria-busy="loading">
    <div class="list-heading"><h3 id="receipt-inclusions-title">실사에 이미 포함된 수량</h3><button class="text-button" type="button" :disabled="!available || loading || !materialId" @click="load">실사 근거 새로고침</button></div>
    <p class="field-hint">입고일로 포함량을 추정하지 않습니다. 확인한 실사와 포함된 양을 선택하세요. 이후 사용한 수량도 당시 근거로 확인할 수 있습니다.</p>
    <p v-if="!materialId" class="notice">입고 품목을 먼저 선택해 주세요.</p>
    <p v-if="failure" class="notice warning" role="alert">{{ failure }}</p>
    <p v-if="loading" class="stale-note">실사 근거를 불러오고 있습니다.</p>
    <p v-if="candidates && !candidates.items.length" class="notice">이 품목에 선택할 수 있는 실사 근거가 없습니다. 미완성 입력은 초안으로 보관할 수 있습니다.</p>
    <p v-if="!rows.length" class="notice">포함 수량 0으로 확인했습니다. 실제 입고 수량 전체를 새로 반영합니다.</p>
    <fieldset v-for="(row, index) in rows" :key="index" class="receipt-inclusion-row">
      <legend>포함 근거 {{ index + 1 }}</legend>
      <div class="field"><label :for="`receipt-inclusion-${index}-observation_version_id`">관련 실사 {{ index + 1 }}</label><select :id="`receipt-inclusion-${index}-observation_version_id`" :value="row.observation_version_id ?? ''" :disabled="locked || !available" :aria-invalid="!!rowIssue(index, 'observation_version_id')" :aria-describedby="`receipt-inclusion-${index}-observation-error`" @change="changeObservation(row, $event)"><option value="">확인한 실사 선택</option><option v-if="row.observation_version_id && !observation(row)" :value="row.observation_version_id">선택한 실사 재확인 필요 · {{ row.observation_version_id.slice(0, 8) }}</option><option v-for="item in candidates?.items ?? []" :key="item.observation_version_id" :value="item.observation_version_id">{{ formatCreatedAt(item.recorded_at) }} · {{ displayDecimal(item.quantity) }}{{ item.unit }} · 기록 {{ item.seq }}</option></select><p :id="`receipt-inclusion-${index}-observation-error`" class="field-error">{{ rowIssue(index, 'observation_version_id') }}</p></div>
      <div v-if="observation(row)" class="receipt-inclusion-evidence"><p>당시 확인 수량 {{ displayDecimal(observation(row)!.quantity) }}{{ observation(row)!.unit }} · 당시 가치 {{ displayDecimal(observation(row)!.value) }}원</p><p>현재 선택 가능한 포함량 {{ displayDecimal(observation(row)!.available_quantity) }}{{ observation(row)!.unit }}</p><NuxtLink :to="`/stocktakes/${observation(row)!.stocktake_id}`">선택한 실사 기록 보기</NuxtLink></div>
      <div class="field"><label :for="`receipt-inclusion-${index}-source`">포함 수량의 근거 {{ index + 1 }}</label><select :id="`receipt-inclusion-${index}-source`" :value="sourceValue(row)" :disabled="locked || !available || !observation(row)" :aria-invalid="!!rowIssue(index, 'source')" :aria-describedby="`receipt-inclusion-${index}-source-error`" @change="changeSource(row, $event)"><option value="">근거 선택</option><option value="all">이 실사의 남은 근거 전체</option><option v-if="row.source?.kind === 'source' && !selectedSource(row)" :value="sourceValue(row)">선택한 근거 재확인 필요</option><option v-for="item in observation(row)?.sources ?? []" :key="sourceKeyText(item.source_key)" :value="sourceKeyText(item.source_key)" :disabled="item.available_quantity === '0' && sourceValue(row) !== sourceKeyText(item.source_key)">{{ receiptSourceLabels[item.source_key.kind] }} · {{ item.source_material_name }} · 가능 {{ displayDecimal(item.available_quantity) }}{{ observation(row)!.unit }}</option></select><p :id="`receipt-inclusion-${index}-source-error`" class="field-error">{{ rowIssue(index, 'source') }}</p></div>
      <p v-if="row.source?.kind === 'all_available'" class="field-hint">이 실사에 남아 있는 여러 근거를 함께 확인합니다. 실제 연결되는 몫은 미리보기에서 확인할 수 있습니다.</p>
      <details v-if="selectedSource(row)" class="holding-references"><summary>원래 근거의 연결 한도</summary><p>{{ selectedSource(row)!.source_material_name }} · 원래 수량 {{ displayDecimal(selectedSource(row)!.source_quantity) }}{{ selectedSource(row)!.source_unit }}</p><p>이미 연결 {{ shareText(selectedSource(row)!.reserved_source_share) }} · 남은 몫 {{ shareText(selectedSource(row)!.remaining_source_share) }}</p><p>선택한 실사에서 가능한 정확한 몫 {{ shareText(selectedSource(row)!.available_share) }}</p><p class="field-hint">원래 근거와 현재 품목의 단위는 다를 수 있습니다. 서로 다른 단위의 수량은 더하지 않습니다.</p></details>
      <div class="field"><label :for="`receipt-inclusion-${index}-included_quantity`">이미 포함된 수량 {{ index + 1 }} <span class="required-label">{{ observation(row)?.unit ?? '선택 실사의 기준 단위' }}</span></label><input :id="`receipt-inclusion-${index}-included_quantity`" v-model="row.included_quantity" inputmode="decimal" :readonly="locked" :disabled="!available" :aria-invalid="!!rowIssue(index, 'included_quantity')" :aria-describedby="`receipt-inclusion-${index}-quantity-error`"><p :id="`receipt-inclusion-${index}-quantity-error`" class="field-error">{{ rowIssue(index, 'included_quantity') }}</p></div>
      <button class="text-button" type="button" :disabled="locked || !available" :aria-label="`포함 근거 ${index + 1} 삭제`" @click="remove(index)">이 근거 삭제</button>
    </fieldset>
    <button id="receipt-inclusion-add" class="secondary-button" type="button" :disabled="locked || !available || rows.length >= 20" @click="add">포함 근거 추가</button>
    <p class="field-hint">최대 20행입니다. 마지막 행을 삭제하면 포함 수량 0으로 확인합니다. 선택한 양은 미리보기에서 다시 검증합니다.</p>
    <p class="field-error">{{ issue('stock_effect.selections') }}</p>
  </section>
</template>
