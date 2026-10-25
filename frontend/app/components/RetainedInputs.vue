<script setup lang="ts">
const app = useNuxtApp()
const stored = ref<Array<{ key: string; raw: string }>>([])
const storageFailure = ref(false)
function load() {
  try {
    stored.value = Array.from({ length: sessionStorage.length }, (_, index) => sessionStorage.key(index)).filter((key): key is string => !!key && (/^sunchang\.(business|recipe|profile|material-editor)\..+\.(draft|pending)\.v1$/.test(key) || /^sunchang\.(pending-material|draft-material)\.v1$/.test(key) || /^sunchang\.retained\.material\.[0-9a-f-]{36}$/.test(key) || /^sunchang\.restore\.(pending\.v1|retained\.|reviewed\.)/.test(key) || /^sunchang\.bulk\.(session\.v1|retained\.[0-9a-f-]{36}\.v1)$/.test(key))).map(key => ({ key, raw: sessionStorage.getItem(key) ?? '' }))
    storageFailure.value = false
  } catch { storageFailure.value = true }
}
const rows = computed(() => {
  const values = new Map(stored.value.map(row => [row.key, { ...row, memory: false }]))
  // Read existing reactive state only; never initialize or clear another editor's pending ref.
  for (const [rawKey, value] of Object.entries(app.payload.state)) {
    const key = rawKey.replace(/^\$s/, '')
    if (value == null || !(/^sunchang\.(business|recipe|profile|material-editor)\..+\.(pending|payload|input|data|edit|price)$/.test(key) || /^(pending-material|material-name|material-unit|material-description)$/.test(key) || key === 'sunchang.bulk.session')) continue
    const storageKey = key === 'pending-material' ? 'sunchang.pending-material.v1' : key.endsWith('.pending') ? `${key}.v1` : `memory:${key}`
    try { values.set(storageKey, { key: storageKey, raw: JSON.stringify(value, null, 2), memory: true }) } catch { /* The editor itself remains available if a value is not serializable. */ }
  }
  return [...values.values()]
})
function label(key: string) { return key.includes('bulk.') ? '일괄 입력·원요청' : key.includes('restore.') ? '복원 원요청' : key.includes('business.') ? '업무 입력' : key.includes('recipe.') ? '레시피 입력' : key.includes('profile.') ? '사진 · 구입처 입력' : '재료 입력' }
function link(key: string) {
  if (key.includes('sunchang.bulk.')) return '/bulk'
  const match = key.match(/sunchang\.(business|recipe|profile|material-editor)\.([0-9a-f-]{36})\./i)
  return match ? `/${match[1] === 'business' ? 'business' : match[1] === 'recipe' ? 'recipes' : 'materials'}/${match[2]}` : null
}
onMounted(load)
</script>
<template>
  <section id="retained-inputs" class="restore-retained" aria-labelledby="retained-title"><h2 id="retained-title">보존한 입력·요청</h2><p>이 탭의 미저장 입력과 원요청입니다. 복원된 자료에 자동 적용하거나 삭제하지 않습니다. 현재 자료에서 사라진 상세의 입력도 여기에서 읽을 수 있습니다.</p><p class="field-hint">사진 파일 자체는 브라우저 메모리에만 있습니다. 다시 열었다면 사진 편집 화면에서 같은 파일을 재선택해 주세요.</p><p><a href="/overview" target="_blank" rel="noopener">새 탭에서 현재 자료로 작업 시작</a></p><p class="field-hint">새 탭에는 이 탭의 미저장 입력과 요청을 복사하지 않습니다. 원래 입력은 이 탭에 남습니다.</p><button class="text-button" type="button" @click="load">보존한 입력 다시 확인</button><p v-if="storageFailure" class="notice warning">탭 저장공간을 읽지 못했습니다. 현재 메모리에서 확인할 수 있는 입력만 표시합니다. 열려 있는 편집 화면을 닫지 마세요.</p><p v-if="!rows.length" class="empty-state">이 탭에서 확인할 보존 입력이 없습니다. 다른 탭의 입력은 해당 탭에서 확인해 주세요.</p><ol v-else class="history-list"><li v-for="(row, index) in rows" :key="row.key"><details><summary>{{ label(row.key) }} {{ index + 1 }} · {{ row.key.includes('pending') || row.key.includes('retained.') || row.key.includes('reviewed.') ? '원요청 보존' : '입력 보존' }}{{ row.memory ? ' · 현재 메모리' : '' }}</summary><p v-if="link(row.key)"><NuxtLink :to="link(row.key)!">현재 자료와 비교하기</NuxtLink></p><label :for="`retained-${index}`">보존한 원입력</label><textarea :id="`retained-${index}`" :value="row.raw" readonly rows="8" spellcheck="false"/></details></li></ol></section>
</template>
