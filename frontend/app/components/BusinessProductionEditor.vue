<script setup lang="ts">
import type { BusinessDetail } from '~/types/business'
import type { BusinessEditorInitial } from '~/composables/useBusinessDraft'
import type { Material } from '~/types/material'
import type { DraftProductionInput, ProductionPayload } from '~/types/production'
import { apiRequest } from '~/utils/api'
import { validateBusinessDraft } from '~/utils/business'
import { blankProductionInput, blankProductionOutput } from '~/utils/production'
const props = defineProps<{ identity: string; initial: BusinessEditorInitial<ProductionPayload>; refresh: () => Promise<BusinessDetail | null> }>()
const { payload, status, issues, preview, busy, failure, notice, storageFailure, created, pending, dirty, locked, canRetry, canReview, send, showPreview, review, resetNew, synchronizeTerminal } = useBusinessDraft(props.identity, props.initial, props.refresh)
watch(() => props.initial, synchronizeTerminal, { immediate: true })
const connection = useConnection()
const available = computed(() => connection.value === 'connected')
const materials = ref<Material[]>([])
const referenceFailure = ref('')
const checking = ref<string | null>(null)
const errors = ref<Record<string, string>>({})
const form = ref<HTMLFormElement | null>(null)
const previewPanel = ref<HTMLElement | null>(null)
const archiveReview = ref(false)
const keys = new WeakMap<DraftProductionInput, string>()
let nextKey = 0
let active = true
let generation = 0
onScopeDispose(() => { active = false; generation += 1 })
function rowKey(row: DraftProductionInput) { if (!keys.has(row)) keys.set(row, `production-row-${++nextKey}`); return keys.get(row)! }
const sides = [{ key: 'inputs', label: '투입' }, { key: 'outputs', label: '결과' }] as const
function choices(row: DraftProductionInput) { return materials.value.filter(item => !item.archived || item.id === row.material_id) }
async function loadReferences() {
  const current = ++generation
  if (!available.value) return
  try { const value = await apiRequest<Material[]>('/api/materials?archive=all'); if (active && current === generation) { materials.value = value; referenceFailure.value = '' } }
  catch { if (active && current === generation) referenceFailure.value = '생산 품목을 불러오지 못했습니다. 다시 확인해 주세요.' }
}
watch(available, () => { void loadReferences() }, { immediate: true })
function selectMaterial(row: DraftProductionInput, event: Event) {
  if (locked.value) return
  const item = materials.value.find(value => value.id === (event.target as HTMLSelectElement).value)
  row.material_id = item?.id ?? null; row.material_version_id = item?.version_id ?? null
  if (item) row.unit = item.unit
}
async function applyCurrent(row: DraftProductionInput) {
  if (locked.value || checking.value || !available.value || !row.material_id) return
  const fingerprint = JSON.stringify(row)
  checking.value = rowKey(row)
  try {
    const material = await apiRequest<Material>(`/api/materials/${encodeURIComponent(row.material_id)}`)
    if (!active || locked.value || JSON.stringify(row) !== fingerprint || ![...payload.value.data.inputs, ...payload.value.data.outputs].includes(row)) return
    materials.value = [material, ...materials.value.filter(item => item.id !== material.id)]
    row.material_version_id = material.version_id
    referenceFailure.value = ''
  } catch { if (active) referenceFailure.value = '현재 품목 정보를 확인하지 못했습니다. 다시 시도해 주세요.' }
  finally { checking.value = null }
}
async function add(side: 'inputs' | 'outputs') {
  if (locked.value || !available.value || payload.value.data[side].length >= 100) return
  if (side === 'inputs') payload.value.data.inputs.push(blankProductionInput())
  else payload.value.data.outputs.push(blankProductionOutput(payload.value.data.outputs.length === 0))
  await nextTick(); form.value?.querySelector<HTMLElement>(`[data-side="${side}"]:last-of-type select`)?.focus()
}
async function remove(side: 'inputs' | 'outputs', index: number) {
  if (locked.value || !available.value) return
  payload.value.data[side].splice(index, 1)
  await nextTick(); form.value?.querySelector<HTMLElement>(`#production-add-${side}`)?.focus()
}
function issue(field: string) { return errors.value[field] ?? issues.value.find(item => item.field.replace(/\[(\d+)\]/g, '.$1').replace(/^data\./, '') === field)?.message }
async function focusIssue(client = false) {
  await nextTick()
  const field = (client ? Object.keys(errors.value)[0] : issues.value[0]?.field)?.replace(/\[(\d+)\]/g, '.$1').replace(/^data\./, '')
  const target = field ? form.value?.querySelector<HTMLElement>(`[data-field="${field}"]`) : null
  if (target) target.focus(); else previewPanel.value?.focus()
}
async function saveDraft() { errors.value = validateBusinessDraft(payload.value); if (Object.keys(errors.value).length) { await focusIssue(true); return }; await send(props.initial.recordId ? 'revise' : 'create') }
async function previewDraft() { if (await showPreview()) { await nextTick(); previewPanel.value?.focus() } else await focusIssue() }
</script>
<template>
  <section class="business-editor production-editor" aria-label="실제 생산 초안 입력" :aria-busy="busy">
    <p v-if="storageFailure" class="notice warning" role="alert">{{ storageFailure }}</p>
    <div v-if="failure" class="notice warning" role="alert"><p>{{ failure }}</p><button v-if="canReview" class="text-button" type="button" :disabled="busy || !available" @click="review">최신 기록 확인 후 입력 검토</button></div>
    <p v-if="pending && !failure" class="notice warning" role="status">{{ busy ? '처리 결과를 확인하고 있습니다.' : '완료를 확인하지 못한 전송 요청이 있습니다. 같은 요청으로 재시도해 주세요.' }}</p>
    <button v-if="canRetry && !busy" class="secondary-button" type="button" :disabled="!available" @click="send(pending!.action)">같은 업무 요청 재시도</button>
    <p v-if="notice" class="notice success" role="status">{{ notice }}</p>
    <NuxtLink v-if="created" class="business-saved-link" :to="`/business/${created.id}`">저장한 초안 열기</NuxtLink>
    <button v-if="created" class="text-button" type="button" :disabled="busy" @click="resetNew">새 생산 초안 작성</button>
    <section v-if="status !== 'draft' && dirty" class="notice warning" aria-labelledby="production-unsaved-title"><h2 id="production-unsaved-title">이 화면에 남은 미저장 입력</h2><p>다른 화면에서 확정하거나 보관해 이 입력은 반영되지 않았습니다.</p><ProductionDraftSummary :payload="payload"/><p class="preserve-lines">{{ payload.data.note }}</p></section>
    <form v-if="status === 'draft' && !created" ref="form" novalidate @submit.prevent="saveDraft">
      <div class="section-heading"><h2>실제 생산 초안</h2><p>사용한 양과 만들어진 양을 각각 입력하세요. 레시피 기준량과 수율로 실제 수량을 채우지 않습니다.</p></div>
      <p v-if="referenceFailure" class="notice warning" role="alert">{{ referenceFailure }} <button class="text-button" type="button" :disabled="!available" @click="loadReferences">품목 다시 불러오기</button></p>
      <section v-for="side in sides" :key="side.key" class="production-row-section" :aria-labelledby="`production-${side.key}-title`"><div class="list-heading"><h3 :id="`production-${side.key}-title`">실제 {{ side.label }} <span class="field-hint">{{ payload.data[side.key].length }} / 100행</span></h3></div>
        <p v-if="side.key === 'outputs'" class="field-hint">첫 결과는 원가 배분 100%, 추가 결과는 0%로 시작합니다. 실제 결과 수량과 별도로 배분을 조정하고 합계를 100%로 맞추세요.</p>
        <p v-if="!payload.data[side.key].length" class="empty-state">입력한 {{ side.label }} 행이 없습니다.</p>
        <fieldset v-for="(row, index) in payload.data[side.key]" :key="rowKey(row)" :data-side="side.key" class="production-row"><legend>{{ side.label }} {{ index + 1 }}</legend>
          <div class="field"><label :for="`${rowKey(row)}-material`">{{ side.label }} {{ index + 1 }} 품목</label><select :id="`${rowKey(row)}-material`" :data-field="`${side.key}.${index}.material_id`" :value="row.material_id ?? ''" :disabled="!available || locked" :aria-invalid="!!issue(`${side.key}.${index}.material_id`)" :aria-describedby="`${rowKey(row)}-material-error`" @change="selectMaterial(row, $event)"><option value="">품목 선택</option><option v-if="row.material_id && !materials.some(item => item.id === row.material_id)" :value="row.material_id">참조 품목 확인 필요</option><option v-for="item in choices(row)" :key="item.id" :value="item.id">{{ item.name }}{{ item.archived ? ' (보관됨)' : '' }} · {{ item.id.slice(0, 8) }}</option></select><p :id="`${rowKey(row)}-material-error`" class="field-error">{{ issue(`${side.key}.${index}.material_id`) }}</p></div>
          <div class="production-quantity-fields"><div class="field"><label :for="`${rowKey(row)}-quantity`">{{ side.label }} {{ index + 1 }} 실제 수량</label><input :id="`${rowKey(row)}-quantity`" v-model="row.quantity" :data-field="`${side.key}.${index}.quantity`" inputmode="decimal" :readonly="locked" :disabled="!available" :aria-invalid="!!issue(`${side.key}.${index}.quantity`)" :aria-describedby="`${rowKey(row)}-quantity-error`"><p :id="`${rowKey(row)}-quantity-error`" class="field-error">{{ issue(`${side.key}.${index}.quantity`) }}</p></div><div class="field"><label :for="`${rowKey(row)}-unit`">{{ side.label }} {{ index + 1 }} 단위</label><input :id="`${rowKey(row)}-unit`" v-model="row.unit" :data-field="`${side.key}.${index}.unit`" :readonly="locked" :disabled="!available" :aria-invalid="!!issue(`${side.key}.${index}.unit`)" :aria-describedby="`${rowKey(row)}-unit-error`"><p :id="`${rowKey(row)}-unit-error`" class="field-error">{{ issue(`${side.key}.${index}.unit`) }}</p></div></div>
          <div v-if="'allocation_percent' in row" class="field"><label :for="`${rowKey(row)}-allocation`">결과 {{ index + 1 }} 원가 배분 (%)</label><input :id="`${rowKey(row)}-allocation`" v-model="row.allocation_percent" :data-field="`outputs.${index}.allocation_percent`" inputmode="decimal" :readonly="locked" :disabled="!available" :aria-invalid="!!issue(`outputs.${index}.allocation_percent`)" :aria-describedby="`${rowKey(row)}-allocation-error`"><p :id="`${rowKey(row)}-allocation-error`" class="field-error">{{ issue(`outputs.${index}.allocation_percent`) }}</p></div>
          <div class="button-row"><button v-if="row.material_id" class="text-button" type="button" :data-field="`${side.key}.${index}.material_version_id`" :disabled="!available || locked || !!checking" @click="applyCurrent(row)">{{ side.label }} {{ index + 1 }} 현재 품목 정보 적용</button><button class="text-button" type="button" :disabled="!available || locked" @click="remove(side.key, index)">{{ side.label }} {{ index + 1 }} 행 삭제</button></div><p class="field-error">{{ issue(`${side.key}.${index}.material_version_id`) }}</p>
        </fieldset>
        <p class="field-error">{{ issue(side.key) }}</p><button :id="`production-add-${side.key}`" class="secondary-button" type="button" :disabled="!available || locked || payload.data[side.key].length >= 100" @click="add(side.key)">{{ side.label }} 행 추가</button>
      </section>
      <div class="field"><label for="production-note">생산 메모 (선택)</label><textarea id="production-note" v-model="payload.data.note" data-field="note" rows="3" :readonly="locked" :disabled="!available" :aria-invalid="!!issue('note')" aria-describedby="production-note-error"/><p id="production-note-error" class="field-error">{{ issue('note') }}</p></div>
      <p v-if="dirty" class="draft-note" role="status">화면에서 바꾼 내용이 아직 임시 저장되지 않았습니다.</p><div class="button-row"><button class="primary-button" type="submit" :disabled="!available || locked">{{ initial.recordId ? '수정 내용 임시 저장' : '초안 임시 저장' }}</button><button v-if="initial.recordId" class="secondary-button" type="button" :disabled="!available || locked || dirty" @click="previewDraft">확정 전 미리보기</button></div><p class="field-hint">미완성 입력도 임시 저장할 수 있습니다. 수량·원가 근거가 부족하면 입력을 유지한 채 확인이 필요한 이유를 안내합니다.</p>
    </form>
    <section v-if="preview" ref="previewPanel" class="business-preview production-preview" tabindex="-1" aria-labelledby="production-preview-title"><h2 id="production-preview-title">실제 생산 미리보기</h2><ul v-if="preview.issues.length" class="business-issues"><li v-for="item in preview.issues" :key="`${item.field}-${item.code}`">{{ item.message }}</li></ul><ProductionResult v-if="preview.effect?.kind === 'production'" :inputs="preview.effect.inputs" :outputs="preview.effect.outputs" :materials="preview.effect.materials" :total="preview.effect.total_input_value"/><p v-if="!preview.can_confirm" class="field-hint">수량이나 원가를 임의로 줄이거나 0으로 처리하지 않습니다. 필요한 근거를 확인한 뒤 다시 미리보기해 주세요.</p><button class="primary-button" type="button" :disabled="!available || busy || !!pending || dirty || !preview.can_confirm" @click="send('confirm')">이 내용으로 확정</button></section>
    <section v-if="initial.recordId && status === 'draft'" class="archive-section" aria-label="초안 보관"><p>초안을 보관해도 이미 확정된 생산과 정산 기준은 바뀌지 않습니다.</p><button v-if="!archiveReview" class="text-button" type="button" :disabled="!available || locked" @click="archiveReview = true">초안 보관</button><div v-else class="notice warning"><p>화면의 미저장 입력은 함께 저장되지 않습니다.</p><div class="button-row"><button class="secondary-button" type="button" :disabled="!available || locked" @click="send('archive')">초안 보관 확정</button><button class="text-button" type="button" :disabled="locked" @click="archiveReview = false">취소</button></div></div></section>
  </section>
</template>
