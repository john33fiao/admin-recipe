<script setup lang="ts">
import type { ProfileDetail } from '~/types/profile'
import { apiRequest } from '~/utils/api'
import { safeSupplierUrl } from '~/utils/profile'
import { formatCreatedAt } from '~/utils/material'

const props = defineProps<{ materialId: string }>()
const editor = useMaterialProfile(props.materialId)
const { input, current, photos, file, pending, dirty, locked, busy, reviewing, hashing, loading, readFailure, restoreFailure, failure, notice, storageFailure, errors, canRetry, canReview, needsFile, history, historyLoading, historyFailure } = editor
const connection = useConnection(), prefix = `profile-${props.materialId}`
const form = ref<HTMLFormElement | null>(null), fileInput = ref<HTMLInputElement | null>(null)
const showHistory = ref(false), past = ref<ProfileDetail | null>(null), pastLoading = ref(false), pastFailure = ref('')
const selectedPosition = ref(0), objectUrl = ref(''), imageFailed = ref(false)
let generation = 0
watch(file, selected => { if (objectUrl.value) URL.revokeObjectURL(objectUrl.value); objectUrl.value = selected ? URL.createObjectURL(selected) : '' }, { immediate: true })
onScopeDispose(() => { generation++; if (objectUrl.value) URL.revokeObjectURL(objectUrl.value) })
const displayed = computed(() => past.value ? past.value.version?.photos.map(photo => ({ key: photo.id, caption: photo.caption, primary: photo.is_primary, src: `/api/photo-assets/${photo.asset.sha256}` })) ?? [] : input.value.photos.map((photo, index) => ({ key: photo.source === 'existing' ? photo.photo_id : 'upload', caption: photo.caption, primary: input.value.primary_position === index, src: photo.source === 'upload' ? objectUrl.value : photos.value[photo.photo_id] ? `/api/photo-assets/${photos.value[photo.photo_id]!.asset.sha256}` : '' })))
const selected = computed(() => displayed.value[selectedPosition.value] ?? null)
watch(() => displayed.value.length, length => { if (selectedPosition.value >= length) selectedPosition.value = Math.max(0, length - 1) })
watch(() => selected.value?.src, () => { imageFailed.value = false })
watch(() => current.value?.version?.id, () => { if (!past.value && !dirty.value) selectedPosition.value = input.value.primary_position ?? 0 })
const unavailable = computed(() => connection.value !== 'connected' || !current.value)
const fileLocked = computed(() => unavailable.value || busy.value || reviewing.value || hashing.value || !!restoreFailure.value || (!!pending.value && !canRetry.value))
async function focusError() { await nextTick(); form.value?.querySelector<HTMLElement>('[aria-invalid="true"]')?.focus() }
async function save() { if (!(await editor.send())) await focusError() }
async function choose(event: Event) {
  const target = event.target as HTMLInputElement, selectedFile = target.files?.[0]
  target.value = ''
  if (!selectedFile) return
  if (await editor.chooseFile(selectedFile)) { selectedPosition.value = Math.max(0, input.value.photos.findIndex(photo => photo.source === 'upload')); await nextTick(); if (!pending.value) form.value?.querySelector<HTMLTextAreaElement>(`#${prefix}-photo-${selectedPosition.value}`)?.focus() }
  else await focusError()
}
function addSupplier() { if (!locked.value && input.value.suppliers.length < 20) { input.value.suppliers.push({ url: null, description: '' }); void nextTick(() => document.getElementById(`${prefix}-supplier-${input.value.suppliers.length - 1}-url`)?.focus()) } }
function removeSupplier(index: number) { if (locked.value) return; input.value.suppliers.splice(index, 1); void nextTick(() => document.getElementById(`${prefix}-add-supplier`)?.focus()) }
function removePhoto(index: number) { editor.removePhoto(index); void nextTick(() => fileInput.value?.focus()) }
async function historyToggle() { showHistory.value = !showHistory.value; if (showHistory.value) await editor.loadHistory() }
async function readVersion(id: string) {
  const token = ++generation
  pastLoading.value = true; pastFailure.value = ''
  try { const result = await apiRequest<ProfileDetail>(`/api/materials/${props.materialId}/profiles/${id}`); if (token === generation) { past.value = result; selectedPosition.value = result.version?.photos.findIndex(photo => photo.is_primary) ?? 0 } }
  catch { if (token === generation) pastFailure.value = '이 사진·구입처 버전을 읽지 못했습니다.' }
  finally { if (token === generation) pastLoading.value = false }
}
function showCurrent() { generation++; past.value = null; pastLoading.value = false; pastFailure.value = ''; selectedPosition.value = input.value.primary_position ?? 0 }
</script>

