<script setup lang="ts">
import type { CorrectionInclusionCandidate } from '~/types/correction'
import type { ReceiptInclusionCandidates, ReceiptInclusionSelection } from '~/types/reconciliation'
import { apiRequest } from '~/utils/api'
import { correctionReferenceVersion } from '~/utils/correction'
import { blankReceiptSelection, receiptSourceLabels, shareText, sourceKeyText } from '~/utils/receipt'
import { displayDecimal } from '~/utils/catalog'
const rows = defineModel<ReceiptInclusionSelection[]>({ required: true })
const props = defineProps<{ prefix: string; field: string; disabled: boolean; materialId?: string | null; proposed?: CorrectionInclusionCandidate[] }>()
const loaded = ref<CorrectionInclusionCandidate[]>([])
const candidates = computed(() => props.proposed ?? loaded.value)
const failure = ref('')
let generation = 0, active = true
onScopeDispose(() => { active = false; generation += 1 })
watch(() => props.materialId, async material => {
  const current = ++generation
  loaded.value = []
  if (!material || props.proposed) return
  try {
    const result = await apiRequest<ReceiptInclusionCandidates>(`/api/materials/${encodeURIComponent(material)}/receipt-inclusion-bases`)
    if (active && current === generation && material === props.materialId) loaded.value = result.items.map(item => ({ ...item, observation: { kind: 'existing', entity: 'observation', id: item.observation_id, version_id: item.observation_version_id }, logical_seq: item.seq, sources: item.sources.map(source => ({ ...source, source_ref: { kind: 'existing', entity: source.source_key.kind === 'receipt_inflow' ? 'receipt' : 'observation', id: source.source_key.id, version_id: source.source_version_id } })) }))
  } catch { if (active && current === generation) failure.value = '실사 근거를 불러오지 못했습니다. 입력은 유지했습니다.' }
}, { immediate: true })
const observation = (row: ReceiptInclusionSelection) => candidates.value.find(item => correctionReferenceVersion(item.observation) === row.observation_version_id)
const sourceValue = (row: ReceiptInclusionSelection) => row.source?.kind === 'all_available' ? 'all' : row.source?.kind === 'source' ? sourceKeyText(row.source.source_key) : ''
function selectObservation(row: ReceiptInclusionSelection, event: Event) { if (props.disabled) return; row.observation_version_id = (event.target as HTMLSelectElement).value || null; row.source = null }
function selectSource(row: ReceiptInclusionSelection, event: Event) { if (props.disabled) return; const value = (event.target as HTMLSelectElement).value; const source = observation(row)?.sources.find(item => sourceKeyText(item.source_key) === value); row.source = value === 'all' ? { kind: 'all_available' } : source ? { kind: 'source', source_key: { ...source.source_key } } : null }
async function add() { if (props.disabled || rows.value.length >= 20) return; rows.value.push(blankReceiptSelection()); await nextTick(); document.getElementById(`${props.prefix}-${rows.value.length - 1}-observation`)?.focus() }
async function remove(index: number) { if (props.disabled) return; rows.value.splice(index, 1); await nextTick(); document.getElementById(`${props.prefix}-add`)?.focus() }
</script>
<template>
  <section class="receipt-inclusions"><h4>실사 포함 근거</h4><p class="field-hint">{{ proposed ? '이번 정정에 따른 제안 근거입니다. 원래 실사 참조를 보내고, 확정할 때 함께 개정된 근거로 연결합니다.' : '현재 기록에서 조회한 근거입니다. 이번 정정의 영향은 미리보기에서 다시 확인합니다.' }}</p><p v-if="failure" class="notice warning">{{ failure }}</p><p v-if="!rows.length" class="notice">포함 수량 0으로 명시합니다. 실제 입고 수량 전체를 새로 반영합니다.</p>
    <fieldset v-for="(row, index) in rows" :key="index" class="receipt-inclusion-row"><legend>포함 근거 {{ index + 1 }}</legend>
      <div class="field"><label :for="`${prefix}-${index}-observation`">관련 실사 {{ index + 1 }}</label><select :id="`${prefix}-${index}-observation`" :data-field="`${field}[${index}].observation_version_id`" :value="row.observation_version_id ?? ''" :disabled="disabled" @change="selectObservation(row, $event)"><option value="">실사 선택</option><option v-if="row.observation_version_id && !observation(row)" :value="row.observation_version_id">저장한 실사 재확인 필요 · {{ row.observation_version_id.slice(0, 8) }}</option><option v-for="item in candidates" :key="item.observation.id" :value="correctionReferenceVersion(item.observation) ?? ''">{{ item.observation.kind === 'proposed' ? '정정 제안' : '현재 기록' }} · {{ item.material_name }} · {{ displayDecimal(item.quantity) }}{{ item.unit }} · 기록 {{ item.logical_seq }}</option></select></div>
      <template v-if="observation(row)"><p>당시 가치 {{ displayDecimal(observation(row)!.value) }}원 · 선택 가능 {{ displayDecimal(observation(row)!.available_quantity) }}{{ observation(row)!.unit }}</p><NuxtLink :to="`/stocktakes/${observation(row)!.stocktake_id}`">관련 실사 보기</NuxtLink></template>
      <div class="field"><label :for="`${prefix}-${index}-source`">포함 수량의 근거 {{ index + 1 }}</label><select :id="`${prefix}-${index}-source`" :data-field="`${field}[${index}].source`" :value="sourceValue(row)" :disabled="disabled || !observation(row)" @change="selectSource(row, $event)"><option value="">근거 선택</option><option value="all">이 실사의 남은 근거 전체</option><option v-if="row.source?.kind === 'source' && !observation(row)?.sources.some(item => sourceKeyText(item.source_key) === sourceValue(row))" :value="sourceValue(row)">저장한 근거 재확인 필요</option><option v-for="source in observation(row)?.sources ?? []" :key="sourceKeyText(source.source_key)" :value="sourceKeyText(source.source_key)">{{ receiptSourceLabels[source.source_key.kind] }} · {{ source.source_material_name }} · 가능 {{ source.available_quantity }}{{ observation(row)!.unit }}</option></select></div>
      <details v-if="observation(row)"><summary>원천별 연결 한도</summary><ul><li v-for="source in observation(row)!.sources" :key="sourceKeyText(source.source_key)">{{ source.source_material_name }} · 원래 {{ source.source_quantity }}{{ source.source_unit }} · 남은 몫 {{ shareText(source.remaining_source_share) }} · 이 실사에서 선택 가능 {{ shareText(source.available_share) }}</li></ul><p class="field-hint">원천과 실사의 단위는 다를 수 있습니다. 포함량은 선택한 실사의 단위로 입력합니다.</p></details>
      <div class="field"><label :for="`${prefix}-${index}-quantity`">이미 포함된 수량 {{ index + 1 }} ({{ observation(row)?.unit ?? '실사 기준 단위' }})</label><input :id="`${prefix}-${index}-quantity`" v-model="row.included_quantity" :data-field="`${field}[${index}].included_quantity`" inputmode="decimal" :disabled="disabled"></div><button class="text-button" type="button" :disabled="disabled" @click="remove(index)">포함 근거 {{ index + 1 }} 삭제</button>
    </fieldset><button :id="`${prefix}-add`" class="secondary-button" type="button" :disabled="disabled || rows.length >= 20" @click="add">포함 근거 추가</button>
  </section>
</template>
