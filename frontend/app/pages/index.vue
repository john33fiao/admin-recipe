<script setup lang="ts">
import type { Material, MaterialInput } from '~/types/material'
import { apiRequest } from '~/utils/api'
import { formatCreatedAt, normalizeMaterial, validateMaterial } from '~/utils/material'

const connection = useConnection()
const { name, unit, pending, saving, failure, saved, rejected, save, editRejected } = useMaterialRegistration()
const materials = ref<Material[]>([])
const loaded = ref(false)
const loading = ref(false)
const loadFailure = ref('')
const errors = ref<Partial<Record<keyof MaterialInput, string>>>({})
const nameInput = ref<HTMLInputElement | null>(null)
const unitInput = ref<HTMLInputElement | null>(null)
const available = computed(() => connection.value === 'connected')

async function loadMaterials() {
  if (!available.value || loading.value) return
  loading.value = true
  loadFailure.value = ''
  try {
    materials.value = await apiRequest<Material[]>('/api/materials')
    loaded.value = true
  } catch {
    loadFailure.value = '재료 목록을 불러오지 못했습니다. 연결을 확인한 뒤 다시 불러와 주세요.'
  } finally {
    loading.value = false
  }
}

watch(available, (connected) => { if (connected) void loadMaterials() }, { immediate: true })

async function submit() {
  if (!available.value || saving.value) return
  errors.value = validateMaterial(normalizeMaterial({ name: name.value, unit: unit.value }))
  if (errors.value.name) { nameInput.value?.focus(); return }
  if (errors.value.unit) { unitInput.value?.focus(); return }
  const material = await save()
  if (material) {
    materials.value = [material, ...materials.value.filter(item => item.id !== material.id)]
    loaded.value = true
    nameInput.value?.focus()
    void loadMaterials()
  }
}

useHead({ title: '재료 관리 | 순창한상' })
</script>

<template>
  <div class="page-heading">
    <div><h1>재료 관리</h1><p>주방에서 사용하는 재료와 관리 단위를 기록하세요.</p></div>
    <span v-if="loaded" class="count-label">등록한 재료 <strong>{{ materials.length }}</strong></span>
  </div>

  <div class="workspace-grid">
    <section class="registration-panel" aria-labelledby="register-title">
      <div class="section-heading"><h2 id="register-title">새 재료 등록</h2><p>이름과 단위부터 차근차근.</p></div>
      <form novalidate :aria-busy="saving" @submit.prevent="submit">
        <div class="field">
          <label for="material-name">재료 이름 <span class="required-label">필수</span></label>
          <input id="material-name" ref="nameInput" v-model="name" name="name" autocomplete="off" placeholder="예: 무, 간장, 김" :readonly="!!pending || saving" :disabled="!available" :aria-invalid="!!errors.name" :aria-describedby="errors.name ? 'name-error' : undefined" @input="errors.name = undefined">
          <p v-if="errors.name" id="name-error" class="field-error">{{ errors.name }}</p>
        </div>
        <div class="field">
          <label for="material-unit">관리 단위 <span class="required-label">필수</span></label>
          <input id="material-unit" ref="unitInput" v-model="unit" name="unit" list="material-unit-options" autocomplete="off" :readonly="!!pending || saving" :disabled="!available" :aria-invalid="!!errors.unit" :aria-describedby="errors.unit ? 'unit-error unit-hint' : 'unit-hint'" @input="errors.unit = undefined">
          <datalist id="material-unit-options"><option value="g"/><option value="mL"/><option value="개"/><option value="장"/></datalist>
          <p id="unit-hint" class="field-hint">g, mL, 개, 장 등 원하는 단위를 입력하세요.</p>
          <p v-if="errors.unit" id="unit-error" class="field-error">{{ errors.unit }}</p>
        </div>
        <div v-if="pending && !failure && !saving" class="notice warning" role="status">완료를 확인하지 못한 등록이 있습니다. 같은 내용으로 재시도하면 중복으로 등록되지 않습니다.</div>
        <div v-if="failure" class="notice warning" role="alert">
          <p>{{ failure }}</p>
          <button v-if="rejected" class="text-button" type="button" @click="editRejected">입력 내용 수정</button>
        </div>
        <button class="primary-button" type="submit" :disabled="!available || saving || rejected">
          {{ saving ? '등록 확인 중…' : pending ? '같은 요청 재시도' : '재료 등록' }}
        </button>
        <p v-if="!available" class="field-hint">앱에 연결되면 등록할 수 있습니다.</p>
      </form>
      <div v-if="saved" class="notice success" role="status">
        <p><strong>{{ saved.name }}</strong> 재료를 등록했습니다.</p>
        <NuxtLink :to="`/materials/${saved.id}`">등록한 재료 보기</NuxtLink>
      </div>
    </section>

    <section class="material-panel" aria-labelledby="materials-title" :aria-busy="loading">
      <div class="list-heading"><h2 id="materials-title">등록한 재료</h2><button class="text-button" type="button" :disabled="!available || loading" @click="loadMaterials">{{ loading ? '불러오는 중…' : '새로고침' }}</button></div>
      <div v-if="loadFailure" class="notice warning" role="alert">{{ loadFailure }}</div>
      <div v-if="!loaded" class="empty-state"><div class="empty-symbol" aria-hidden="true">○</div><h3>{{ loading ? '재료를 불러오고 있습니다' : '연결을 기다리고 있습니다' }}</h3><p>{{ loadFailure ? '위의 새로고침을 눌러 다시 확인하세요.' : '연결되면 저장된 재료를 보여드릴게요.' }}</p></div>
      <div v-else-if="materials.length === 0" class="empty-state"><svg class="empty-bowl" viewBox="0 0 80 64" fill="none" aria-hidden="true"><path d="M11 25h58c-2 20-12 29-29 29S13 45 11 25Z" stroke="currentColor" stroke-width="2"/><path d="M20 34h40M27 9c-5 5 5 5 0 10m13-10c-5 5 5 5 0 10m13-10c-5 5 5 5 0 10" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg><h3>첫 재료를 기록해 보세요</h3><p>이름과 관리 단위를 등록하면<br>여기서 바로 확인할 수 있습니다.</p></div>
      <template v-else>
        <p v-if="!available" class="stale-note">마지막으로 불러온 목록입니다. 연결되면 다시 확인합니다.</p>
        <ul class="material-list">
          <li v-for="material in materials" :key="material.id">
            <NuxtLink v-if="available" class="material-row" :to="`/materials/${material.id}`">
              <span class="material-initial" aria-hidden="true">{{ [...material.name][0] }}</span>
              <span class="material-copy"><strong>{{ material.name }}</strong><span>{{ formatCreatedAt(material.created_at) }} 등록</span></span>
              <span class="unit-tag">{{ material.unit }}</span><span class="row-chevron" aria-hidden="true">›</span>
            </NuxtLink>
            <div v-else class="material-row"><span class="material-initial" aria-hidden="true">{{ [...material.name][0] }}</span><span class="material-copy"><strong>{{ material.name }}</strong><span>{{ formatCreatedAt(material.created_at) }} 등록</span></span><span class="unit-tag">{{ material.unit }}</span></div>
          </li>
        </ul>
      </template>
    </section>
  </div>
</template>