<template>
  <section class="material-profile" :aria-labelledby="`${prefix}-title`" :aria-busy="busy || loading || pastLoading">
    <div class="profile-heading"><div><h2 :id="`${prefix}-title`">사진·구입처</h2><p>품목 설명과 배합은 각각의 입력에서 저장합니다.</p></div><button class="text-button" type="button" :aria-expanded="showHistory" :disabled="historyLoading" @click="historyToggle">사진·구입처 이력</button></div>
    <p v-if="readFailure" class="notice warning" role="alert">{{ readFailure }} <button class="text-button" type="button" :disabled="connection !== 'connected' || loading" @click="editor.load">다시 불러오기</button></p>
    <p v-if="storageFailure || restoreFailure" class="notice warning" role="alert">{{ restoreFailure || storageFailure }}</p>
    <div v-if="showHistory" class="profile-history"><h3>사진·구입처 버전</h3><p v-if="historyFailure" class="notice warning" role="alert">{{ historyFailure }}</p><p v-else-if="historyLoading">이력을 읽고 있습니다.</p><p v-else-if="!history?.versions.length" class="stale-note">저장된 사진·구입처 버전이 없습니다.</p><ol v-else class="history-list"><li v-for="version in [...history.versions].reverse()" :key="version.id"><button class="text-button" type="button" :aria-pressed="past?.version?.id === version.id" :disabled="pastLoading" @click="readVersion(version.id)">{{ formatCreatedAt(version.recorded_at) }} · 기록 {{ version.seq }} 보기</button><p>사진 {{ version.photo_count }}장 · 구입처 {{ version.supplier_count }}건<span v-if="version.copied_from_version_id"> · 독립 복제로 시작</span></p></li></ol></div>
    <p v-if="pastFailure" class="notice warning" role="alert">{{ pastFailure }}</p>
    <div v-if="past" class="notice profile-past"><strong>저장된 과거 사진·구입처</strong><p>{{ formatCreatedAt(past.version!.recorded_at) }} · 기록 {{ past.version!.seq }}</p><button class="text-button" type="button" @click="showCurrent">현재 사진·구입처로 돌아가기</button></div>
    <template v-else><p v-if="current?.version" class="field-hint">현재 저장본 · {{ formatCreatedAt(current.version.recorded_at) }} · 기록 {{ current.version.seq }}</p><p v-if="dirty && !pending" class="draft-note" role="status">사진·구입처에 저장하지 않은 변경이 있습니다.</p><p v-if="pending && !failure" class="notice warning" role="status">{{ busy ? '사진·구입처 저장을 확인하고 있습니다.' : '완료를 확인하지 못한 요청이 있습니다. 원래 사진과 입력으로 재시도해 주세요.' }}</p></template>
    <div class="profile-gallery">
      <div class="profile-image-frame"><img v-if="selected?.src && !imageFailed" :src="selected.src" :alt="selected.caption || `${current?.material.name ?? '품목'} 사진 ${selectedPosition + 1}`" @error="imageFailed = true"><p v-else-if="imageFailed" class="notice warning" role="alert">사진을 불러오지 못했습니다. 저장된 연결은 유지됩니다. <button class="text-button" type="button" @click="imageFailed = false">사진 다시 보기</button></p><p v-else-if="selected" class="stale-note">이 사진의 원본을 확인할 수 없습니다. {{ selected.key === 'upload' ? '선택했던 파일을 다시 선택해 주세요.' : '현재 입력과 참조는 유지됩니다.' }}</p><p v-else class="stale-note">등록한 사진이 없습니다. 사진 없이도 저장할 수 있습니다.</p></div>
      <div v-if="displayed.length" class="profile-photo-choices" aria-label="표시할 사진"><button v-for="(photo, index) in displayed" :key="photo.key" type="button" :aria-pressed="selectedPosition === index" @click="selectedPosition = index"><span>{{ index + 1 }}. {{ photo.caption || '설명 없는 사진' }}</span><span v-if="photo.primary" class="status-label">대표</span><span v-if="photo.key === 'upload'" class="status-label">새 사진</span></button></div>
    </div>
    <section v-if="past" class="profile-suppliers"><h3>당시 구입처</h3><p v-if="!past.version?.suppliers.length" class="stale-note">등록한 구입처가 없습니다.</p><ul v-else class="history-list"><li v-for="supplier in past.version.suppliers" :key="supplier.position"><a v-if="supplier.url && safeSupplierUrl(supplier.url)" :href="supplier.url" target="_blank" rel="noopener noreferrer">{{ supplier.url }}</a><p v-if="supplier.description" class="preserve-lines">{{ supplier.description }}</p></li></ul></section>
    <form v-else ref="form" class="profile-form" novalidate @submit.prevent="save">
      <div v-if="failure" class="notice warning" role="alert"><p>{{ failure }}</p><button v-if="canReview" class="text-button" type="button" :disabled="unavailable || busy || reviewing" @click="editor.review">{{ reviewing ? '최신 상태 확인 중…' : '최신 사진·구입처 확인 후 입력 검토' }}</button></div>
      <p v-if="notice" class="notice success" role="status">{{ notice }}</p>
      <div class="field"><label :for="`${prefix}-file`">{{ needsFile ? '선택했던 사진 다시 선택' : '사진 한 장 추가 또는 새 사진 바꾸기' }}</label><input :id="`${prefix}-file`" ref="fileInput" type="file" accept="image/jpeg,image/png,.jpg,.jpeg,.png" :disabled="fileLocked" :aria-invalid="!!errors.file" :aria-describedby="`${prefix}-file-hint ${prefix}-file-error`" @change="choose"><p :id="`${prefix}-file-hint`" class="field-hint">JPEG·PNG 한 장씩, 최대 10MiB·8,192px·2천만 화소. 선택한 원본을 보관합니다. {{ pending?.file ? '이 요청에 사용한 같은 사진만 다시 선택할 수 있습니다.' : '여러 장은 저장 후 한 장씩 더할 수 있습니다.' }}</p><p v-if="hashing" role="status">선택한 사진을 확인하고 있습니다.</p><p :id="`${prefix}-file-error`" class="field-error">{{ errors.file }}</p></div>
      <fieldset :disabled="locked || unavailable" class="profile-fields"><legend>현재 사진과 구입처 편집</legend>
        <p v-if="errors.photos" class="field-error" role="alert">{{ errors.photos }}</p>
        <div v-for="(photo, index) in input.photos" :key="photo.source === 'existing' ? photo.photo_id : 'upload'" class="profile-photo-edit"><div class="profile-photo-edit-heading"><h3>사진 {{ index + 1 }}<span v-if="photo.source === 'upload'" class="status-label">새 사진</span></h3><button class="text-button" type="button" :aria-label="`사진 ${index + 1} 연결 해제`" @click="removePhoto(index)">연결 해제</button></div><div class="field"><label :for="`${prefix}-photo-${index}`">사진 {{ index + 1 }} 설명 (선택)</label><textarea :id="`${prefix}-photo-${index}`" v-model="photo.caption" rows="2" :aria-invalid="!!errors[`photo-${index}`]" :aria-describedby="`${prefix}-photo-${index}-error`"/><p :id="`${prefix}-photo-${index}-error`" class="field-error">{{ errors[`photo-${index}`] }}</p></div><label class="checkbox-label"><input v-model="input.primary_position" type="radio" :name="`${prefix}-primary`" :value="index"> 사진 {{ index + 1 }}을 대표로 표시</label></div>
        <p v-if="input.photos.length" class="field-hint">연결 해제는 현재 목록만 바꿉니다. 이전 기록의 사진은 남습니다. 대표를 해제하면 남은 첫 사진이 대표가 됩니다.</p>
        <div class="profile-heading"><h3>구입처</h3><button :id="`${prefix}-add-supplier`" class="secondary-button" type="button" :disabled="input.suppliers.length >= 20" @click="addSupplier">구입처 추가</button></div>
        <p class="field-hint">링크나 설명 중 하나만 입력해도 됩니다. 최대 20건이며 외부 주소는 직접 열 때만 접속합니다.</p><p v-if="errors.suppliers" class="field-error">{{ errors.suppliers }}</p>
        <fieldset v-for="(supplier, index) in input.suppliers" :key="index" class="profile-supplier-edit"><legend>구입처 {{ index + 1 }}</legend><div class="field"><label :for="`${prefix}-supplier-${index}-url`">구입처 {{ index + 1 }} 링크 (선택)</label><input :id="`${prefix}-supplier-${index}-url`" v-model="supplier.url" type="url" inputmode="url" placeholder="https://" :aria-invalid="!!errors[`supplier-${index}-url`]" :aria-describedby="`${prefix}-supplier-${index}-url-error`"><p :id="`${prefix}-supplier-${index}-url-error`" class="field-error">{{ errors[`supplier-${index}-url`] }}</p><a v-if="supplier.url && safeSupplierUrl(supplier.url)" :href="supplier.url" target="_blank" rel="noopener noreferrer">구입처 {{ index + 1 }} 링크 열기</a></div><div class="field"><label :for="`${prefix}-supplier-${index}-description`">구입처 {{ index + 1 }} 설명 (선택)</label><textarea :id="`${prefix}-supplier-${index}-description`" v-model="supplier.description" rows="3" :aria-invalid="!!errors[`supplier-${index}-description`]" :aria-describedby="`${prefix}-supplier-${index}-description-error`"/><p :id="`${prefix}-supplier-${index}-description-error`" class="field-error">{{ errors[`supplier-${index}-description`] }}</p></div><button class="text-button" type="button" :aria-label="`구입처 ${index + 1} 제거`" @click="removeSupplier(index)">구입처 제거</button></fieldset>
      </fieldset>
      <p v-if="errors.form" class="field-error" role="alert">{{ errors.form }}</p>
      <button v-if="canRetry" class="primary-button" type="submit" :disabled="unavailable || busy || reviewing || hashing || needsFile">같은 사진·구입처 요청 재시도</button><button v-else class="primary-button" type="submit" :disabled="unavailable || locked">사진·구입처 저장</button>
      <p class="field-hint">이 버튼은 사진과 구입처만 저장합니다. 품목 정보나 배합의 작성 중인 변경은 함께 저장하지 않습니다.</p>
    </form>
  </section>
</template>
