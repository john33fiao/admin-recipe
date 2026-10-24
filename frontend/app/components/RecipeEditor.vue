<script setup lang="ts">
import type { RecipeEditorInitial } from '~/composables/useRecipeEditor'
import type { Material, MaterialDetail } from '~/types/material'
import type { RecipeDetail, RecipeLineInput } from '~/types/recipe'
import { apiRequest } from '~/utils/api'
import { materialKinds } from '~/utils/catalog'
import { validateRecipe, validateRecipeOwner } from '~/utils/recipe'
const props = defineProps<{ identity: string; initial: RecipeEditorInitial; refresh: () => Promise<RecipeDetail | null> }>()
const editor = useRecipeEditor(props.identity, props.initial, props.refresh)
const { owner, data, head, busy, pending, locked, dirty, canRetry, canReview, failure, notice, storageFailure, result, created, send, review, synchronize, resetNew } = editor
watch(() => props.initial, synchronize, { immediate: true })
const connection = useConnection()
const materials = ref<Material[]>([])
const readFailure = ref('')
const errors = ref<Record<string, string>>({})
const form = ref<HTMLFormElement | null>(null)
const checking = ref<string[]>([])
const rowKeys = new WeakMap<object, string>()
const newOwnerKinds = { raw: materialKinds.raw, prepared: materialKinds.prepared, stocked: materialKinds.stocked, menu: materialKinds.menu }
let active = true, readGeneration = 0
onScopeDispose(() => { active = false; readGeneration += 1 })
const selectedOwner = computed(() => owner.value.mode === 'existing' ? owner.value.material_id : null)
const ownerChoices = computed(() => materials.value.filter(item => !item.archived || item.id === selectedOwner.value))
function choices(line: RecipeLineInput) { return materials.value.filter(item => !item.archived || item.id === line.material_id) }
function rowKey(line: RecipeLineInput) { if (!rowKeys.has(line)) rowKeys.set(line, crypto.randomUUID()); return rowKeys.get(line)! }
function oldName(line: RecipeLineInput) { return head.value?.lines.find(old => old.material_id === line.material_id && old.material_version_id === line.material_version_id)?.material_name }
async function loadMaterials() {
  if (connection.value !== 'connected') return
  const token = ++readGeneration
  try { const values = await apiRequest<Material[]>('/api/materials?archive=all'); if (active && token === readGeneration) { materials.value = values; readFailure.value = '' } }
  catch { if (active && token === readGeneration) readFailure.value = '구성 재료 목록을 불러오지 못했습니다. 저장한 배합은 유지합니다.' }
}
watch(connection, () => { void loadMaterials() }, { immediate: true })
function chooseOwner(event: Event) { const item = materials.value.find(value => value.id === (event.target as HTMLSelectElement).value); owner.value = { mode: 'existing', material_id: item?.id ?? '', expected_material_version_id: item?.version_id ?? '' } }
function changeMode(event: Event) { if ((event.target as HTMLSelectElement).value === 'new') owner.value = { mode: 'new', name: '', unit: '개', kind: 'menu', description: '' }; else owner.value = { mode: 'existing', material_id: '', expected_material_version_id: '' } }
function chooseMaterial(line: RecipeLineInput, event: Event) { const item = materials.value.find(value => value.id === (event.target as HTMLSelectElement).value); line.material_id = item?.id ?? ''; line.material_version_id = item?.version_id ?? ''; if (item) line.unit = item.unit }
function setReference(event: Event) { data.value.reference_output = (event.target as HTMLInputElement).checked ? { quantity: '', unit: owner.value.mode === 'new' ? owner.value.unit : materials.value.find(item => item.id === selectedOwner.value)?.unit ?? '' } : null }
async function addLine() { if (locked.value || data.value.lines.length >= 100) return; data.value.lines.push({ material_id: '', material_version_id: '', quantity: '0', unit: 'g', note: '' }); await nextTick(); form.value?.querySelector<HTMLElement>(`#recipe-line-${data.value.lines.length - 1}-material`)?.focus() }
async function removeLine(index: number) { if (locked.value) return; data.value.lines.splice(index, 1); await nextTick(); (form.value?.querySelector<HTMLElement>(`#recipe-line-${Math.min(index, data.value.lines.length - 1)}-material`) ?? form.value?.querySelector<HTMLElement>('#recipe-add-line'))?.focus() }
async function applyCurrent(line: RecipeLineInput) {
  const key = rowKey(line), captured = JSON.stringify(line)
  if (locked.value || checking.value.includes(key) || !line.material_id || connection.value !== 'connected') return
  checking.value.push(key)
  try {
    const material = await apiRequest<MaterialDetail>(`/api/materials/${encodeURIComponent(line.material_id)}`)
    if (!active || locked.value || !data.value.lines.includes(line) || JSON.stringify(line) !== captured) return
    line.material_version_id = material.version_id
    materials.value = [material, ...materials.value.filter(value => value.id !== material.id)]
    readFailure.value = ''
  } catch { if (active) readFailure.value = '현재 재료 정보를 불러오지 못했습니다. 기존 참조를 유지합니다.' }
  finally { checking.value = checking.value.filter(value => value !== key) }
}
async function save() {
  errors.value = validateRecipe(data.value)
  if (!props.initial.materialId) {
    for (const [key, value] of Object.entries(validateRecipeOwner(owner.value))) errors.value[`owner_${key}`] = value
  }
  if (Object.keys(errors.value).length) { await nextTick(); form.value?.querySelector<HTMLElement>(`#recipe-${Object.keys(errors.value)[0]}`)?.focus(); return }
  await send(props.initial.materialId ? 'revise' : 'create')
}
</script>
<template>
  <section class="recipe-editor history-section" aria-label="레시피 편집" :aria-busy="busy">
    <p v-if="storageFailure" class="notice warning" role="alert">{{ storageFailure }}</p>
    <div v-if="failure" class="notice warning" role="alert"><p>{{ failure }}</p><button v-if="canReview" class="text-button" type="button" :disabled="busy || connection !== 'connected'" @click="review">최신 저장본 확인 후 입력 검토</button></div>
    <p v-if="pending && !failure" class="notice warning" role="status">{{ busy ? '레시피 처리 결과를 확인하고 있습니다.' : '완료를 확인하지 못한 요청이 있습니다. 같은 요청으로 재시도해 주세요.' }}</p>
    <button v-if="canRetry && !busy" class="secondary-button" type="button" :disabled="connection !== 'connected'" @click="send(pending!.action)">같은 레시피 요청 재시도</button>
    <div v-if="notice" class="notice success" role="status"><p>{{ notice }}</p><NuxtLink v-if="result && (created || result.item.id !== initial.materialId)" :to="`/recipes/${result.item.id}`">{{ created ? '저장한 레시피 보기' : '복제한 레시피 보기' }}</NuxtLink></div>
    <button v-if="created" class="secondary-button" type="button" @click="resetNew">새 레시피 작성</button>
    <p v-if="readFailure" class="notice warning" role="alert">{{ readFailure }} <button type="button" class="text-button" @click="loadMaterials">재료 다시 불러오기</button></p>
    <form ref="form" novalidate @submit.prevent="save">
      <fieldset :disabled="locked" class="recipe-fieldset">
        <legend class="section-title">{{ initial.materialId ? '배합 수정' : '새 레시피 입력' }}</legend>
        <template v-if="!initial.materialId">
          <div class="field"><label for="recipe-mode">레시피 품목</label><select id="recipe-mode" :value="owner.mode" @change="changeMode"><option value="new">새 품목과 함께 만들기</option><option value="existing">기존 품목에 연결</option></select></div>
          <template v-if="owner.mode === 'new'">
            <div class="form-row"><div class="field"><label for="recipe-owner_name">레시피 이름</label><input id="recipe-owner_name" v-model="owner.name" :aria-invalid="!!errors.owner_name" aria-describedby="recipe-owner-name-error"><p id="recipe-owner-name-error" class="field-error">{{ errors.owner_name }}</p></div><div class="field"><label for="recipe-owner_unit">품목 관리 단위</label><input id="recipe-owner_unit" v-model="owner.unit" :aria-invalid="!!errors.owner_unit"><p class="field-error">{{ errors.owner_unit }}</p></div></div>
            <div class="field"><label for="recipe-owner_kind">품목 종류</label><select id="recipe-owner_kind" v-model="owner.kind" :aria-invalid="!!errors.owner_kind"><option v-for="(label, kind) in newOwnerKinds" :key="kind" :value="kind">{{ label }}</option></select><p class="field-error">{{ errors.owner_kind }}</p></div>
            <div class="field"><label for="recipe-owner_description">품목 설명 (선택)</label><textarea id="recipe-owner_description" v-model="owner.description" rows="2" :aria-invalid="!!errors.owner_description" /><p class="field-error">{{ errors.owner_description }}</p></div>
          </template>
          <div v-else class="field"><label for="recipe-owner_material">연결할 품목</label><select id="recipe-owner_material" :value="owner.material_id" :aria-invalid="!!errors.owner_material" @change="chooseOwner"><option value="">품목 선택</option><option v-for="item in ownerChoices" :key="item.id" :value="item.id">{{ item.name }} · {{ materialKinds[item.kind] }} · {{ item.unit }} · {{ item.id.slice(0, 8) }}{{ item.archived ? ' · 보관됨' : '' }}</option></select><p class="field-error">{{ errors.owner_material }}</p></div>
        </template>
        <div class="form-row"><div class="field"><label for="recipe-basis_count">기준 개수</label><input id="recipe-basis_count" v-model="data.basis_count" inputmode="numeric" :aria-invalid="!!errors.basis_count" aria-describedby="recipe-basis-hint"><p id="recipe-basis-hint" class="field-hint">아래 수량은 이 개수 전체의 배합입니다. 3개분 100g을 나누어 잘라 저장하지 않습니다.</p><p class="field-error">{{ errors.basis_count }}</p></div><div class="field"><label for="recipe-selling_price">판매가 (원, 선택)</label><input id="recipe-selling_price" :value="data.selling_price ?? ''" inputmode="decimal" :aria-invalid="!!errors.selling_price" @input="data.selling_price = ($event.target as HTMLInputElement).value || null"><p class="field-error">{{ errors.selling_price }}</p></div></div>
        <label class="recipe-check"><input type="checkbox" :checked="!!data.reference_output" @change="setReference">참고 생산량 기록</label>
        <div v-if="data.reference_output" class="form-row"><div class="field"><label for="recipe-reference_quantity">참고 생산량</label><input id="recipe-reference_quantity" v-model="data.reference_output.quantity" inputmode="decimal" :aria-invalid="!!errors.reference_quantity"><p class="field-error">{{ errors.reference_quantity }}</p></div><div class="field"><label for="recipe-reference_unit">참고 생산 단위</label><input id="recipe-reference_unit" v-model="data.reference_output.unit" :aria-invalid="!!errors.reference_unit"><p class="field-error">{{ errors.reference_unit }}</p></div></div>
        <p v-if="data.reference_output" class="field-hint">기준 개수 1의 배합 참고량입니다. 실제 생산량·수율·현재 재고를 만들지 않습니다.</p>
        <div class="section-heading"><h3>직접 구성 재료</h3><p>수량은 0부터 적을 수 있습니다. 적당량 등의 메모를 남기고 나중에 수량을 바꿀 수 있습니다.</p></div>
        <p v-if="!data.lines.length" class="stale-note">구성 재료가 없어도 레시피를 저장할 수 있습니다.</p>
        <fieldset v-for="(line, index) in data.lines" :key="rowKey(line)" class="recipe-line"><legend>구성 {{ index + 1 }}</legend>
          <div class="field"><label :for="`recipe-line-${index}-material`">구성 {{ index + 1 }} 재료</label><select :id="`recipe-line-${index}-material`" :value="line.material_id" :aria-invalid="!!errors[`line-${index}-material`]" @change="chooseMaterial(line, $event)"><option value="">재료 선택</option><option v-for="item in choices(line)" :key="item.id" :value="item.id">{{ item.name }} · {{ materialKinds[item.kind] }} · {{ item.unit }} · {{ item.id.slice(0, 8) }}{{ item.archived ? ' · 보관됨' : '' }}</option></select><p class="field-error">{{ errors[`line-${index}-material`] }}</p><p v-if="oldName(line)" class="field-hint">저장 당시 이름: {{ oldName(line) }}</p><div v-if="line.material_id" class="recipe-row-links"><NuxtLink :to="`/materials/${line.material_id}`">현재 재료 보기</NuxtLink><button type="button" class="text-button" :disabled="checking.includes(rowKey(line)) || connection !== 'connected'" @click="applyCurrent(line)">구성 {{ index + 1 }} 현재 재료 정보 적용</button></div></div>
          <div class="form-row"><div class="field"><label :for="`recipe-line-${index}-quantity`">구성 {{ index + 1 }} 기준 전체 수량</label><input :id="`recipe-line-${index}-quantity`" v-model="line.quantity" inputmode="decimal" :aria-invalid="!!errors[`line-${index}-quantity`]"><p class="field-error">{{ errors[`line-${index}-quantity`] }}</p></div><div class="field"><label :for="`recipe-line-${index}-unit`">구성 {{ index + 1 }} 단위</label><input :id="`recipe-line-${index}-unit`" v-model="line.unit" :aria-invalid="!!errors[`line-${index}-unit`]"><p class="field-error">{{ errors[`line-${index}-unit`] }}</p></div></div>
          <div class="field"><label :for="`recipe-line-${index}-note`">구성 {{ index + 1 }} 메모 (선택)</label><textarea :id="`recipe-line-${index}-note`" v-model="line.note" rows="2" :aria-invalid="!!errors[`line-${index}-note`]"><!-- value via v-model --></textarea><p class="field-error">{{ errors[`line-${index}-note`] }}</p></div><button type="button" class="text-button" @click="removeLine(index)">구성 {{ index + 1 }} 삭제</button>
        </fieldset>
        <p v-if="errors.lines" class="field-error">{{ errors.lines }}</p><button id="recipe-add-line" class="secondary-button" type="button" :disabled="data.lines.length >= 100" @click="addLine">구성 재료 추가</button>
        <div class="recipe-notes"><div class="field"><label for="recipe-cooking_description">조리 설명 (선택)</label><textarea id="recipe-cooking_description" v-model="data.cooking_description" rows="3" :aria-invalid="!!errors.cooking_description" /><p class="field-error">{{ errors.cooking_description }}</p></div><div class="field"><label for="recipe-process">조리 공정 (선택)</label><textarea id="recipe-process" v-model="data.process" rows="4" :aria-invalid="!!errors.process" /><p class="field-error">{{ errors.process }}</p></div><div class="field"><label for="recipe-reference_time">참고 조리 시간 (선택)</label><input id="recipe-reference_time" v-model="data.reference_time" :aria-invalid="!!errors.reference_time"><p class="field-error">{{ errors.reference_time }}</p></div></div>
        <p v-if="dirty" class="draft-note">저장하지 않은 배합 변경이 있습니다. 아래 원가는 저장된 구성 기준입니다.</p>
        <button class="primary-button" type="submit" :disabled="connection !== 'connected'">{{ initial.materialId ? '레시피 수정 저장' : '레시피 저장' }}</button>
      </fieldset>
    </form>
    <div v-if="initial.materialId" class="recipe-copy"><h3>독립 복제</h3><p>현재 저장된 배합을 새 품목으로 복제합니다. 작성 중인 변경은 복제하지 않습니다.</p><button class="secondary-button" type="button" :disabled="locked || connection !== 'connected'" @click="send('copy')">현재 저장본 독립 복제</button></div>
  </section>
</template>
