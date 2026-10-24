<script setup lang="ts">
defineProps<{ error: { statusCode?: number } }>()
</script>

<template>
  <main class="app-shell error-page"><h1>{{ error.statusCode === 404 ? '페이지를 찾을 수 없습니다' : '화면을 불러오지 못했습니다' }}</h1><p>앱이 실행 중인지 확인한 뒤 재료 목록을 다시 열어 주세요.</p><a class="primary-button" href="/">재료 목록 열기</a></main>
</template>
