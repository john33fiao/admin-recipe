<script setup lang="ts">
import type { BusinessDetail } from '~/types/business'
import type { BusinessEditorHost } from '~/types/workspace'
import type { BusinessEditorInitial } from '~/composables/useBusinessDraft'
import type { RecipeDetail, RecipeList } from '~/types/recipe'
import type { SaleOriginal, SalePayload } from '~/types/sale'
import { apiRequest } from '~/utils/api'
import { businessInputText, validateBusinessDraft } from '~/utils/business'
import { displayDecimal, materialKinds } from '~/utils/catalog'
const props = defineProps<{ identity: string; initial: BusinessEditorInitial<SalePayload>; refresh: () => Promise<BusinessDetail | null>; host?: BusinessEditorHost; frozen?: SaleOriginal | null }>()
const { payload, status, issues, preview, previewOutdated, awaitingPrior, busy, failure, notice, storageFailure, created, pending, dirty, locked, canRetry, canReview, send, showPreview, review, resetNew, synchronizeTerminal } = useBusinessDraft(props.identity, props.initial, props.refresh, props.host)
watch(() => props.initial, synchronizeTerminal, { immediate: true })
const connection = useConnection()
const available = computed(() => connection.value === 'connected')
const recipes = ref<RecipeList['items']>([])
const referenceFailure = ref('')
const checking = ref(false)
const errors = ref<Record<string, string>>({})
const form = ref<HTMLFormElement | null>(null)
const previewPanel = ref<HTMLElement | null>(null)
const archiveReview = ref(false)
const choices = computed(() => recipes.value.filter(entry => !entry.item.archived || entry.item.id === payload.value.data.material_id))
let active = true, generation = 0
onScopeDispose(() => { active = false; generation += 1 })
async function loadRecipes() {
  const current = ++generation
  if (!available.value || props.initial.corrects) return
  try { const value = await apiRequest<RecipeList>('/api/recipes?archive=all'); if (active && current === generation) { recipes.value = value.items; referenceFailure.value = '' } }
  catch { if (active && current === generation) referenceFailure.value = '판매할 메뉴의 레시피 목록을 불러오지 못했습니다.' }
}
watch(available, () => { void loadRecipes() }, { immediate: true })
function selectRecipe(event: Event) {
  if (locked.value || props.initial.corrects) return
  const entry = recipes.value.find(value => value.item.id === (event.target as HTMLSelectElement).value)
  payload.value.data.material_id = entry?.item.id ?? null
  payload.value.data.material_version_id = entry?.item.version_id ?? null
  payload.value.data.recipe_version_id = entry?.recipe_version_id ?? null
}
async function applyCurrent() {
  if (!available.value || locked.value || checking.value || props.initial.corrects || !payload.value.data.material_id) return
  const data = payload.value.data, fingerprint = JSON.stringify(data)
  checking.value = true
  try {
    const detail = await apiRequest<RecipeDetail>(`/api/recipes/${encodeURIComponent(data.material_id!)}`)
    if (!active || locked.value || props.initial.corrects || JSON.stringify(payload.value.data) !== fingerprint) return
    payload.value.data.material_version_id = detail.item.version_id
    payload.value.data.recipe_version_id = detail.head.id
    recipes.value = [{ item: detail.item, recipe_version_id: detail.head.id, updated_at: detail.head.recorded_at }, ...recipes.value.filter(entry => entry.item.id !== detail.item.id)]
    referenceFailure.value = ''
  } catch { if (active) referenceFailure.value = '현재 메뉴와 레시피를 확인하지 못했습니다. 저장된 입력은 유지했습니다.' }
  finally { checking.value = false }
}
function issue(field: string) { return errors.value[field] ?? issues.value.find(value => value.field === field || value.field === `data.${field}`)?.message }
async function focusIssue(client = false) { await nextTick(); const field = (client ? Object.keys(errors.value)[0] : issues.value[0]?.field)?.replace(/^data\./, ''); const id = field?.includes('version') ? 'sale-current-reference' : field === 'material_id' ? 'sale-material' : `sale-${field}`; const target = form.value?.querySelector<HTMLElement>(`#${id}`); if (target) target.focus(); else previewPanel.value?.focus() }
async function saveDraft() { errors.value = validateBusinessDraft(payload.value); if (Object.keys(errors.value).length) { await focusIssue(true); return }; await send(props.initial.recordId ? 'revise' : 'create') }
async function previewDraft() { if (await showPreview()) { await nextTick(); previewPanel.value?.focus() } else await focusIssue() }
</script>
<template>
  <section class="business-editor sale-editor" aria-label="판매 초안 입력" :aria-busy="busy">
    <p v-if="storageFailure" class="notice warning" role="alert">{{ storageFailure }}</p><div v-if="failure" class="notice warning" role="alert"><p>{{ failure }}</p><button v-if="canReview" class="text-button" type="button" :disabled="busy || !available" @click="review">최신 기록 확인 후 입력 검토</button></div><p v-if="pending && !failure" class="notice warning" role="status">{{ busy ? '처리 결과를 확인하고 있습니다.' : '완료를 확인하지 못한 전송 요청이 있습니다. 같은 요청으로 재시도해 주세요.' }}</p><button v-if="canRetry && !busy" class="secondary-button" type="button" :disabled="!available" @click="send(pending!.action)">같은 업무 요청 재시도</button><p v-if="notice" class="notice success" role="status">{{ notice }}</p>
    <NuxtLink v-if="created" class="business-saved-link" :to="`/business/${created.id}`">저장한 초안 열기</NuxtLink><button v-if="created && !initial.corrects" class="text-button" type="button" :disabled="busy" @click="resetNew">새 판매 초안 작성</button>
    <section v-if="status !== 'draft' && dirty" class="notice warning" aria-labelledby="sale-unsaved-title"><h2 id="sale-unsaved-title">이 화면에 남은 미저장 입력</h2><p>다른 화면에서 확정하거나 보관해 이 입력은 반영되지 않았습니다.</p><p>{{ businessInputText(payload) }}</p><p class="preserve-lines">{{ payload.data.note }}</p></section>
    <form v-if="status === 'draft' && !created" ref="form" novalidate @submit.prevent="saveDraft"><div class="section-heading"><h2>{{ initial.corrects ? '판매 개수 정정 초안' : '판매 초안' }}</h2><p>{{ initial.corrects ? '당시 메뉴·레시피·판매가·원가는 유지하고 개수와 정정 이유를 입력합니다.' : '판매한 메뉴와 개수를 입력하세요. 실제 재료 사용량과 판매 날짜는 입력하지 않습니다.' }}</p></div>
      <template v-if="initial.corrects"><h3>{{ frozen?.material_name ?? '정정 대상 판매' }}</h3><p>당시 레시피와 가격을 유지합니다. 현재 레시피로 과거 판매를 다시 계산하지 않습니다.</p><NuxtLink v-if="frozen" :to="`/sales/${frozen.id}`">정정 대상 판매 보기</NuxtLink></template>
      <div v-else class="field"><label for="sale-material">판매 메뉴</label><select id="sale-material" :value="payload.data.material_id ?? ''" :disabled="!available || locked" :aria-invalid="!!issue('material_id')" aria-describedby="sale-material-error" @change="selectRecipe"><option value="">레시피가 있는 메뉴 선택</option><option v-if="payload.data.material_id && !recipes.some(entry => entry.item.id === payload.data.material_id)" :value="payload.data.material_id">저장한 메뉴 참조 확인 필요</option><option v-for="entry in choices" :key="entry.item.id" :value="entry.item.id">{{ entry.item.name }} · {{ materialKinds[entry.item.kind] }}{{ entry.item.archived ? ' (보관됨)' : '' }} · {{ entry.item.id.slice(0, 8) }}</option></select><p id="sale-material-error" class="field-error">{{ issue('material_id') }}</p><p class="field-hint">판매할 품목에 레시피가 없다면 <NuxtLink to="/recipes/new">레시피를 등록하세요</NuxtLink>. 빈 구성이나 구성 수량 0도 사용할 수 있습니다.</p></div>
      <div v-if="payload.data.material_id && payload.data.recipe_version_id" class="sale-reference"><NuxtLink :to="`/recipes/${payload.data.material_id}/versions/${payload.data.recipe_version_id}`">참조한 레시피 버전 보기</NuxtLink><p class="material-id">레시피 {{ payload.data.recipe_version_id }}</p></div><button v-if="payload.data.material_id && !initial.corrects" id="sale-current-reference" class="text-button" type="button" :disabled="!available || locked || checking" @click="applyCurrent">현재 메뉴·레시피 정보 적용</button><p class="field-error">{{ issue('material_version_id') || issue('recipe_version_id') }}</p>
      <p v-if="referenceFailure" class="notice warning" role="alert">{{ referenceFailure }} <button class="text-button" type="button" :disabled="!available" @click="loadRecipes">메뉴 다시 불러오기</button></p>
      <div class="field"><label for="sale-count">판매 개수</label><input id="sale-count" v-model="payload.data.count" inputmode="numeric" :readonly="locked" :disabled="!available" :aria-invalid="!!issue('count')" aria-describedby="sale-count-hint sale-count-error"><p id="sale-count-hint" class="field-hint">0~1,000,000,000의 정수입니다. 0개도 기록할 수 있습니다.</p><p id="sale-count-error" class="field-error">{{ issue('count') }}</p></div>
      <div class="field"><label for="sale-note">{{ initial.corrects ? '판매 정정 이유' : '판매 메모 (선택)' }}</label><textarea id="sale-note" v-model="payload.data.note" rows="3" :readonly="locked" :disabled="!available" :aria-invalid="!!issue('note')" aria-describedby="sale-note-error"/><p id="sale-note-error" class="field-error">{{ issue('note') }}</p></div>
      <p v-if="dirty" class="draft-note" role="status">화면에서 바꾼 내용이 아직 임시 저장되지 않았습니다.</p><div class="button-row"><button class="primary-button" type="submit" :disabled="!available || locked">{{ initial.recordId ? '수정 내용 임시 저장' : '초안 임시 저장' }}</button><button v-if="initial.recordId" class="secondary-button" type="button" :disabled="!available || locked || dirty" @click="previewDraft">확정 전 미리보기</button></div><p class="field-hint">미완성 입력도 임시 저장할 수 있습니다. 판매 기록만으로 재고 수량과 보유 가치를 차감하지 않습니다.</p>
    </form>
    <p v-if="previewOutdated" class="notice warning" role="status">다른 기록이 바뀌었습니다. 확정 전 미리보기를 다시 확인해 주세요.</p>
    <p v-if="awaitingPrior" class="notice warning" role="status">다른 업무의 처리 결과를 먼저 확인한 뒤 실사를 미리보기해 주세요. 임시 저장과 입력은 계속할 수 있습니다.</p>
    <section v-if="preview" ref="previewPanel" class="business-preview sale-preview" tabindex="-1" aria-labelledby="sale-preview-title"><h2 id="sale-preview-title">{{ initial.corrects ? '판매 개수 정정 미리보기' : '판매 미리보기' }}</h2><ul v-if="preview.issues.length" class="business-issues"><li v-for="item in preview.issues" :key="`${item.field}-${item.code}`">{{ item.message }}</li></ul><template v-if="preview.effect?.kind === 'sale'"><h3>{{ preview.effect.sale.material_name }}</h3><dl class="business-price-comparison"><div v-if="preview.effect.previous"><dt>이전 유효 판매</dt><dd>{{ displayDecimal(preview.effect.previous.count) }}개 · {{ preview.effect.previous.amount === null ? '판매금액 미계산' : `${displayDecimal(preview.effect.previous.amount)}원` }}</dd></div><div><dt>{{ initial.corrects ? '정정 후 판매' : '판매 개수·금액' }}</dt><dd class="preview-result">{{ displayDecimal(preview.effect.sale.count) }}개 · {{ preview.effect.sale.amount === null ? '판매금액 미계산' : `${displayDecimal(preview.effect.sale.amount)}원` }}</dd></div></dl><SaleCostSnapshot :cost="preview.effect.sale.recipe_cost"/><p class="notice">실사 소모 미확정 · 판매 기록만으로 재고를 차감하지 않습니다.</p><p v-if="initial.corrects" class="field-hint">개수 정정은 고객 환불이나 실물 반환을 기록하지 않습니다.</p></template><button class="primary-button" type="button" :disabled="!available || busy || !!pending || dirty || previewOutdated || awaitingPrior || !preview.can_confirm" @click="send('confirm')">이 내용으로 확정</button></section>
    <section v-if="initial.recordId && status === 'draft'" class="archive-section" aria-label="초안 보관"><p>초안을 보관해도 확정한 판매 기록은 바뀌지 않습니다.</p><button v-if="!archiveReview" class="text-button" type="button" :disabled="!available || locked" @click="archiveReview = true">초안 보관</button><div v-else class="notice warning"><p>화면의 미저장 입력은 함께 저장되지 않습니다.</p><div class="button-row"><button class="secondary-button" type="button" :disabled="!available || locked" @click="send('archive')">초안 보관 확정</button><button class="text-button" type="button" :disabled="locked" @click="archiveReview = false">취소</button></div></div></section>
  </section>
</template>
