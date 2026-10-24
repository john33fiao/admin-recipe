<script setup lang="ts">
import type { ProductionPayload } from '~/types/production'
defineProps<{ payload: ProductionPayload }>()
</script>
<template>
  <div class="production-draft-summary"><section v-for="side in (['inputs', 'outputs'] as const)" :key="side"><h3>{{ side === 'inputs' ? '실제 투입 입력' : '실제 결과 입력' }}</h3><p v-if="!payload.data[side].length">입력한 행 없음</p><ol v-else class="history-list"><li v-for="(row, index) in payload.data[side]" :key="index"><p>{{ row.quantity || '수량 미입력' }} {{ row.unit || '단위 미입력' }}<template v-if="'allocation_percent' in row"> · 원가 배분 {{ row.allocation_percent || '미입력' }}%</template></p><NuxtLink v-if="row.material_id" :to="`/materials/${row.material_id}`">참조 품목 {{ row.material_id.slice(0, 8) }}</NuxtLink><p v-else>품목 미선택</p><p class="material-id">당시 품목 버전 {{ row.material_version_id ?? '미선택' }}</p></li></ol></section></div>
</template>
