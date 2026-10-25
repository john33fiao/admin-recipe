<script setup lang="ts">
import { restoredStateNotice } from '~/utils/current-state'
</script>
<template>
  <aside v-if="restoredStateNotice" class="notice warning restore-banner" role="status"><strong>자료가 복원되었습니다.</strong><p>이전에 열어 둔 화면과 입력은 복원 전 자료일 수 있습니다. 입력과 응답 미확인 요청은 유지했습니다. 현재 자료와 비교하고 다시 검토해 주세요.</p><NuxtLink to="/overview">현재 작업 시작 화면 열기</NuxtLink> · <NuxtLink to="/restore#retained-inputs">보존한 입력·요청 확인</NuxtLink></aside>
</template>
