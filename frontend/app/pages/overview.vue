<script setup lang="ts">
const { materials, loaded, loading, loadFailure, available, loadMaterials } = useMaterials()
useHead({ title: '작업 시작 | 순창한상' })
</script>

<template>
  <div class="page-heading">
    <div><h1>작업 시작</h1><p>저장한 재료를 확인하고 주방의 기록을 이어가세요.</p></div>
  </div>
  <section class="overview-panel" aria-labelledby="overview-materials-title" :aria-busy="loading">
    <div class="list-heading">
      <h2 id="overview-materials-title">사용 중인 재료</h2>
      <button class="text-button" type="button" :disabled="!available || loading" @click="loadMaterials">{{ loading ? '불러오는 중…' : '새로고침' }}</button>
    </div>
    <div v-if="loadFailure" class="notice warning" role="alert">{{ loadFailure }}</div>
    <div v-if="!loaded" class="empty-state" role="status">
      <h3>{{ loading ? '기록을 불러오고 있습니다' : '연결을 기다리고 있습니다' }}</h3>
      <p>{{ loadFailure ? '새로고침을 눌러 다시 확인하세요.' : '앱에 연결되면 저장한 재료 수를 보여드립니다.' }}</p>
    </div>
    <template v-else>
      <div class="overview-summary">
        <strong class="overview-count">{{ materials.length }}개</strong>
        <p>{{ materials.length ? '재료 목록에서 이름과 관리 단위를 확인할 수 있습니다.' : '첫 재료의 이름과 관리 단위를 등록해 보세요.' }}</p>
      </div>
      <p v-if="!available || loadFailure" class="stale-note">마지막으로 불러온 기록입니다. 연결을 확인한 뒤 새로고침해 주세요.</p>
      <NuxtLink class="primary-button overview-action" to="/">{{ materials.length ? '재료 관리 열기' : '첫 재료 등록하기' }}</NuxtLink>
    </template>
  </section>
</template>
