<script setup lang="ts">
import type { StocktakeExcessDraft, StocktakeValueDraft } from '~/types/stocktake'
const data = defineModel<{ initial_value: StocktakeValueDraft | null; excess: StocktakeExcessDraft | null }>({ required: true })
defineProps<{ prefix: string; field: string; disabled: boolean; quantity?: string; mode?: 'initial' | 'excess'; existingAllowed?: boolean; manualAllowed?: boolean }>()
</script>
<template>
  <div class="correction-valuation"><StockValueInput v-if="!mode || mode === 'initial'" :id="`${prefix}-initial`" v-model="data.initial_value" label="시작 가치" :field="`${field}.initial_value`" :disabled="disabled" :quantity="quantity" :existing-allowed="existingAllowed" :manual-allowed="manualAllowed"/>
    <template v-if="!mode || mode === 'excess'"><button v-if="!data.excess" class="secondary-button" type="button" :disabled="disabled" @click="data.excess = { reviewed: false, reason: '', value: null }">초과분 확인 내용 입력</button><fieldset v-else class="correction-excess"><legend>초과 수량의 확인 근거</legend><label class="checkbox-label"><input v-model="data.excess.reviewed" type="checkbox" :data-field="`${field}.excess.reviewed`" :disabled="disabled">실제로 확인한 초과 수량입니다</label><div class="field"><label :for="`${prefix}-excess-reason`">초과 확인 이유</label><textarea :id="`${prefix}-excess-reason`" v-model="data.excess.reason" :data-field="`${field}.excess.reason`" rows="2" :disabled="disabled"/></div><StockValueInput :id="`${prefix}-excess-value`" v-model="data.excess.value" label="초과분 가치" :field="`${field}.excess.value`" :disabled="disabled" :existing-allowed="existingAllowed" :manual-allowed="manualAllowed"/><button class="text-button" type="button" :disabled="disabled" @click="data.excess = null">초과 확인 입력 제거</button></fieldset></template>
    <p class="field-hint">입력만으로 초과 수량이나 0원을 만들지 않습니다. 제안된 과거 위치의 가치 출처는 미리보기에서 확인합니다.</p>
  </div>
</template>
