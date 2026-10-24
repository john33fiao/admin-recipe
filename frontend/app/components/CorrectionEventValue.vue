<script setup lang="ts">
import type { CorrectionEventValue } from '~/types/correction'
import { displayDecimal } from '~/utils/catalog'
defineProps<{ value: CorrectionEventValue | null }>()
</script>
<template>
  <div class="correction-event-value"><p v-if="!value" class="notice">이 기록의 유효 효과가 취소되었습니다. 원본 입력은 이력에 남습니다.</p>
    <template v-else-if="value.kind === 'receipt'"><dl><div><dt>입고 수량·금액</dt><dd>{{ displayDecimal(value.input.quantity) }}{{ value.input.unit }} · {{ value.input.amount === '' ? '금액 미확인' : displayDecimal(value.input.amount) + '원' }}</dd></div><div><dt>입고일</dt><dd>{{ value.input.receipt_date }}</dd></div><div><dt>연결</dt><dd>{{ value.input.source.kind === 'direct' ? '직접 입고' : '주문 연결 입고' }}</dd></div></dl><ReceiptReconciliationResult v-if="value.reconciliation" :effect="value.reconciliation"/></template>
    <ProductionResult v-else-if="value.kind === 'production'" :inputs="value.result.inputs" :outputs="value.result.outputs" :materials="value.result.materials" :total="value.result.total_input_value"/>
    <template v-else-if="value.kind === 'sale_count'"><p class="preview-result">판매 {{ displayDecimal(value.input.count) }}개 · {{ value.result.amount === null ? '판매금액 미계산' : displayDecimal(value.result.amount) + '원' }}</p><details><summary>당시 판매가와 레시피 원가</summary><SaleCostSnapshot :cost="value.result.recipe_cost"/></details></template>
    <PurchaseReturnResult v-else-if="value.kind === 'purchase_return'" :value="value.result"/>
    <WasteResult v-else-if="value.kind === 'waste'" :value="value.result"/>
    <ul v-else class="history-list"><li v-for="(row, index) in value.input.rows" :key="row.observation_id ?? index">실사 행 {{ index + 1 }} · {{ row.action.kind === 'void' ? '확인 기록 취소' : `${row.action.quantity}${row.action.unit}` }}<p class="material-id">관측 {{ row.observation_id }}</p></li></ul>
  </div>
</template>
