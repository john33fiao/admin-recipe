<script setup lang="ts">
import type { CatalogInput, MaterialDetail, PriceInput } from '~/types/material'
import { materialKinds, normalizeCatalog, validateCatalogInput, validatePriceInput } from '~/utils/catalog'

const props = defineProps<{ material: MaterialDetail; refresh: () => Promise<MaterialDetail | null> }>()
const connection = useConnection()
const { edit, price, pending, saving, reviewing, failure, saved, locked, canRetry, canReview, draftRestored, draftFailure, send, review } = useMaterialEditor(props.material, props.refresh)
const available = computed(() => connection.value === 'connected')
const editErrors = ref<Partial<Record<keyof CatalogInput, string>>>({})
const priceErrors = ref<Partial<Record<keyof PriceInput, string>>>({})
const editForm = ref<HTMLFormElement | null>(null)
const priceForm = ref<HTMLFormElement | null>(null)
const archiveReview = ref(false)

async function focusError(form: HTMLFormElement | null) { await nextTick(); form?.querySelector<HTMLElement>('[aria-invalid="true"]')?.focus() }
async function saveEdit() {
  editErrors.value = validateCatalogInput(normalizeCatalog(edit.value))
  if (Object.keys(editErrors.value).length) { await focusError(editForm.value); return }
  if (await send('version')) editForm.value?.querySelector<HTMLInputElement>('input')?.focus()
}
async function savePrice() {
  priceErrors.value = validatePriceInput(price.value)
  if (Object.keys(priceErrors.value).length) { await focusError(priceForm.value); return }
  if (await send('price')) priceForm.value?.querySelector<HTMLInputElement>('input')?.focus()
}
async function reviewInput() { if (await review()) { await nextTick(); editForm.value?.querySelector<HTMLInputElement>('input')?.focus() } }
</script>

