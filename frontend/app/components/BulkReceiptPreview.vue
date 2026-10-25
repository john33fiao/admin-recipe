<script setup lang="ts">
import type { BusinessPreview } from '~/types/business'
import { displayDecimal } from '~/utils/catalog'
defineProps<{ effect: BusinessPreview['effect'] }>()
</script>
<template>
  <ReceiptReconciliationResult v-if="effect?.kind === 'reconciled_receipt'" :effect="effect.receipt" />
  <dl v-else-if="effect?.kind === 'receipt'" class="bulk-receipt-values"><div><dt>실제 입고</dt><dd>{{ displayDecimal(effect.input_quantity) }}{{ effect.input_unit }} · {{ displayDecimal(effect.amount) }}원</dd></div><div><dt>입고일</dt><dd>{{ effect.receipt_date }}</dd></div><div><dt>이전 정산 기준</dt><dd>{{ effect.before ? `${displayDecimal(effect.before.quantity)}${effect.unit} · ${effect.before.value === null ? '가치 미확인' : displayDecimal(effect.before.value) + '원'}` : '정산 기준 없음' }}</dd></div><div><dt>입고 후 정산 기준</dt><dd>{{ displayDecimal(effect.after.quantity) }}{{ effect.unit }} · {{ effect.after.value === null ? '가치 미확인' : displayDecimal(effect.after.value) + '원' }}</dd></div></dl>
</template>
<style scoped>
.bulk-receipt-values { display: grid; grid-template-columns: repeat(auto-fit,minmax(200px,1fr)); gap: 16px; margin: 16px 0; }
dt { color: var(--muted); } dd { margin: 4px 0 0; font-variant-numeric: tabular-nums; overflow-wrap: anywhere; }
</style>
