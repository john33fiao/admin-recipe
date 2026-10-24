<script setup lang="ts">
import type { StocktakePayload } from '~/types/stocktake'
import type { WastePayload } from '~/types/waste'
defineProps<{ payload: StocktakePayload | WastePayload }>()
</script>
<template>
  <ul v-if="payload.kind === 'stocktake'" class="history-list"><li v-for="(row, i) in payload.data.rows" :key="i"><p>{{ i + 1 }}행 · <NuxtLink v-if="row.material_id" :to="`/materials/${row.material_id}`">품목 {{ row.material_id.slice(0, 8) }}</NuxtLink><span v-else>품목 미선택</span> · {{ row.quantity.trim() === '' ? '수량 미확인' : row.quantity + row.unit }}</p><p v-if="row.initial_value">시작 가치: {{ row.initial_value.kind === 'existing_price' ? '기존 단가 적용' : row.initial_value.amount === '' ? '금액 미입력' : row.initial_value.amount + '원 직접 확인' }}</p><template v-if="row.excess"><p>초과 원인 확인: {{ row.excess.reviewed ? '확인함' : '미확인' }}</p><p class="preserve-lines">{{ row.excess.reason || '이유 미입력' }}</p><p v-if="row.excess.value">초과분 가치: {{ row.excess.value.kind === 'existing_price' ? '기존 단가 적용' : row.excess.value.amount === '' ? '금액 미입력' : row.excess.value.amount + '원 직접 확인' }}</p></template></li></ul>
  <p v-else>{{ payload.data.reason === 'discard' ? '명시 폐기' : '직원식' }} · {{ payload.data.quantity || '수량 미입력' }}{{ payload.data.unit }}</p>
</template>