<template>
  <section class="catalog-editor" aria-label="재료 정보와 사전 가격 입력" :aria-busy="saving || reviewing">
    <p v-if="draftRestored && !pending" class="draft-note" role="status">이 탭에서 작성하던 변경 내용을 불러왔습니다.</p>
    <p v-if="draftFailure" class="notice warning" role="alert">{{ draftFailure }}</p>
    <div v-if="pending && !failure" class="notice warning" role="status">{{ saving ? '저장을 확인하고 있습니다.' : '완료를 확인하지 못한 변경이 있습니다. 같은 요청으로 재시도해 주세요.' }}</div>
    <div v-if="failure" class="notice warning" role="alert"><p>{{ failure }}</p><button v-if="canReview" class="text-button" type="button" :disabled="!available || saving || reviewing" @click="reviewInput">{{ reviewing ? '최신 상태 확인 중…' : '최신 상태 확인 후 입력 검토' }}</button></div>
    <button v-if="canRetry && !saving" class="secondary-button retry-mutation" type="button" :disabled="!available" @click="send(pending!.action)">같은 변경 요청 재시도</button>
    <p v-if="saved" class="notice success" role="status">{{ saved }}</p>
    <div class="editor-columns">
      <form ref="editForm" class="editor-form" novalidate aria-labelledby="edit-material-title" @submit.prevent="saveEdit">
        <h2 id="edit-material-title">재료 정보 수정</h2>
        <div class="field"><label for="edit-name">재료 이름</label><input id="edit-name" v-model="edit.name" :readonly="locked" :disabled="!available" :aria-invalid="!!editErrors.name" :aria-describedby="editErrors.name ? 'edit-name-error' : undefined" @input="editErrors.name = undefined"><p v-if="editErrors.name" id="edit-name-error" class="field-error">{{ editErrors.name }}</p></div>
        <div class="field"><label for="edit-unit">관리 단위</label><input id="edit-unit" v-model="edit.unit" :readonly="locked" :disabled="!available" :aria-invalid="!!editErrors.unit" aria-describedby="edit-unit-hint edit-unit-error" @input="editErrors.unit = undefined"><p id="edit-unit-hint" class="field-hint">g↔kg, mL↔L은 표시를 바꿉니다. 질량↔부피 등 다른 차원은 새 재료로 등록하세요.</p><p id="edit-unit-error" class="field-error">{{ editErrors.unit }}</p></div>
        <div class="field"><label for="edit-kind">재료 종류</label><select id="edit-kind" v-model="edit.kind" :disabled="!available || locked"><option v-for="(label, value) in materialKinds" :key="value" :value="value">{{ label }}</option></select></div>
        <div class="field"><label for="edit-description">설명</label><textarea id="edit-description" v-model="edit.description" rows="4" :readonly="locked" :disabled="!available" :aria-invalid="!!editErrors.description" :aria-describedby="editErrors.description ? 'edit-description-error' : undefined" @input="editErrors.description = undefined"/><p v-if="editErrors.description" id="edit-description-error" class="field-error">{{ editErrors.description }}</p></div>
        <button class="primary-button" type="submit" :disabled="!available || locked">재료 정보 저장</button>
      </form>
      <form ref="priceForm" class="editor-form" novalidate aria-labelledby="price-input-title" @submit.prevent="savePrice">
        <h2 id="price-input-title">사전 가격 등록</h2>
        <p class="field-hint price-explanation">입고 전 원가 계산에 쓸 가격입니다. 가격 등록으로 보유 수량이나 금액이 늘어나지 않습니다. 변경하면 이전 가격을 이력에 남깁니다.</p>
        <div class="field"><label for="price-amount">기준 금액 (원)</label><input id="price-amount" v-model="price.amount" inputmode="decimal" placeholder="0원도 등록할 수 있습니다" :readonly="locked" :disabled="!available" :aria-invalid="!!priceErrors.amount" :aria-describedby="priceErrors.amount ? 'price-amount-error' : undefined" @input="priceErrors.amount = undefined"><p v-if="priceErrors.amount" id="price-amount-error" class="field-error">{{ priceErrors.amount }}</p></div>
        <div class="field"><label for="price-quantity">가격 기준 수량</label><input id="price-quantity" v-model="price.quantity" inputmode="decimal" :readonly="locked" :disabled="!available" :aria-invalid="!!priceErrors.quantity" :aria-describedby="priceErrors.quantity ? 'price-quantity-error' : 'price-quantity-hint'" @input="priceErrors.quantity = undefined"><p id="price-quantity-hint" class="field-hint">예: 100g 가격이면 100. 실제 보유량을 입력하는 칸이 아닙니다.</p><p v-if="priceErrors.quantity" id="price-quantity-error" class="field-error">{{ priceErrors.quantity }}</p></div>
        <div class="field"><label for="price-unit">가격 기준 단위</label><input id="price-unit" v-model="price.unit" :readonly="locked" :disabled="!available" :aria-invalid="!!priceErrors.unit" :aria-describedby="priceErrors.unit ? 'price-unit-error' : undefined" @input="priceErrors.unit = undefined"><p v-if="priceErrors.unit" id="price-unit-error" class="field-error">{{ priceErrors.unit }}</p></div>
        <button class="primary-button" type="submit" :disabled="!available || locked">가격 등록</button>
      </form>
    </div>
    <section class="archive-section" aria-labelledby="archive-title">
      <h2 id="archive-title">재료 보관</h2>
      <p v-if="material.archived">보관한 재료입니다. 기본 목록에서는 숨기며 정보와 가격·변경 이력은 계속 확인하고 수정할 수 있습니다.</p>
      <template v-else><p>사용하지 않는 재료를 기본 목록에서 숨깁니다. 이전 정보와 가격은 그대로 남습니다.</p><button v-if="!archiveReview" class="secondary-button" type="button" :disabled="!available || locked" @click="archiveReview = true">보관하기</button><div v-else class="notice warning"><p>이 재료를 사용 중 목록에서 제외할까요? 작성 중인 정보 수정은 함께 저장되지 않습니다.</p><div class="button-row"><button class="secondary-button" type="button" :disabled="!available || locked" @click="send('archive')">보관 확정</button><button class="text-button" type="button" :disabled="locked" @click="archiveReview = false">취소</button></div></div></template>
    </section>
  </section>
</template>
