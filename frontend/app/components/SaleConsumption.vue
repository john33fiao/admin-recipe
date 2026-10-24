<script setup lang="ts">
import type { SaleMaterialConsumption, SaleConsumptionStatus } from '~/types/sale'
import { saleConsumptionLabels } from '~/utils/sale'
import { displayDecimal } from '~/utils/catalog'
defineProps<{ status: SaleConsumptionStatus; rows: SaleMaterialConsumption[] }>()
const labels = { unconfirmed: '실사 소모 미확정', allocated: '배분됨', unallocated: '미배분' }
const reasons = { not_observed: '아직 연결된 실사가 없습니다.', initial_basis_missing: '실제 시작 기준 이전의 판매입니다. 소모를 임의로 역산하지 않습니다.', zero_weight: '대상 판매의 총개수가 0이라 소모와 원가를 배분하지 않았습니다.' }
</script>
<template>
  <section class="history-section sale-consumption" aria-labelledby="sale-consumption-title"><div class="list-heading"><h2 id="sale-consumption-title">{{ status === 'unconfirmed' ? '실사 소모 미확정' : '실사 소모 배분' }}</h2><span class="status-label">{{ saleConsumptionLabels[status] }}</span></div><p>확인한 수량과 판매 개수 비율에 따른 배분입니다. 판매 당시 레시피 원가와 다른 기록이며 두 원가를 합하지 않습니다.</p><p class="field-hint">당시 레시피의 직접 구성 재료만 연결합니다. 0수량 참조도 포함하며 같은 재료는 한 번 표시합니다.</p>
    <ul v-if="rows.length" class="history-list"><li v-for="row in rows" :key="row.material_id"><div><NuxtLink :to="`/materials/${row.material_id}`">{{ row.material_name }}</NuxtLink><span class="status-label">{{ labels[row.status] }}</span></div><p>배분 수량 {{ row.quantity === null ? '미확정' : displayDecimal(row.quantity) + row.unit }} · 배분 원가 {{ row.value === null ? '미확정' : displayDecimal(row.value) + '원' }}</p><p v-if="row.reason" class="field-hint">{{ reasons[row.reason] }}</p><NuxtLink v-if="row.stocktake_id" :to="`/stocktakes/${row.stocktake_id}`">연결된 실사와 배분 근거</NuxtLink><details v-if="row.observation_version_id || row.consumption_version_id"><summary>당시 참조 확인</summary><p class="material-id">관측 버전 {{ row.observation_version_id }} · 배분 버전 {{ row.consumption_version_id }} · 판매 버전 {{ row.sale_version_id }}</p></details></li></ul>
    <p v-else>직접 구성 재료가 없습니다. 실제 소모가 0이라는 뜻은 아닙니다.</p>
    <p v-if="rows.some(row => row.consumption_version_id)" class="notice warning">판매 개수를 바꾸려면 연결된 실사의 배분을 함께 정정해야 합니다. 개수 정정 초안은 보관할 수 있으며 현재 기록과 입력은 유지됩니다.</p>
  </section>
</template>
