<script setup lang="ts">
import type { WasteEffect } from '~/types/waste'
import { displayDecimal } from '~/utils/catalog'
import { wasteReasons } from '~/utils/stocktake'
defineProps<{ value: WasteEffect }>()
</script>
<template>
  <div class="waste-result"><h3><NuxtLink :to="`/materials/${value.material_id}`">{{ value.material_name }}</NuxtLink> · {{ wasteReasons[value.reason] }}</h3><dl class="stocktake-fields"><div><dt>실제 수량</dt><dd>{{ displayDecimal(value.quantity) }}{{ value.unit }} <span class="field-hint">입력 {{ value.input_quantity }}{{ value.input_unit }}</span></dd></div><div><dt>이전 정산 기준</dt><dd>{{ displayDecimal(value.before.quantity) }}{{ value.unit }} · {{ value.before.value === null ? '가치 미확인' : displayDecimal(value.before.value) + '원' }}</dd></div><div><dt>이후 정산 기준</dt><dd>{{ displayDecimal(value.after.quantity) }}{{ value.unit }} · {{ value.after.value === null ? '가치 미확인' : displayDecimal(value.after.value) + '원' }}</dd></div><div><dt>전량 폐기 손실</dt><dd>{{ displayDecimal(value.loss_value) }}원</dd></div></dl><p class="field-hint">일부가 남으면 총원가를 잔량에 유지합니다. 전량을 폐기하면 남은 원가는 손실로 종료하며 다음 입고로 넘기지 않습니다. 이 수량은 판매 소모에서 제외됩니다.</p></div>
</template>
