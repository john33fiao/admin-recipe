<script setup lang="ts">
import type { CatalogState } from '~/types/material'
import type { OverviewTarget } from '~/types/overview'
import { overviewTarget, overviewListUrl, overviewLabels, overviewDescriptions, overviewPageText, sameOverviewScope } from '~/utils/overview'
import { formatCreatedAt } from '~/utils/material'
const route = useRoute(), router = useRouter(), target = shallowRef<OverviewTarget | null>(null), addressFailure = ref('')
watch(() => route.query, query => { try { target.value = overviewTarget(query); addressFailure.value = '' } catch (cause) { target.value = null; addressFailure.value = cause instanceof Error ? cause.message : '목록 주소를 확인해 주세요.' } }, { immediate: true, flush: 'sync' })
const handoff = useState<{ url: string; state: CatalogState } | null>('overview-list-handoff', () => null)
const initialState = target.value && handoff.value?.url === overviewListUrl(target.value, target.value.offset) ? handoff.value.state : null
handoff.value = null
const { snapshot, loading, failure, stale, connection, oldResult, load, restart } = useOverviewList(target, initialState)
const zone = Intl.DateTimeFormat().resolvedOptions().timeZone
const heading = ref<HTMLElement | null>(null)
async function refresh() { if (await restart() && target.value) { await router.replace(overviewListUrl(target.value, 0)); await nextTick(); heading.value?.focus() } }
watch(snapshot, async (value, previous) => {
  if (!value || (previous && sameOverviewScope(value.response.scope, previous.response.scope) && value.response.list.page.offset === previous.response.list.page.offset)) return
  await nextTick(); heading.value?.focus()
})
const title = computed(() => target.value ? overviewLabels[target.value.section] : '전체 목록')
useHead({ title: computed(() => `${title.value} | 순창한상`) })
</script>
<template>
  <NuxtLink class="back-link" to="/overview">작업 시작으로</NuxtLink>
  <div class="page-heading"><div><h1 ref="heading" tabindex="-1">{{ title }}</h1><p>같은 조회의 기록을 페이지별로 봅니다. 새 진입·새로고침에서는 최신 자료를 읽습니다.</p></div><button class="text-button" type="button" :disabled="!target || loading || connection !== 'connected'" @click="refresh">전체 다시 조회하고 첫 페이지로</button></div>
  <p v-if="addressFailure" class="notice warning" role="alert">{{ addressFailure }}</p><p v-if="failure" class="notice warning" role="alert">{{ failure }}</p>
  <p v-if="loading" role="status">목록을 불러오고 있습니다.</p>
  <div v-if="snapshot" :data-overview-section="snapshot.response.list.section">
    <div class="overview-read-note" role="status"><strong>{{ oldResult ? '이전에 조회한 목록입니다.' : '이 조회의 목록입니다.' }}</strong><p>{{ overviewLabels[snapshot.response.list.section] }} · 조회 시각 {{ formatCreatedAt(snapshot.receivedAt) }}</p><p v-if="oldResult">표시된 건수와 행은 함께 보존했습니다. 최신 자료와 섞지 않습니다.</p></div>
    <p class="field-hint">{{ overviewDescriptions[snapshot.response.list.section] }}</p><p class="overview-page-count">{{ overviewPageText(snapshot.response.list.page) }}</p>
    <OverviewRows :value="snapshot.response.list" :zone="zone" />
    <nav class="overview-pagination" aria-label="목록 페이지">
      <NuxtLink v-if="!loading && !stale && target && snapshot.response.list.page.offset > 0" :to="overviewListUrl(snapshot.response.scope, Math.max(0, snapshot.response.list.page.offset - snapshot.response.list.page.page_size))">이전 페이지</NuxtLink>
      <NuxtLink v-if="!loading && !stale && target && snapshot.response.list.page.next_offset !== null" :to="overviewListUrl(snapshot.response.scope, snapshot.response.list.page.next_offset)">다음 페이지</NuxtLink>
      <button v-if="failure && !stale" class="text-button" type="button" :disabled="loading || connection !== 'connected'" @click="load">같은 목록 다시 조회</button>
    </nav>
  </div>
  <p v-else-if="!loading && !failure && !addressFailure" class="empty-state">연결을 확인한 뒤 전체 다시 조회해 주세요.</p>
</template>
