<script setup lang="ts">
import type { StocktakeValueDraft } from '~/types/stocktake'
import { explicitZeroValue } from '~/utils/stocktake'
const value = defineModel<StocktakeValueDraft | null>({ required: true })
const props = defineProps<{ id: string; label: string; disabled: boolean; quantity?: string; field: string; manualAllowed?: boolean; existingAllowed?: boolean }>()
function select(event: Event) { if (props.disabled) return; const kind = (event.target as HTMLSelectElement).value; value.value = kind === 'existing_price' ? { kind } : kind === 'manual_amount' ? { kind, amount: '' } : null }
function confirmZero() { if (!props.disabled && props.quantity !== undefined) { const zero = explicitZeroValue(props.quantity); if (zero) value.value = zero } }
</script>
<template>
  <div class="stock-value-input"><div class="field"><label :for="id">{{ label }}</label><select :id="id" :data-field="field" :value="value?.kind ?? ''" :disabled="disabled" @change="select"><option value="">아직 확인하지 않음</option><option value="existing_price" :disabled="existingAllowed === false">기존 단가 적용 확인</option><option value="manual_amount" :disabled="manualAllowed === false">총금액 직접 확인</option></select></div><div v-if="value?.kind === 'manual_amount'" class="field"><label :for="`${id}-amount`">{{ label }} 총금액 (원)</label><input :id="`${id}-amount`" v-model="value.amount" :data-field="`${field}.amount`" inputmode="decimal" :disabled="disabled"><p class="field-hint">빈칸은 금액 미확인, 0은 실제 0원입니다.</p></div><button v-if="quantity !== undefined && explicitZeroValue(quantity)" class="secondary-button" type="button" :disabled="disabled" @click="confirmZero">실제 수량 0 · 시작 가치 0원 확인</button><p v-if="quantity !== undefined && explicitZeroValue(quantity)" class="field-hint">실제 시작 수량이 0이므로 시작 가치도 0원입니다. 확인 버튼을 눌러 이 근거를 함께 보관하세요.</p></div>
</template>
