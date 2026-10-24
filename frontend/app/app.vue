<script setup lang="ts">
const dark = ref(false)
const route = useRoute()
watch(() => route.path, async () => {
  await nextTick()
  document.getElementById('main-content')?.focus({ preventScroll: true })
  window.scrollTo(0, 0)
})
onMounted(() => {
  let preference: string | null = null
  try { preference = localStorage.getItem('sunchang.theme') } catch { /* Use system appearance. */ }
  dark.value = preference ? preference === 'dark' : window.matchMedia('(prefers-color-scheme: dark)').matches
  applyTheme()
})
function applyTheme() {
  document.documentElement.dataset.theme = dark.value ? 'dark' : 'light'
}
function toggleTheme() {
  dark.value = !dark.value
  applyTheme()
  try { localStorage.setItem('sunchang.theme', dark.value ? 'dark' : 'light') } catch { /* Theme still applies to this page. */ }
}
</script>

<template>
  <a class="skip-link" href="#main-content">본문으로 건너뛰기</a>
  <div class="app-shell">
    <header class="app-header">
      <NuxtLink class="brand" to="/" aria-label="순창한상 재료 관리 홈">
        <svg class="brand-mark" viewBox="0 0 40 40" fill="none" aria-hidden="true"><path d="M5 17h30c-1 11-7 16-15 16S6 28 5 17Z" stroke="currentColor" stroke-width="2"/><path d="M9 22h22M15 7c-3 3 3 3 0 6m10-6c-3 3 3 3 0 6" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>
        <span>순창한상<span class="brand-subtitle">주방의 재료 기록</span></span>
      </NuxtLink>
      <button class="theme-button" type="button" :aria-label="dark ? '라이트 모드로 전환' : '다크 모드로 전환'" :aria-pressed="dark" @click="toggleTheme">
        <svg v-if="dark" viewBox="0 0 24 24" fill="none" aria-hidden="true"><circle cx="12" cy="12" r="4" stroke="currentColor" stroke-width="1.7"/><path d="M12 2v2m0 16v2M2 12h2m16 0h2M5 5l1.5 1.5m11 11L19 19M5 19l1.5-1.5m11-11L19 5" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/></svg>
        <svg v-else viewBox="0 0 24 24" fill="none" aria-hidden="true"><path d="M20.5 14A8.5 8.5 0 0 1 10 3.5 8.5 8.5 0 1 0 20.5 14Z" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round"/></svg>
        <span>{{ dark ? '밝게' : '어둡게' }}</span>
      </button>
    </header>
    <nav class="app-navigation" aria-label="주요 메뉴">
      <NuxtLink to="/overview" :aria-current="route.path === '/overview' ? 'page' : undefined">작업 시작</NuxtLink>
      <NuxtLink to="/" :aria-current="route.path === '/' || route.path.startsWith('/materials/') ? 'page' : undefined">재료 관리</NuxtLink>
      <NuxtLink to="/recipes" :aria-current="route.path.startsWith('/recipes') ? 'page' : undefined">레시피</NuxtLink>
      <NuxtLink to="/business" :aria-current="route.path === '/business' || route.path.startsWith('/business/') ? 'page' : undefined">업무 기록</NuxtLink>
      <NuxtLink to="/orders" :aria-current="route.path === '/orders' || route.path.startsWith('/orders/') ? 'page' : undefined">발주 내역</NuxtLink>
      <NuxtLink to="/receipts" :aria-current="route.path === '/receipts' || route.path.startsWith('/receipts/') ? 'page' : undefined">입고 기록</NuxtLink>
      <NuxtLink to="/purchase-returns" :aria-current="route.path.startsWith('/purchase-returns') ? 'page' : undefined">구입 반품</NuxtLink>
      <NuxtLink to="/productions" :aria-current="route.path === '/productions' || route.path.startsWith('/productions/') ? 'page' : undefined">생산 기록</NuxtLink>
      <NuxtLink to="/stocktakes" :aria-current="route.path.startsWith('/stocktakes') ? 'page' : undefined">실사</NuxtLink>
      <NuxtLink to="/wastes" :aria-current="route.path.startsWith('/wastes') ? 'page' : undefined">폐기 · 직원식</NuxtLink>
      <NuxtLink to="/sales" :aria-current="route.path === '/sales' || route.path.startsWith('/sales/') ? 'page' : undefined">판매 기록</NuxtLink>
    </nav>
    <ConnectionBanner />
    <main id="main-content" tabindex="-1"><NuxtPage /></main>
    <footer class="app-footer"><span>순창한상</span><span>이 PC의 앱에서 재료를 관리합니다.</span></footer>
  </div>
</template>
