<script setup lang="ts">
import type { CatalogInput } from '~/types/material'
import { formatCreatedAt } from '~/utils/material'
import { materialKinds, normalizeCatalog, priceText, validateCatalogInput } from '~/utils/catalog'

const { name, unit, kind, description, pending, saving, reviewing, failure, saved, rejected, recovery, draftRestored, draftFailure, save, editRejected } = useMaterialRegistration()
const { query, archive, materials, loaded, loading, loadFailure, available, loadMaterials } = useMaterialList()
const errors = ref<Partial<Record<keyof CatalogInput, string>>>({})
const nameInput = ref<HTMLInputElement | null>(null)
const unitInput = ref<HTMLInputElement | null>(null)
const descriptionInput = ref<HTMLTextAreaElement | null>(null)

async function editInput() {
  await editRejected()
  await nextTick()
  nameInput.value?.focus()
}

async function submit() {
  if (!available.value || saving.value || reviewing.value) return
  errors.value = validateCatalogInput(normalizeCatalog({ name: name.value, unit: unit.value, kind: kind.value, description: description.value }))
  if (errors.value.name) { nameInput.value?.focus(); return }
  if (errors.value.unit) { unitInput.value?.focus(); return }
  if (errors.value.description) { descriptionInput.value?.focus(); return }
  const material = await save()
  if (material) {
    nameInput.value?.focus()
    void loadMaterials()
  }
}

useHead({ title: '재료 관리 | 순창한상' })
</script>

