<script setup lang="ts">
import type { CorrectionSeed } from '~/types/correction'
import { apiRequest } from '~/utils/api'
const props = defineProps<{ confirmationId: string; revision: string }>()
const seed = ref<CorrectionSeed | null>(null), failure = ref('')
const connection = useConnection()
let active = true, generation = 0
onScopeDispose(() => { active = false; generation += 1 })
async function load() {
  const current = ++generation
  if (connection.value !== 'connected') return
  try { const value = await apiRequest<CorrectionSeed>(`/api/business/confirmations/${encodeURIComponent(props.confirmationId)}/correction-seed`); if (active && current === generation) { seed.value = value; failure.value = '' } }
  catch { if (active && current === generation) failure.value = '현재 확정 결과를 불러오지 못했습니다. 원기록에서 다시 확인할 수 있습니다.' }
}
watch([connection, () => props.confirmationId, () => props.revision], () => { seed.value = null; void load() }, { immediate: true })
</script>
<template>
  <section class="workspace-confirmed-result" aria-label="현재 확정 결과"><h2>현재 확정 결과</h2><p v-if="failure" class="notice warning" role="alert">{{ failure }} <button type="button" class="text-button" @click="load">결과 다시 불러오기</button></p><p v-else-if="!seed" class="field-hint">확정 결과를 확인하고 있습니다.</p><template v-if="seed"><p v-if="seed.current.value === null" class="notice warning">이 기록의 유효 효과가 취소되었습니다. 원기록은 보존됩니다.</p><CorrectionEventValue v-else :value="seed.current.value"/><p v-if="seed.current.reference.version_id !== seed.original.reference.version_id" class="field-hint">원본 이후의 정정을 반영한 현재 유효 결과입니다.</p></template></section>
</template>
