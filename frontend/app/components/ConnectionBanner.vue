<script setup lang="ts">
const connection = useConnection()
const text = computed(() => ({
  connecting: '앱에 연결하고 있습니다.',
  connected: '이 PC에 연결됨',
  reconnecting: '연결이 끊겨 다시 연결하고 있습니다. 입력 내용은 유지됩니다.',
  unavailable: '앱에 연결할 수 없습니다. 앱이 종료되었다면 실행파일을 다시 열어 주세요.',
})[connection.value])
</script>

<template>
  <div class="connection-banner" :class="{ 'connection-alert': connection !== 'connected' }" role="status" aria-live="polite">
    <span class="connection-dot" :class="{ 'is-connected': connection === 'connected' }" aria-hidden="true" />
    <span>{{ text }}</span>
    <button v-if="connection === 'unavailable' || connection === 'reconnecting'" class="text-button" type="button" @click="retryConnection">다시 연결</button>
  </div>
</template>