<template>
  <div class="page-heading">
    <div><h1>재료 관리</h1><p>주방에서 사용하는 재료와 관리 단위를 기록하세요.</p></div>
    <span v-if="loaded" class="count-label">조회한 재료 <strong>{{ materials.length }}</strong></span>
  </div>

  <div class="workspace-grid">
    <section class="registration-panel" aria-labelledby="register-title">
      <div class="section-heading"><h2 id="register-title">새 재료 등록</h2><p>이름과 단위부터 차근차근.</p></div>
      <p v-if="draftRestored && !pending" class="draft-note" role="status">이 탭에서 작성하던 내용을 불러왔습니다.</p>
      <p v-if="draftFailure" class="notice warning" role="alert">{{ draftFailure }}</p>
      <form novalidate :aria-busy="saving || reviewing" @submit.prevent="submit">
        <div class="field">
          <label for="material-name">재료 이름 <span class="required-label">필수</span></label>
          <input id="material-name" ref="nameInput" v-model="name" name="name" autocomplete="off" placeholder="예: 무, 간장, 김" :readonly="!!pending || saving" :disabled="!available" :aria-invalid="!!errors.name" :aria-describedby="errors.name ? 'name-error' : undefined" @input="errors.name = undefined">
          <p v-if="errors.name" id="name-error" class="field-error">{{ errors.name }}</p>
        </div>
        <div class="field">
          <label for="material-unit">관리 단위 <span class="required-label">필수</span></label>
          <input id="material-unit" ref="unitInput" v-model="unit" name="unit" list="material-unit-options" autocomplete="off" :readonly="!!pending || saving" :disabled="!available" :aria-invalid="!!errors.unit" :aria-describedby="errors.unit ? 'unit-error unit-hint' : 'unit-hint'" @input="errors.unit = undefined">
          <datalist id="material-unit-options"><option value="g"/><option value="kg"/><option value="mL"/><option value="L"/><option value="개"/><option value="장"/></datalist>
          <p id="unit-hint" class="field-hint">g·kg, mL·L 또는 개, 장 등 원하는 단위를 입력하세요.</p>
          <p v-if="errors.unit" id="unit-error" class="field-error">{{ errors.unit }}</p>
        </div>
        <div class="field">
          <label for="material-kind">재료 종류</label>
          <select id="material-kind" v-model="kind" :disabled="!available || !!pending || saving">
            <option v-for="(label, value) in materialKinds" v-show="value !== 'unclassified' || kind === 'unclassified'" :key="value" :value="value">{{ label }}</option>
          </select>
        </div>
        <div class="field">
          <label for="material-description">설명 <span class="required-label">선택</span></label>
          <textarea id="material-description" ref="descriptionInput" v-model="description" rows="3" :readonly="!!pending || saving" :disabled="!available" :aria-invalid="!!errors.description" :aria-describedby="errors.description ? 'description-error' : undefined" @input="errors.description = undefined" />
          <p v-if="errors.description" id="description-error" class="field-error">{{ errors.description }}</p>
        </div>
        <div v-if="pending && !failure && !saving" class="notice warning" role="status">완료를 확인하지 못한 등록이 있습니다. 같은 내용으로 재시도하면 중복으로 등록되지 않습니다.</div>
        <div v-if="failure" class="notice warning" role="alert">
          <p>{{ failure }}</p>
          <button v-if="recovery === 'edit' || recovery === 'refresh'" class="text-button" type="button" :disabled="!available || saving || reviewing" @click="editInput">{{ reviewing ? '최신 상태 확인 중…' : recovery === 'refresh' ? '최신 상태 확인 후 입력 검토' : '입력 내용 수정' }}</button>
        </div>
        <button class="primary-button" type="submit" :disabled="!available || saving || reviewing || rejected">
          {{ saving ? '등록 확인 중…' : pending ? '같은 요청 재시도' : '재료 등록' }}
        </button>
        <p v-if="!available" class="field-hint">앱에 연결되면 등록할 수 있습니다.</p>
        <p class="field-hint">작성 중인 내용은 이 탭에서 보관합니다. 재료 등록 전에는 저장된 재료에 포함되지 않습니다.</p>
      </form>
      <div v-if="saved" class="notice success" role="status">
        <p><strong>{{ saved.name }}</strong> 재료를 등록했습니다.</p>
        <NuxtLink :to="`/materials/${saved.id}`">등록한 재료 보기</NuxtLink>
      </div>
    </section>

    <section class="material-panel" aria-labelledby="materials-title" :aria-busy="loading">
      <div class="list-heading"><h2 id="materials-title">등록한 재료</h2><button class="text-button" type="button" :disabled="!available || loading" @click="loadMaterials">{{ loading ? '불러오는 중…' : '새로고침' }}</button></div>
      <form class="catalog-search" role="search" @submit.prevent="loadMaterials">
        <div class="field"><label for="material-search">이름으로 찾기</label><input id="material-search" v-model="query" type="search" placeholder="이름의 일부로 검색" :disabled="!available"></div>
        <div class="field"><label for="archive-filter">조회 범위</label><select id="archive-filter" v-model="archive" :disabled="!available" @change="loadMaterials"><option value="active">사용 중</option><option value="archived">보관한 재료</option><option value="all">전체</option></select></div>
        <button class="secondary-button" type="submit" :disabled="!available || loading">검색</button>
      </form>
      <div v-if="loadFailure" class="notice warning" role="alert">{{ loadFailure }}</div>
      <div v-if="!loaded" class="empty-state"><div class="empty-symbol" aria-hidden="true">○</div><h3>{{ loading ? '재료를 불러오고 있습니다' : '연결을 기다리고 있습니다' }}</h3><p>{{ loadFailure ? '위의 새로고침을 눌러 다시 확인하세요.' : '연결되면 저장된 재료를 보여드릴게요.' }}</p></div>
      <div v-else-if="materials.length === 0" class="empty-state"><h3>{{ query.trim() || archive !== 'active' ? '조건에 맞는 재료가 없습니다' : '첫 재료를 기록해 보세요' }}</h3><p>{{ query.trim() || archive !== 'active' ? '이름이나 조회 범위를 바꿔 다시 검색해 주세요.' : '이름과 관리 단위를 등록하면 여기서 바로 확인할 수 있습니다.' }}</p></div>
      <template v-else>
        <p v-if="!available || loadFailure" class="stale-note">마지막으로 불러온 목록입니다. 연결되면 다시 확인합니다.</p>
        <ul class="material-list">
          <li v-for="material in materials" :key="material.id">
            <NuxtLink v-if="available" class="material-row" :to="`/materials/${material.id}`">
              <span class="material-initial" aria-hidden="true">{{ [...material.name][0] }}</span>
              <span class="material-copy"><strong>{{ material.name }} <span v-if="material.archived" class="status-label">보관됨</span></strong><span>{{ materialKinds[material.kind] }} · ID {{ material.id.slice(0, 8) }}</span><span>{{ priceText(material.price, material.canonical_unit) }}</span><span>{{ formatCreatedAt(material.created_at) }} 등록</span></span>
              <span class="unit-tag">{{ material.unit }}</span><span class="row-chevron" aria-hidden="true">›</span>
            </NuxtLink>
            <div v-else class="material-row"><span class="material-initial" aria-hidden="true">{{ [...material.name][0] }}</span><span class="material-copy"><strong>{{ material.name }}</strong><span>{{ formatCreatedAt(material.created_at) }} 등록</span></span><span class="unit-tag">{{ material.unit }}</span></div>
          </li>
        </ul>
      </template>
    </section>
  </div>
</template>
