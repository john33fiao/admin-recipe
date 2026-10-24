<script setup lang="ts">
import type { StatisticsOrigin, StatisticsChangeRef } from '~/types/statistics'
import { originPath } from '~/utils/statistics'
import { formatCreatedAt } from '~/utils/material'
defineProps<{ origin: StatisticsOrigin; changes: StatisticsChangeRef[]; zone: string }>()
</script>
<template>
  <div class="statistics-references">
    <p><NuxtLink :to="originPath(origin)">원기록과 전체 이력</NuxtLink> · <NuxtLink :to="`/business/${origin.business_record_id}`">원업무</NuxtLink></p>
    <p v-for="change in changes" :key="`${change.kind}-${change.id}`"><template v-if="change.kind === 'correction'"><NuxtLink :to="`/corrections/${change.id}`">현재 값에 반영한 정정</NuxtLink></template><NuxtLink v-else :to="`/business/${change.business_record_id}`">판매 개수 개정 업무</NuxtLink> · <time :datetime="change.recorded_at">{{ formatCreatedAt(change.recorded_at, zone) }}</time></p>
  </div>
</template>
