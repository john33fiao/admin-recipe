<script setup lang="ts">
import type { BulkKind, BulkRow } from '~/types/bulk'
import { bulkColumns, bulkLabels, bulkTemplate } from '~/utils/bulk-csv'
import { blankReceiptSelection } from '~/utils/receipt'
const { session, connection, materials, busy, failure, notice, storageFailure, corrupt, unresolved, changedEpoch, ready, rowLocked, editCell, addRow, removeRow, start, upload, paste, prepare, run, acceptEffect, cancelReview, separateDuplicate, resumeDuplicate, draftUpdates, applyCurrentDraft, load } = useBulkEntry()
const pasted = ref(''), selectedId = ref(''), previewPanel = ref<HTMLElement | null>(null)
const selected = computed(() => session.value.rows.find(row => row.id === selectedId.value) ?? null)
const currentDraftData = (id: string) => { const payload = draftUpdates.value[id]?.head.payload; return payload?.kind === 'receipt' ? payload.data : null }
const changed = (row: BulkRow, field: string, value: string) => editCell(row, field, value)
function download() {
  const url = URL.createObjectURL(new Blob([bulkTemplate(session.value.kind)], { type: 'text/csv;charset=utf-8' }))
  const link = document.createElement('a'); link.href = url; link.download = `sunchang-${session.value.kind}.csv`; link.click(); URL.revokeObjectURL(url)
}
function fileChanged(event: Event) { const input = event.target as HTMLInputElement; const file = input.files?.[0]; if (file) void upload(file); input.value = '' }
function changeKind(event: Event) { const input = event.target as HTMLSelectElement; start(input.value as BulkKind); input.value = session.value.kind; selectedId.value = '' }
async function confirmInputs() { await prepare(); await nextTick(); previewPanel.value?.focus() }
function inclusions(row: BulkRow) {
  if (rowLocked(row)) return
  row.values.stock_effect = 'reconciled'
  if (!row.selections.length) row.selections = row.values.observation_version_id ? [{ observation_version_id: row.values.observation_version_id, source: { kind: 'all_available' }, included_quantity: row.values.included_quantity ?? '' }] : [blankReceiptSelection()]
  row.values.observation_version_id = ''; row.values.included_quantity = ''; selectedId.value = row.id
}
useHead({ title: '일괄 입력 | 순창한상' })
</script>
<template>
  <div class="page-heading"><div><h1>일괄 입력</h1><p>CSV를 올리거나 표에 직접 입력하고, 수정한 최종 내용을 확인한 뒤 등록합니다.</p></div></div>
  <p v-if="failure" class="notice warning" role="alert">{{ failure }}</p><p v-if="storageFailure" class="notice warning" role="alert">{{ storageFailure }}</p><p v-if="notice" class="notice" role="status">{{ notice }}</p>
  <p v-if="changedEpoch" class="notice warning" role="alert">자료가 복원되었습니다. 이전 입력·결과·미확인 요청을 보존했습니다. <NuxtLink to="/restore#retained-inputs">보존 입력 확인</NuxtLink> 후 빈 새 입력에서 시작하세요.</p>
  <section class="bulk-input-panel" aria-labelledby="bulk-source-title" :aria-busy="busy"><h2 id="bulk-source-title">1. 입력 준비</h2>
    <div class="bulk-tools"><div class="field"><label for="bulk-kind">등록 대상</label><select id="bulk-kind" :value="session.kind" :disabled="busy || unresolved || changedEpoch" @change="changeKind"><option v-for="(label, kind) in bulkLabels" :key="kind" :value="kind">{{ label }}</option></select></div><button class="secondary-button" type="button" @click="download">{{ bulkLabels[session.kind] }} CSV 양식 내려받기</button><div class="field"><label for="bulk-file">UTF-8 CSV 업로드</label><input id="bulk-file" type="file" accept=".csv,text/csv" :disabled="busy || unresolved || changedEpoch" @change="fileChanged"></div></div>
    <p class="field-hint">Excel에서는 UTF-8 CSV로 저장하세요. .xlsx 파일은 지원하지 않습니다. 파일·붙여넣기는 1MiB, 한 번에 200행까지입니다. 업로드는 아래 표를 바꾸며 이전 입력은 보존합니다.</p>
    <details class="bulk-help"><summary>양식·참조·등록 방법</summary><p>열 순서: {{ bulkColumns[session.kind].map(column => column.label).join(' → ') }}. 표에서는 Tab·Shift+Tab으로 셀을 이동합니다. 쉼표·인용부호·줄바꿈이 든 CSV 값은 인용부호로 감쌉니다.</p><p v-if="session.kind === 'material'">종류는 raw(원물), prepared(손질재료), stocked(비축품), menu(판매메뉴)입니다. 같은 이름도 별도 ID로 등록하며, 재료 등록만으로 재고나 가격을 만들지 않습니다.</p><p v-if="session.kind === 'menu'">같은 메뉴 묶음의 행들을 하나의 새 메뉴·레시피로 등록합니다. 묶음마다 이름·단위·설명·기준 개수·판매가를 똑같이 적고 구성 재료를 여러 줄로 적으세요. 구성 없는 메뉴는 구성 열을 모두 비웁니다. 판매가 빈 값은 미설정, 0은 0원입니다. 구성 재료 ID는 사용 중인 재료 목록에서 선택할 수 있습니다.</p><p v-if="session.kind === 'receipt'">재료 ID는 표에서 실제 재료를 선택해 바꿀 수 있습니다. 구입금액이 없으면 확정하지 않으며 실제 0원은 0을 적습니다. 입고일은 받은 날짜이고 앱의 입력 시각과 다릅니다. 재고 반영은 unreflected(미반영), reconciled(실사 포함)입니다. 날짜에서 포함량을 추정하지 않습니다. CSV의 실사 버전 ID·포함 수량은 해당 실사의 남은 근거 전체 선택으로 해석하며, 여러 근거는 행의 ‘실사 포함 근거’에서 편집하세요.</p><p>파일 이름이나 항목 이름으로 병합하지 않습니다. 같은 내용의 재업로드는 이 탭에 보관된 처리 상태를 확인합니다. 앱을 다시 실행해 주소가 바뀌었으면 실제 목록을 먼저 확인하세요. 표의 입력과 요청은 이 탭에 보관하며 서버의 영구 파일 중복 원장은 아닙니다.</p></details>
    <details class="bulk-paste"><summary>시트에서 여러 행 붙여넣기</summary><label for="bulk-paste">제목 없이 위 열 순서로 복사한 셀</label><textarea id="bulk-paste" v-model="pasted" rows="4" :disabled="busy || unresolved || changedEpoch" spellcheck="false"/><button class="secondary-button" type="button" :disabled="!pasted || busy || unresolved || changedEpoch" @click="paste(pasted)">붙여넣은 값으로 표 만들기</button></details>
    <div class="button-row"><button class="text-button" type="button" :disabled="busy" @click="start(session.kind); selectedId = ''">원입력 보존 후 빈 새 입력</button><button class="text-button" type="button" :disabled="busy || connection !== 'connected'" @click="load">참조 목록 다시 불러오기</button><NuxtLink to="/restore#retained-inputs">보존 입력·요청</NuxtLink></div>
  </section>
  <section v-if="!corrupt" class="bulk-sheet-section" aria-labelledby="bulk-sheet-title"><div class="list-heading"><h2 id="bulk-sheet-title">2. 입력·수정</h2><button id="bulk-add-row" class="secondary-button" type="button" :disabled="busy || unresolved || changedEpoch || session.rows.length >= 200" @click="addRow">행 추가</button></div>
    <p class="field-hint">표가 넓으면 안쪽을 가로로 스크롤하세요. 확인 뒤 값을 바꾸면 다시 확인해야 합니다. 성공한 행과 결과가 불명확한 원요청은 편집하지 않습니다.</p>
    <BulkSheet :kind="session.kind" :rows="session.rows" :materials="materials" :errors="session.review?.errors ?? {}" :locked="rowLocked" @edit="changed" @remove="removeRow" @inclusions="inclusions" />
    <section v-if="selected && session.kind === 'receipt'" class="bulk-inclusions" aria-label="선택 행의 실사 포함 근거"><div class="list-heading"><h3>{{ session.rows.indexOf(selected) + 1 }}행 실사 포함 근거</h3><button class="text-button" type="button" @click="selectedId = ''">근거 편집 닫기</button></div><ReceiptInclusionEditor v-model="selected.selections" :material-id="selected.values.material_id" :locked="rowLocked(selected)" :available="connection === 'connected'" :issue="field => session.review?.errors[selected!.id]?.[field]" /></section>
    <div class="button-row"><button class="primary-button" type="button" :disabled="busy || unresolved || session.duplicate || connection !== 'connected' || !session.rows.length" @click="confirmInputs">등록 내용 확인</button><button class="text-button" type="button" :disabled="busy || unresolved" @click="cancelReview">확인 취소</button></div>
  </section>
  <section v-if="session.duplicate" class="notice warning" aria-label="같은 내용 재업로드"><h2>같은 내용의 이전 입력</h2><p>아래 처리 결과를 확인하세요. 성공 행은 다시 등록하지 않습니다.</p><div class="button-row"><button class="secondary-button" type="button" @click="resumeDuplicate">이전 입력 이어가기</button><button class="text-button" type="button" :disabled="unresolved" @click="separateDuplicate">같은 내용을 별도 새 입력으로 준비</button></div></section>
  <section v-if="session.review" ref="previewPanel" class="bulk-review" tabindex="-1" aria-labelledby="bulk-review-title"><h2 id="bulk-review-title">3. 최종 확인·등록 결과</h2><p>등록은 항목별로 진행합니다. 중간에 멈추면 성공한 행은 유지하고 미처리 행을 이어갑니다. 전체 배치를 한 번에 되돌리는 기능은 없습니다.</p><p v-if="session.kind === 'receipt'" class="notice">입고별 미리보기는 확인한 현재 상태의 독립 결과입니다. 앞선 입고나 다른 변경으로 효과가 달라지면 해당 행에서 멈추고 새 결과를 확인받습니다.</p>
    <ul v-if="session.review.warnings.length" class="notice warning"><li v-for="warning in session.review.warnings" :key="warning">{{ warning }}</li></ul><label v-if="session.review.warnings.length" class="bulk-warning-check"><input v-model="session.acceptedWarnings" type="checkbox" :disabled="busy">중복·같은 이름·순차 등록 안내를 확인했습니다</label>
    <p v-if="Object.keys(session.review.errors).length" class="notice warning" role="alert">표의 오류를 고친 뒤 등록 내용을 다시 확인해 주세요. 등록 전 검사에서 오류가 있으면 어떤 행도 새로 등록하지 않습니다.</p>
    <ol class="bulk-results"><li v-for="entry in session.review.items" :key="entry.id" :data-bulk-result="entry.id"><h3>{{ entry.label || '이름 미입력' }}</h3><p>표의 {{ entry.rowIds.map(id => session.rows.findIndex(row => row.id === id) + 1).join(', ') }}행</p>
        <template v-if="entry.command.kind === 'material'"><p>{{ entry.command.input.name }} · {{ entry.command.input.unit }} · {{ entry.command.input.kind }}</p><p class="bulk-text">{{ entry.command.input.description }}</p></template>
        <template v-else-if="entry.command.kind === 'menu'"><p>기준 {{ entry.command.data.basis_count }}개 · 판매가 {{ entry.command.data.selling_price === null ? '미설정' : entry.command.data.selling_price + '원' }} · 구성 {{ entry.command.data.lines.length }}행</p><ul><li v-for="(line,index) in entry.command.data.lines" :key="index">{{ materials.find(item => item.id === line.material_id)?.name ?? line.material_id }} · {{ line.quantity }}{{ line.unit }}<p class="material-id">ID {{ line.material_id }} · 버전 {{ line.material_version_id }}</p></li></ul></template>
        <template v-else><p class="material-id">재료 ID {{ entry.command.payload.data.source.kind === 'direct' ? entry.command.payload.data.source.material_id : '' }} · 버전 {{ entry.command.payload.data.source.material_version_id }}</p><BulkReceiptPreview :effect="entry.effect" /></template>
        <p v-if="entry.result" class="notice success" role="status">등록 완료 · <NuxtLink :to="entry.result.href">등록한 {{ bulkLabels[session.kind] }} 보기</NuxtLink></p><p v-else-if="entry.draft" class="field-hint">입고 초안 보관 · <NuxtLink :to="`/business/${entry.draft.id}`">초안·이력 보기</NuxtLink></p>
        <p v-if="entry.failure" class="notice warning" role="alert">{{ entry.failure }}</p><section v-if="entry.recovery === 'effect' && entry.preview" class="bulk-changed-effect"><h4>변경된 입고 효과</h4><BulkReceiptPreview :effect="entry.preview.effect" /><button class="secondary-button" type="button" :disabled="busy" @click="acceptEffect(entry)">변경된 미리보기 확인</button></section>
        <section v-if="currentDraftData(entry.id) && (!entry.result || draftUpdates[entry.id]!.item.status === 'confirmed')" class="notice warning"><p>다른 화면의 현재 {{ draftUpdates[entry.id]!.item.status === 'confirmed' ? '확정 기록' : '초안' }}: 수량 {{ currentDraftData(entry.id)?.quantity }}{{ currentDraftData(entry.id)?.unit }} · {{ currentDraftData(entry.id)?.amount }}원 · {{ currentDraftData(entry.id)?.receipt_date }} 입고</p><button v-if="!entry.result && draftUpdates[entry.id]!.item.status === 'draft'" class="secondary-button" type="button" :disabled="busy || !!entry.pending" @click="applyCurrentDraft(entry)">현재 초안 내용으로 표 갱신</button></section>
        <p v-if="entry.pending" class="field-hint">원요청을 보존했습니다. {{ entry.recovery === 'retry' ? '아래 버튼은 같은 요청으로 재시도합니다.' : '새 요청으로 자동 교체하지 않습니다.' }}</p>
      </li></ol>
    <button class="primary-button bulk-submit" type="button" :disabled="!ready || connection !== 'connected' || session.review.items.every(item => !!item.result)" @click="run">{{ session.review.items.some(item => !!item.pending || !!item.result || !!item.draft) ? '같은 요청으로 이어서 등록' : '확인한 내용 일괄 등록' }}</button>
    <button v-if="session.review.items.some(item => item.recovery === 'review')" class="secondary-button" type="button" :disabled="busy || unresolved || connection !== 'connected'" @click="confirmInputs">최신 자료 확인 후 입력 재검토</button>
  </section>
