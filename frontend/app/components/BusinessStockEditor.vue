<script setup lang="ts">
import type { BusinessDetail } from '~/types/business'
import type { BusinessEditorInitial } from '~/composables/useBusinessDraft'
import type { Material } from '~/types/material'
import type { HoldingDetail } from '~/types/receipt'
import type { DraftStocktakeRow, StocktakePayload } from '~/types/stocktake'
import type { WastePayload } from '~/types/waste'
import { apiRequest } from '~/utils/api'
import { validateBusinessDraft } from '~/utils/business'
import { displayDecimal, priceSources, priceText } from '~/utils/catalog'
import { formatCreatedAt } from '~/utils/material'
import { blankStocktakeRow } from '~/utils/stocktake'
type StockPayload = StocktakePayload | WastePayload
type Row = DraftStocktakeRow | WastePayload['data']
const props = defineProps<{ identity: string; initial: BusinessEditorInitial<StockPayload>; refresh: () => Promise<BusinessDetail | null> }>()
const { payload, status, issues, preview, busy, failure, notice, storageFailure, created, pending, dirty, locked, canRetry, canReview, send, showPreview, review, resetNew, synchronizeTerminal } = useBusinessDraft(props.identity, props.initial, props.refresh)
watch(() => props.initial, synchronizeTerminal, { immediate: true })
const connection = useConnection(), available = computed(() => connection.value === 'connected')
const isStocktake = computed(() => payload.value.kind === 'stocktake')
const label = computed(() => isStocktake.value ? '실사' : '폐기 · 직원식')
const rows = computed<Row[]>(() => payload.value.kind === 'stocktake' ? payload.value.data.rows : [payload.value.data])
const materials = ref<Material[]>([]), holdings = ref<Record<string, HoldingDetail>>({})
const referenceFailure = ref(''), checking = ref<string | null>(null), errors = ref<Record<string, string>>({})
const form = ref<HTMLFormElement | null>(null), previewPanel = ref<HTMLElement | null>(null), archiveReview = ref(false)
const keys = new WeakMap<Row, string>(); let nextKey = 0, active = true, generation = 0, holdingGeneration = 0
onScopeDispose(() => { active = false; generation += 1; holdingGeneration += 1 })
function rowKey(row: Row) { if (!keys.has(row)) keys.set(row, `stock-row-${++nextKey}`); return keys.get(row)! }
function field(index: number, name: string) { return isStocktake.value ? `rows.${index}.${name}` : name }
function choices(row: Row) { return materials.value.filter(item => !item.archived || item.id === row.material_id) }
function holding(row: Row) { return row.material_id ? holdings.value[row.material_id] : undefined }
async function loadMaterials() {
  const current = ++generation
  if (!available.value) return
  try { const value = await apiRequest<Material[]>('/api/materials?archive=all'); if (active && current === generation) { materials.value = value; referenceFailure.value = '' } }
  catch { if (active && current === generation) referenceFailure.value = '품목을 불러오지 못했습니다. 다시 확인해 주세요.' }
}
async function loadHoldings() {
  const current = ++holdingGeneration
  if (!available.value) return
  const ids = [...new Set(rows.value.map(row => row.material_id).filter((id): id is string => !!id))]
  const result = await Promise.allSettled(ids.map(async id => [id, await apiRequest<HoldingDetail>(`/api/holdings/${encodeURIComponent(id)}`)] as const))
  if (!active || current !== holdingGeneration) return
  for (const value of result) if (value.status === 'fulfilled') holdings.value[value.value[0]] = value.value[1]
  if (result.some(value => value.status === 'rejected')) referenceFailure.value = '일부 보유 근거를 확인하지 못했습니다. 입력은 보존됩니다. 보유 근거를 다시 확인해 주세요.'
}
watch(available, () => { void loadMaterials() }, { immediate: true })
watch([available, () => rows.value.map(row => `${row.material_id}:${row.material_version_id}`).join('|'), () => props.initial.state.revision], () => { void loadHoldings() }, { immediate: true })
function selectMaterial(row: Row, event: Event) {
  if (locked.value) return
  const item = materials.value.find(value => value.id === (event.target as HTMLSelectElement).value)
  row.material_id = item?.id ?? null; row.material_version_id = item?.version_id ?? null
  if (item) row.unit = item.unit
}
async function applyCurrent(row: Row) {
  if (!active || locked.value || checking.value || !available.value || !row.material_id) return
  const fingerprint = JSON.stringify(row); checking.value = rowKey(row)
  try {
    const value = await apiRequest<Material>(`/api/materials/${encodeURIComponent(row.material_id)}`)
    if (!active || locked.value || JSON.stringify(row) !== fingerprint || !rows.value.includes(row)) return
    materials.value = [value, ...materials.value.filter(item => item.id !== value.id)]; row.material_version_id = value.version_id
  } catch { if (active) referenceFailure.value = '현재 품목 정보를 확인하지 못했습니다. 입력을 유지했습니다.' }
  finally { checking.value = null }
}
async function add(all = false) {
  if (payload.value.kind !== 'stocktake' || locked.value || !available.value) return
  const target = payload.value.data.rows
  if (all) {
    const missing = materials.value.filter(item => !item.archived && !target.some(row => row.material_id === item.id))
    const empty = target.filter(row => !row.material_id && !row.material_version_id && row.quantity.trim() === '' && !row.initial_value && !row.excess)
    if (target.length + Math.max(0, missing.length - empty.length) > 100) { referenceFailure.value = '일괄 추가하면 100행을 넘습니다. 품목을 나누어 추가해 주세요.'; return }
    for (const item of missing) { const row = empty.shift(); if (row) Object.assign(row, { material_id: item.id, material_version_id: item.version_id, unit: item.unit }); else target.push({ ...blankStocktakeRow(), material_id: item.id, material_version_id: item.version_id, unit: item.unit }) }
  } else if (target.length < 100) target.push(blankStocktakeRow())
  await nextTick(); form.value?.querySelector<HTMLElement>('fieldset:last-of-type select')?.focus()
}
async function remove(index: number) { if (payload.value.kind !== 'stocktake' || locked.value) return; payload.value.data.rows.splice(index, 1); await nextTick(); form.value?.querySelector<HTMLElement>('#stock-add')?.focus() }
function startExcess(row: DraftStocktakeRow) { if (!locked.value && !row.excess) row.excess = { reviewed: false, reason: '', value: null } }
function issue(name: string) { return errors.value[name] ?? issues.value.find(item => item.field.replace(/\[(\d+)\]/g, '.$1').replace(/^data\./, '') === name)?.message }
async function focusIssue(client = false) { await nextTick(); const name = (client ? Object.keys(errors.value)[0] : issues.value[0]?.field)?.replace(/\[(\d+)\]/g, '.$1').replace(/^data\./, ''); const target = name ? form.value?.querySelector<HTMLElement>(`[data-field="${name}"]`) : null; if (target) target.focus(); else previewPanel.value?.focus() }
async function saveDraft() { errors.value = validateBusinessDraft(payload.value); if (Object.keys(errors.value).length) { await focusIssue(true); return }; await send(props.initial.recordId ? 'revise' : 'create') }
async function previewDraft() { if (await showPreview()) { await nextTick(); previewPanel.value?.focus() } else await focusIssue() }
</script>
<template>
  <section class="business-editor stock-editor" :aria-label="`${label} 초안 입력`" :aria-busy="busy">
    <p v-if="storageFailure" class="notice warning" role="alert">{{ storageFailure }}</p><div v-if="failure" class="notice warning" role="alert"><p>{{ failure }}</p><button v-if="canReview" class="text-button" type="button" :disabled="busy || !available" @click="review">최신 기록 확인 후 입력 검토</button></div><p v-if="pending && !failure" class="notice warning" role="status">{{ busy ? '처리 결과를 확인하고 있습니다.' : '완료를 확인하지 못한 요청이 있습니다. 같은 요청으로 재시도해 주세요.' }}</p><button v-if="canRetry && !busy" class="secondary-button" type="button" :disabled="!available" @click="send(pending!.action)">같은 업무 요청 재시도</button><p v-if="notice" class="notice success" role="status">{{ notice }}</p><NuxtLink v-if="created" class="business-saved-link" :to="`/business/${created.id}`">저장한 초안 열기</NuxtLink><button v-if="created" class="text-button" type="button" :disabled="busy" @click="resetNew">새 {{ label }} 초안 작성</button>
    <section v-if="status !== 'draft' && dirty" class="notice warning"><h2>이 화면에 남은 미저장 입력</h2><p>다른 화면에서 확정하거나 보관해 이 입력은 반영되지 않았습니다.</p><StockDraftSummary :payload="payload"/><p class="preserve-lines">{{ payload.data.note }}</p></section>
    <form v-if="status === 'draft' && !created" ref="form" novalidate @submit.prevent="saveDraft">
      <div class="section-heading"><h2>{{ label }} 초안</h2><p v-if="isStocktake">직접 확인한 현재 수량을 입력하세요. 빈칸은 미확인, 0은 실제 수량 0입니다.</p><p v-else>실제로 폐기하거나 직원식에 사용한 양을 입력하세요. 판매 소모와 구분해 반영합니다.</p></div>
      <p v-if="referenceFailure" class="notice warning" role="alert">{{ referenceFailure }} <button class="text-button" type="button" :disabled="!available" @click="loadMaterials(); loadHoldings()">품목·보유 근거 다시 확인</button></p>
      <p v-if="isStocktake" class="field-hint">{{ rows.length }} / 100행 · 입력한 모든 행을 함께 확인하고 저장합니다.</p>
      <fieldset v-for="(row, index) in rows" :key="rowKey(row)" class="production-row stock-row"><legend>{{ isStocktake ? `실사 ${index + 1}` : '폐기 · 직원식' }}</legend>
        <div class="field"><label :for="`${rowKey(row)}-material`">{{ isStocktake ? `실사 ${index + 1} 품목` : '폐기 · 직원식 품목' }}</label><select :id="`${rowKey(row)}-material`" :data-field="field(index, 'material_id')" :value="row.material_id ?? ''" :disabled="locked || !available" :aria-invalid="!!issue(field(index, 'material_id'))" :aria-describedby="`${rowKey(row)}-material-error`" @change="selectMaterial(row, $event)"><option value="">품목 선택</option><option v-if="row.material_id && !materials.some(item => item.id === row.material_id)" :value="row.material_id">참조 품목 확인 필요</option><option v-for="item in choices(row)" :key="item.id" :value="item.id">{{ item.name }}{{ item.archived ? ' (보관됨)' : '' }} · {{ item.id.slice(0, 8) }}</option></select><p :id="`${rowKey(row)}-material-error`" class="field-error">{{ issue(field(index, 'material_id')) }}</p></div>
        <div class="production-quantity-fields"><div class="field"><label :for="`${rowKey(row)}-quantity`">{{ isStocktake ? `실사 ${index + 1} 실제 수량` : '폐기 · 직원식 실제 수량' }}</label><input :id="`${rowKey(row)}-quantity`" v-model="row.quantity" :data-field="field(index, 'quantity')" inputmode="decimal" :readonly="locked" :disabled="!available" :aria-invalid="!!issue(field(index, 'quantity'))" :aria-describedby="`${rowKey(row)}-quantity-error`"><p :id="`${rowKey(row)}-quantity-error`" class="field-error">{{ issue(field(index, 'quantity')) }}</p></div><div class="field"><label :for="`${rowKey(row)}-unit`">{{ isStocktake ? `실사 ${index + 1} 단위` : '폐기 · 직원식 단위' }}</label><input :id="`${rowKey(row)}-unit`" v-model="row.unit" :data-field="field(index, 'unit')" :readonly="locked" :disabled="!available"><p class="field-error">{{ issue(field(index, 'unit')) }}</p></div></div>
        <div v-if="holding(row)" class="stock-context"><p>마지막 확인 수량: {{ holding(row)!.observation ? `${displayDecimal(holding(row)!.observation!.quantity)}${holding(row)!.unit} · ${formatCreatedAt(holding(row)!.observation!.recorded_at)}` : '아직 확인한 수량 없음' }}</p><p>현재 정산 기준: {{ holding(row)!.settlement_basis ? `${displayDecimal(holding(row)!.settlement_basis!.quantity)}${holding(row)!.unit} · ${holding(row)!.settlement_basis!.value === null ? '가치 미확인' : displayDecimal(holding(row)!.settlement_basis!.value!) + '원'}` : '시작 기준 없음' }}</p><p class="field-hint">{{ priceSources[holding(row)!.price.source] }} · {{ priceText(holding(row)!.price, holding(row)!.unit) }}</p><details v-if="holding(row)!.confirmed_changes.length"><summary>기준 이후 확정 변동 {{ holding(row)!.confirmed_changes.length }}건</summary><NuxtLink :to="`/materials/${row.material_id}`">품목의 보유 근거에서 확인</NuxtLink></details><p class="field-hint">조회한 기준이며 새로 계량한 수량이 아닙니다. 확정 전 미리보기에서 최신 근거를 다시 확인합니다.</p></div>
        <template v-if="'initial_value' in row"><div v-if="(holding(row) && !holding(row)!.settlement_basis) || row.initial_value" class="stock-value-section"><h3>실제 시작 가치 확인</h3><StockValueInput v-model="row.initial_value" :id="`${rowKey(row)}-initial`" :label="`실사 ${index + 1} 시작 가치`" :disabled="locked || !available" :quantity="row.quantity" :existing-allowed="holding(row)?.price.source !== 'missing'" :field="field(index, 'initial_value')"/><p class="field-error">{{ issue(field(index, 'initial_value')) }}</p><p v-if="holding(row)?.settlement_basis && row.initial_value" class="notice warning">이미 정산 기준이 생겼습니다. 이전 시작 가치 입력을 보존했으니 최신 근거와 비교하고 확인 선택을 검토해 주세요.</p></div>
          <div v-if="holding(row)?.settlement_basis || row.excess" class="stock-value-section"><button v-if="!row.excess" :data-field="field(index, 'excess')" class="text-button" type="button" :disabled="locked || !available" @click="startExcess(row)">초과 수량이 있다면 원인 확인 입력</button><template v-else><h3 tabindex="-1" :data-field="field(index, 'excess')">초과 수량 원인 확인</h3><label class="stock-check"><input v-model="row.excess.reviewed" type="checkbox" :data-field="field(index, 'excess.reviewed')" :disabled="locked || !available">누락 입고·생산과 입력 오류를 확인했습니다</label><div class="field"><label :for="`${rowKey(row)}-reason`">실사 {{ index + 1 }} 초과 확인 이유</label><textarea :id="`${rowKey(row)}-reason`" v-model="row.excess.reason" :data-field="field(index, 'excess.reason')" rows="2" :readonly="locked" :disabled="!available"/><p class="field-error">{{ issue(field(index, 'excess.reason')) }}</p></div><p v-if="holding(row)?.price.source !== 'missing'" class="field-hint">적용 단가가 있으면 기존 단가를 사용합니다. 기존 입력은 자동으로 지우지 않으며 미리보기에서 적용 근거를 확인할 수 있습니다.</p><StockValueInput v-model="row.excess.value" :manual-allowed="holding(row)?.price.source === 'missing'" :existing-allowed="holding(row)?.price.source !== 'missing'" :id="`${rowKey(row)}-excess`" :label="`실사 ${index + 1} 초과분 가치`" :disabled="locked || !available" :field="field(index, 'excess.value')"/><button class="text-button" type="button" :disabled="locked || !available" @click="row.excess = null">초과 확인 입력 해제</button></template></div>
        </template>
        <div class="button-row"><button v-if="row.material_id" class="text-button" type="button" :data-field="field(index, 'material_version_id')" :disabled="locked || !available || !!checking" @click="applyCurrent(row)">{{ isStocktake ? `실사 ${index + 1} ` : '' }}현재 품목 정보 적용</button><button v-if="isStocktake" class="text-button" type="button" :disabled="locked || !available" @click="remove(index)">실사 {{ index + 1 }} 행 삭제</button></div><p class="field-error">{{ issue(field(index, 'material_version_id')) }}</p>
      </fieldset>
      <p class="field-error">{{ issue('rows') }}</p><div v-if="isStocktake" class="button-row"><button id="stock-add" class="secondary-button" type="button" :disabled="locked || !available || rows.length >= 100" @click="add()">실사 품목 추가</button><button class="text-button" type="button" :disabled="locked || !available || !materials.length" @click="add(true)">사용 중인 품목 일괄 추가</button></div>
      <div v-if="payload.kind === 'waste'" class="field"><label for="waste-reason">사용 구분</label><select id="waste-reason" v-model="payload.data.reason" :disabled="locked || !available"><option value="discard">명시 폐기</option><option value="staff_meal">직원식</option></select></div>
      <div class="field"><label for="stock-note">{{ label }} 메모 (선택)</label><textarea id="stock-note" v-model="payload.data.note" data-field="note" rows="3" :readonly="locked" :disabled="!available"/><p class="field-error">{{ issue('note') }}</p></div><p v-if="dirty" class="draft-note" role="status">화면에서 바꾼 내용이 아직 임시 저장되지 않았습니다.</p><div class="button-row"><button class="primary-button" type="submit" :disabled="locked || !available">{{ initial.recordId ? '수정 내용 임시 저장' : '초안 임시 저장' }}</button><button v-if="initial.recordId" class="secondary-button" type="button" :disabled="locked || !available || dirty" @click="previewDraft">확정 전 미리보기</button></div><p class="field-hint">미완성 입력도 임시 저장할 수 있습니다. 확인되지 않은 수량이나 가치를 0으로 채우지 않습니다.</p>
    </form>
    <section v-if="preview" ref="previewPanel" class="business-preview stocktake-preview" tabindex="-1" aria-labelledby="stock-preview-title"><h2 id="stock-preview-title">{{ label }} 미리보기</h2><ul v-if="preview.issues.length" class="business-issues"><li v-for="item in preview.issues" :key="`${item.field}-${item.code}`">{{ item.message }}</li></ul><StocktakeResult v-if="preview.effect?.kind === 'stocktake'" :rows="preview.effect.rows" :unchecked="preview.effect.unchecked_positions"/><WasteResult v-else-if="preview.effect?.kind === 'waste'" :value="preview.effect.waste"/><p v-if="!preview.can_confirm" class="field-hint">입력을 유지했습니다. 필요한 근거를 확인한 뒤 수정 내용을 저장하고 다시 미리보기해 주세요.</p><button class="primary-button" type="button" :disabled="!available || busy || !!pending || dirty || !preview.can_confirm" @click="send('confirm')">이 내용으로 확정</button></section>
    <section v-if="initial.recordId && status === 'draft'" class="archive-section"><button v-if="!archiveReview" class="text-button" type="button" :disabled="locked || !available" @click="archiveReview = true">초안 보관</button><div v-else class="notice warning"><p>화면의 미저장 입력은 함께 저장되지 않습니다. 확정된 기록은 바뀌지 않습니다.</p><div class="button-row"><button class="secondary-button" type="button" :disabled="locked || !available" @click="send('archive')">초안 보관 확정</button><button class="text-button" type="button" :disabled="locked" @click="archiveReview = false">취소</button></div></div></section>
  </section>
</template>
