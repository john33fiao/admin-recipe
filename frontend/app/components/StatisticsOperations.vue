<script setup lang="ts">
import type { StatisticsReport } from '~/types/statistics'
import { coverageMessage, operationLabels } from '~/utils/statistics'
import { displayDecimal } from '~/utils/catalog'
import { formatCreatedAt } from '~/utils/material'
const props = defineProps<{ value: StatisticsReport['operations']; zone: string }>()
const filter = ref('')
const rows = computed(() => props.value.rows.filter(row => !filter.value || row.origin.kind === filter.value))
</script>
<template>
  <section class="statistics-section" aria-labelledby="statistics-operations-title"><h2 id="statistics-operations-title">종류별 확정 업무 이력</h2><p>{{ coverageMessage(value.coverage, '업무') }}</p><p class="field-hint">입고·생산·폐기·반품의 수량과 금액은 위의 판매 소모에 더하지 않습니다. 사용자 지정 입고일은 원확정 시각과 구별합니다.</p>
    <div class="field"><label for="statistics-operation-kind">이력 종류</label><select id="statistics-operation-kind" v-model="filter"><option value="">모든 종류</option><option v-for="kind in (['receipt', 'production', 'waste', 'purchase_return', 'stocktake'] as const)" :key="kind" :value="kind">{{ operationLabels[kind] }}</option></select></div><p v-if="filter && !rows.length">조회 기간에 이 종류의 기록이 없습니다.</p>
    <ol class="statistics-records"><li v-for="row in rows" :key="`${row.origin.kind}-${row.origin.id}`"><details><summary>{{ operationLabels[row.origin.kind] }} · {{ formatCreatedAt(row.origin.recorded_at, zone) }} · {{ row.status === 'void' ? '취소됨' : row.changes.length ? '정정 반영' : '원확정 유지' }}</summary><StatisticsReferences :origin="row.origin" :changes="row.changes" :zone="zone" /><p v-if="row.status === 'void'">취소된 업무입니다. 원기록은 보존하며 현재 효과를 0으로 만들지 않습니다.</p>
      <template v-if="row.receipt_basis"><h3>원입고</h3><p>{{ row.receipt_basis.original.material_name }} · {{ displayDecimal(row.receipt_basis.original.quantity) }}{{ row.receipt_basis.original.unit }} · {{ displayDecimal(row.receipt_basis.original.amount) }}원 · 입고일 {{ row.receipt_basis.original.receipt_date }}</p><template v-if="row.receipt_basis.effective"><h3>현재 유효 입고</h3><p>{{ row.receipt_basis.effective.material_name }} · {{ displayDecimal(row.receipt_basis.effective.quantity) }}{{ row.receipt_basis.effective.unit }} · {{ displayDecimal(row.receipt_basis.effective.amount) }}원 · 입고일 {{ row.receipt_basis.effective.receipt_date }}</p></template></template>
      <template v-if="row.status === 'active' && row.value"><ReceiptReconciliationResult v-if="row.value.kind === 'receipt' && row.value.reconciliation" :effect="row.value.reconciliation" /><ProductionResult v-else-if="row.value.kind === 'production'" :inputs="row.value.result.inputs" :outputs="row.value.result.outputs" :materials="row.value.result.materials" :total="row.value.result.total_input_value" /><WasteResult v-else-if="row.value.kind === 'waste'" :value="row.value.result" /><PurchaseReturnResult v-else-if="row.value.kind === 'purchase_return'" :value="row.value.result" /><p v-else-if="row.value.kind === 'stocktake'">{{ row.observation_ids.length ? `확인 수량 ${row.observation_ids.length}행의 원실사입니다. 유효 수량·소모는 위의 실사 영역에서 확인하세요.` : '확인 수량을 입력한 행이 없습니다. 수량 0이나 새 관측을 만들지 않았습니다.' }}</p></template>
    </details></li></ol>
  </section>
</template>