</template>
<style scoped>
.bulk-input-panel, .bulk-review, .bulk-inclusions { padding: 24px; border: 1px solid var(--line); background: var(--surface); border-radius: 12px; margin: 24px 0; min-width: 0; }
.bulk-input-panel > * + *, .bulk-review > * + *, .bulk-sheet-section > * + * { margin-top: 20px; }
.bulk-tools { display: flex; flex-wrap: wrap; gap: 20px; align-items: end; }
.bulk-tools > .field { flex: 1 1 230px; }
.bulk-help summary, .bulk-paste summary { cursor: pointer; font-weight: 600; }
.bulk-help p { margin-top: 12px; }
.bulk-paste > * + * { margin-top: 12px; }
.bulk-results { padding: 0; list-style: none; }
.bulk-results > li { padding: 24px 0; border-top: 1px solid var(--line); }
.bulk-results > li > * + * { margin-top: 12px; }
.bulk-warning-check { display: flex; gap: 12px; align-items: start; }
.bulk-warning-check input { width: 22px; min-height: 22px; flex: 0 0 22px; margin-top: 3px; }
.bulk-text { white-space: pre-wrap; overflow-wrap: anywhere; }
.bulk-changed-effect { border: 2px solid var(--focus); padding: 20px; border-radius: 8px; }
.bulk-review .notice.warning { padding-left: 24px; }
.bulk-review:focus { outline: 3px solid var(--focus); outline-offset: 4px; }
@media (max-width: 600px) { .bulk-input-panel, .bulk-review, .bulk-inclusions { padding: 16px; } .bulk-tools { flex-direction: column; align-items: stretch; } .bulk-tools > .field { flex: initial; } }
</style>
