<script setup lang="ts">
import type { CatalogState } from '~/types/material'
import type { OverviewScope } from '~/types/overview'
import { overviewSections, overviewListFrom, overviewListUrl } from '~/utils/overview'
import { formatCreatedAt } from '~/utils/material'
const { snapshot, loading, failure, connection, oldResult, load } = useOperationsOverview()
const handoff = useState<{ url: string; state: CatalogState } | null>('overview-list-handoff', () => null)
const zone = Intl.DateTimeFormat().resolvedOptions().timeZone
const sections = computed(() => snapshot.value ? overviewSections.map(section => overviewListFrom(snapshot.value!.report, section)) : [])
function openList(scope: OverviewScope, offset: number) { if (snapshot.value) handoff.value = { url: overviewListUrl(scope, offset), state: { ...snapshot.value.report.state } } }
useHead({ title: '작업 시작 | 순창한상' })
</script>
<template>
  <div class="page-heading"><div><h1>작업 시작</h1><p>마지막으로 확인한 수량과 그 뒤의 기록을 살펴보세요.</p></div><button class="text-button" type="button" :disabled="loading || connection !== 'connected'" @click="load">{{ loading ? '불러오는 중…' : '전체 새로고침' }}</button></div>
  <nav class="overview-actions" aria-label="기록 시작"><NuxtLink class="primary-button" to="/workspace">모아 입력</NuxtLink><NuxtLink to="/business?kind=sale">판매 기록 시작</NuxtLink><NuxtLink to="/business?kind=stocktake">실사 시작</NuxtLink><NuxtLink to="/business?kind=purchase_order">발주 시작</NuxtLink><NuxtLink to="/business?kind=receipt">입고 기록 시작</NuxtLink><NuxtLink to="/statistics">통계·이력 열기</NuxtLink></nav>
  <div v-if="failure" class="notice warning" role="alert">{{ failure }}</div>
  <p v-if="!snapshot" class="empty-state" role="status">{{ loading ? '확인한 수량과 기록을 불러오고 있습니다.' : '연결을 확인한 뒤 전체 새로고침해 주세요.' }}</p>
  <template v-else>
    <div class="overview-read-note" role="status"><strong>{{ oldResult ? '이전에 조회한 자료입니다.' : '이 조회의 자료입니다.' }}</strong><p>조회 시각 {{ formatCreatedAt(snapshot.receivedAt) }} · 상세 화면에서는 이후 변경이 보일 수 있습니다.</p><p v-if="oldResult">새 건수와 이전 행을 섞지 않고 보존합니다. 연결을 확인한 뒤 전체 새로고침해 주세요.</p></div>
    <div class="overview-summary"><div><p>전체 품목(보관 포함)</p><strong class="overview-count">{{ snapshot.report.materials.total_count }}개</strong></div><NuxtLink class="primary-link" to="/">{{ snapshot.report.materials.total_count ? '재료 관리 열기' : '첫 재료 등록하기' }}</NuxtLink></div>
    <OverviewSection v-for="section in sections" :key="section.section" :value="section" :zone="zone" :expanded="section.section === 'materials'" @open-list="openList" />
    <p class="field-hint">주문 상태: 미입고 {{ snapshot.report.order_counts.open }}건 · 입고 완료 {{ snapshot.report.order_counts.received }}건 · 취소 {{ snapshot.report.order_counts.cancelled }}건. 취소 원기록은 <NuxtLink to="/orders">발주 내역</NuxtLink>에서 확인할 수 있습니다.</p>
    <p><NuxtLink to="/workspace">브라우저 미저장 입력·응답 미확인 요청 확인</NuxtLink></p>
  </template>
</template>
