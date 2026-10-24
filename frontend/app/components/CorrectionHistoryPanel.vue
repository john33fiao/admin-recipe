<script setup lang="ts">
import type { CorrectionSeed } from '~/types/correction'
import { apiRequest } from '~/utils/api'
import { formatCreatedAt } from '~/utils/material'
const emit = defineEmits<{ loaded: [seed: CorrectionSeed] }>()
const props = defineProps<{ confirmationId: string }>()
const seed = ref<CorrectionSeed | null>(null)
const expanded = ref(false)
const failure = ref('')
let active = true, generation = 0
onScopeDispose(() => { active = false; generation += 1 })
async function load() {
  const current = ++generation
  try { const result = await apiRequest<CorrectionSeed>(`/api/business/confirmations/${encodeURIComponent(props.confirmationId)}/correction-seed`); if (active && current === generation) { seed.value = result; failure.value = ''; emit('loaded', result) } }
  catch { if (active && current === generation) failure.value = '현재 유효한 기록과 정정 이력을 불러오지 못했습니다.' }
}
watch(() => props.confirmationId, () => { seed.value = null; void load() }, { immediate: true })
</script>
<template>
  <section class="correction-history history-section"><h2>현재 유효한 기록과 정정</h2><p v-if="failure" class="notice warning">{{ failure }} <button class="text-button" type="button" @click="load">다시 불러오기</button></p><template v-if="seed"><p v-if="seed.current.reference.version_id !== seed.original.reference.version_id" class="notice">원본 이후의 정정이 있습니다. 아래 내용이 현재 유효한 기록이며, 이 페이지의 원본 입력은 그대로 남아 있습니다.</p><p v-else class="field-hint">원본 확정이 현재 유효합니다.</p><p v-if="seed.current.value === null" class="notice warning">이 기록의 유효 효과가 취소되었습니다. 원기록은 보존됩니다.</p><button v-else class="text-button" type="button" :aria-expanded="expanded" @click="expanded = !expanded">{{ expanded ? '현재 유효한 내용 접기' : '현재 유효한 내용 펼치기' }}</button><CorrectionEventValue v-if="expanded" :value="seed.current.value"/><NuxtLink class="secondary-link" :to="{ path: '/business', query: { kind: 'correction', corrects: seed.original.reference.original_confirmation_id } }">이 기록 정정하기</NuxtLink><ol v-if="seed.history.length" class="history-list"><li v-for="entry in seed.history" :key="entry.confirmation.id"><NuxtLink :to="entry.confirmation.result.kind === 'correction' ? `/corrections/${entry.confirmation.result.correction_id}` : `/business/${entry.confirmation.record_id}`">{{ entry.confirmation.id === seed.original.reference.confirmation_id ? '원본 확정' : '정정 확정' }}</NuxtLink><span v-if="entry.confirmation.id === seed.current.reference.confirmation_id" class="status-label">현재 유효</span><p>{{ formatCreatedAt(entry.confirmation.recorded_at) }} · 실제 기록 {{ entry.confirmation.seq }}</p><p class="preserve-lines">{{ entry.reason }}</p></li></ol></template>
  </section>
</template>
