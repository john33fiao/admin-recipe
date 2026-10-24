<script setup lang="ts">
import type { Material } from '~/types/material'
import { apiRequest } from '~/utils/api'
const row = defineModel<{ material_id: string | null; material_version_id: string | null }>({ required: true })
const props = defineProps<{ id: string; label: string; disabled: boolean; field: string }>()
const materials = ref<Material[]>([])
const failure = ref('')
let active = true
onScopeDispose(() => { active = false })
onMounted(async () => { try { const result = await apiRequest<Material[]>('/api/materials?archive=all'); if (active) materials.value = result } catch { if (active) failure.value = '품목 목록을 불러오지 못했습니다. 저장한 참조는 유지했습니다.' } })
function select(event: Event) {
  if (props.disabled) return
  const item = materials.value.find(item => item.id === (event.target as HTMLSelectElement).value)
  row.value.material_id = item?.id ?? null
  row.value.material_version_id = item?.version_id ?? null
}
function current() { if (props.disabled) return; const item = materials.value.find(item => item.id === row.value.material_id); if (item) row.value.material_version_id = item.version_id }
</script>
<template>
  <div class="field"><label :for="id">{{ label }}</label><select :id="id" :data-field="field" :value="row.material_id ?? ''" :disabled="disabled" @change="select"><option value="">품목 선택</option><option v-if="row.material_id && !materials.some(item => item.id === row.material_id)" :value="row.material_id">저장한 품목 · {{ row.material_id.slice(0, 8) }}</option><option v-for="item in materials.filter(item => !item.archived || item.id === row.material_id)" :key="item.id" :value="item.id">{{ item.name }}{{ item.archived ? ' (보관됨)' : '' }}</option></select><p v-if="failure" class="field-error">{{ failure }}</p><p v-if="row.material_version_id" class="field-hint">참조 버전 {{ row.material_version_id.slice(0, 8) }} · 과거 참조를 유지합니다.</p><button v-if="row.material_id" class="text-button" type="button" :disabled="disabled" @click="current">현재 품목 버전 적용</button></div>
</template>
