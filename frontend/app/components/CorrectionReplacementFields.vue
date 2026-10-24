<script setup lang="ts">
import type { CorrectionReplacement, CorrectionInclusionCandidate } from '~/types/correction'
import { blankProductionInput, blankProductionOutput } from '~/utils/production'
import { cloneCorrection } from '~/utils/correction'
const replacement = defineModel<CorrectionReplacement>({ required: true })
const props = defineProps<{ disabled: boolean; prefix?: string; field?: string; original?: CorrectionReplacement | null; proposed?: CorrectionInclusionCandidate[] }>()
const prefix = computed(() => props.prefix ?? 'correction-replacement')
const field = computed(() => props.field ?? 'action.replacement.data')
function rowAction(index: number, event: Event) {
  if (props.disabled || replacement.value.kind !== 'stocktake') return
  const row = replacement.value.data.rows[index]!
  if ((event.target as HTMLSelectElement).value === 'void') row.action = { kind: 'void' }
  else { const old = props.original?.kind === 'stocktake' ? props.original.data.rows.find(value => value.observation_id === row.observation_id)?.action : null; row.action = old?.kind === 'replace' ? cloneCorrection(old) : { kind: 'replace', quantity: '', unit: '', initial_value: null, excess: null } }
}
async function restoreRows() { if (!props.disabled && replacement.value.kind === 'stocktake' && props.original?.kind === 'stocktake') { for (const row of props.original.data.rows) if (!replacement.value.data.rows.some(value => value.observation_id === row.observation_id)) replacement.value.data.rows.push(cloneCorrection(row)); await nextTick(); document.getElementById(`${prefix.value}-0-action`)?.focus() } }
async function removeStock(index: number) {
  if (props.disabled || replacement.value.kind !== 'stocktake') return
  replacement.value.data.rows.splice(index, 1)
  await nextTick()
  document.getElementById(replacement.value.data.rows.length ? `${prefix.value}-${Math.min(index, replacement.value.data.rows.length - 1)}-action` : `${prefix.value}-restore`)?.focus()
}
async function productionRow(group: 'inputs' | 'outputs', index?: number) {
  if (props.disabled || replacement.value.kind !== 'production') return
  if (index === undefined) { if (replacement.value.data[group].length >= 100) return; if (group === 'inputs') replacement.value.data.inputs.push(blankProductionInput()); else replacement.value.data.outputs.push(blankProductionOutput(replacement.value.data.outputs.length === 0)) }
  else replacement.value.data[group].splice(index, 1)
  await nextTick()
  const length = replacement.value.data[group].length
  document.getElementById(length ? `${prefix.value}-${group}-${index === undefined ? length - 1 : Math.min(index, length - 1)}-material` : `${prefix.value}-${group}-add`)?.focus()
}
</script>
<template>
  <div class="correction-replacement-fields">
    <div v-if="replacement.kind === 'sale_count'" class="field"><label :for="`${prefix}-count`">정정할 판매 개수</label><input :id="`${prefix}-count`" v-model="replacement.data.count" :data-field="`${field}.count`" inputmode="numeric" :disabled="disabled"><p class="field-hint">0개도 기록할 수 있습니다. 당시 레시피·판매가·원가는 유지하며 연결된 실사의 판매 개수 배분을 다시 확인합니다. 고객 환불이나 실물 반환을 기록하지 않습니다.</p></div>
    <CorrectionReceiptFields v-else-if="replacement.kind === 'receipt'" v-model="replacement.data" :prefix="prefix" :field="field" :disabled="disabled" :proposed="proposed"/>
    <template v-else-if="replacement.kind === 'production'">
      <p class="field-hint">실제 투입과 결과를 직접 수정하세요. 레시피나 수율로 실측량을 만들지 않습니다.</p>
      <section v-for="group in (['inputs', 'outputs'] as const)" :key="group"><h3>{{ group === 'inputs' ? '실제 투입' : '실제 결과' }}</h3><fieldset v-for="(row, index) in replacement.data[group]" :key="index" class="correction-production-row"><legend>{{ group === 'inputs' ? '투입' : '결과' }} {{ index + 1 }}</legend><CorrectionMaterialInput v-model="replacement.data[group][index]!" :id="`${prefix}-${group}-${index}-material`" :label="`${group === 'inputs' ? '투입' : '결과'} 품목 ${index + 1}`" :field="`${field}.${group}[${index}].material_id`" :disabled="disabled"/><div class="quantity-fields"><div class="field"><label :for="`${prefix}-${group}-${index}-quantity`">실제 수량 {{ index + 1 }}</label><input :id="`${prefix}-${group}-${index}-quantity`" v-model="row.quantity" :data-field="`${field}.${group}[${index}].quantity`" inputmode="decimal" :disabled="disabled"></div><div class="field"><label :for="`${prefix}-${group}-${index}-unit`">단위 {{ index + 1 }}</label><input :id="`${prefix}-${group}-${index}-unit`" v-model="row.unit" :data-field="`${field}.${group}[${index}].unit`" :disabled="disabled"></div><div v-if="'allocation_percent' in row" class="field"><label :for="`${prefix}-${group}-${index}-allocation`">원가 배분율 {{ index + 1 }} (%)</label><input :id="`${prefix}-${group}-${index}-allocation`" v-model="row.allocation_percent" :data-field="`${field}.${group}[${index}].allocation_percent`" inputmode="decimal" :disabled="disabled"></div></div><button class="text-button" type="button" :disabled="disabled" @click="productionRow(group, index)">{{ group === 'inputs' ? '투입' : '결과' }} {{ index + 1 }} 삭제</button></fieldset><button :id="`${prefix}-${group}-add`" class="secondary-button" type="button" :disabled="disabled || replacement.data[group].length >= 100" @click="productionRow(group)">{{ group === 'inputs' ? '투입' : '결과' }} 행 추가</button></section>
      <div class="field"><label :for="`${prefix}-note`">생산 메모</label><textarea :id="`${prefix}-note`" v-model="replacement.data.note" rows="2" :disabled="disabled"/></div>
    </template>
    <template v-else-if="replacement.kind === 'waste'"><CorrectionMaterialInput v-model="replacement.data" :id="`${prefix}-material`" label="폐기 · 직원식 품목" :field="`${field}.material_id`" :disabled="disabled"/><div class="quantity-fields"><div class="field"><label :for="`${prefix}-quantity`">실제 폐기 · 직원식 수량</label><input :id="`${prefix}-quantity`" v-model="replacement.data.quantity" :data-field="`${field}.quantity`" inputmode="decimal" :disabled="disabled"></div><div class="field"><label :for="`${prefix}-unit`">단위</label><input :id="`${prefix}-unit`" v-model="replacement.data.unit" :data-field="`${field}.unit`" :disabled="disabled"></div></div><div class="field"><label :for="`${prefix}-reason`">구분</label><select :id="`${prefix}-reason`" v-model="replacement.data.reason" :disabled="disabled"><option value="discard">명시 폐기</option><option value="staff_meal">직원식</option></select></div><div class="field"><label :for="`${prefix}-note`">폐기 · 직원식 메모</label><textarea :id="`${prefix}-note`" v-model="replacement.data.note" rows="2" :disabled="disabled"/></div></template>
    <template v-else><p class="notice">목록에서 제외한 실사 행은 기존 내용을 유지합니다. 행의 취소는 별도로 선택하세요. 빈 수량은 취소가 아닙니다.</p><fieldset v-for="(row, index) in replacement.data.rows" :key="row.observation_id ?? index" class="correction-stocktake-row"><legend>실사 행 {{ index + 1 }} · {{ row.observation_id?.slice(0, 8) ?? '참조 확인 필요' }}</legend><div class="field"><label :for="`${prefix}-${index}-action`">실사 행 {{ index + 1 }} 처리</label><select :id="`${prefix}-${index}-action`" :value="row.action.kind" :disabled="disabled" @change="rowAction(index, $event)"><option value="replace">확인 수량 정정</option><option value="void">이 확인 기록 취소</option></select></div><template v-if="row.action.kind === 'replace'"><div class="quantity-fields"><div class="field"><label :for="`${prefix}-${index}-quantity`">정정할 확인 수량 {{ index + 1 }}</label><input :id="`${prefix}-${index}-quantity`" v-model="row.action.quantity" :data-field="`${field}.rows[${index}].action.quantity`" inputmode="decimal" :disabled="disabled"></div><div class="field"><label :for="`${prefix}-${index}-unit`">실사 단위 {{ index + 1 }}</label><input :id="`${prefix}-${index}-unit`" v-model="row.action.unit" :data-field="`${field}.rows[${index}].action.unit`" :disabled="disabled"></div></div><CorrectionValuationFields v-model="row.action" :prefix="`${prefix}-${index}`" :field="`${field}.rows[${index}].action`" :disabled="disabled" :quantity="row.action.quantity"/></template><p v-else class="notice warning">이 확인 기록의 유효 효과를 취소합니다. 앞선 확인 기준을 다시 선택하며 원기록은 남습니다.</p><button class="text-button" type="button" :disabled="disabled" @click="removeStock(index)">이 행은 정정하지 않고 유지</button></fieldset><button :id="`${prefix}-restore`" class="secondary-button" type="button" :disabled="disabled" @click="restoreRows">제외한 원래 실사 행 다시 선택</button></template>
  </div>
</template>
