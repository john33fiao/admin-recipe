<script setup lang="ts">
import type { WorkspaceEntry, WorkspaceKind } from '~/types/workspace'
import { businessInputText, businessNote } from '~/utils/business'
import { formatCreatedAt } from '~/utils/material'
import { unresolvedWorkspaceRequest, workspaceKinds, workspaceResultLink, workspaceStatus } from '~/utils/workspace'
const workspace = useBusinessWorkspace()
const { entries, selected, selectedDetail, initial, snapshots, summaries, latestState, failure, loading, storageFailure, recordFailures } = workspace
const connection = useConnection(), filter = ref('open')
const heading = ref<HTMLElement | null>(null), listPanel = ref<HTMLElement | null>(null)
const visibleEntries = computed(() => entries.value.filter(entry => filter.value === 'all' || !entry.recordId || summaries.value.find(item => item.id === entry.recordId)?.status === 'draft' || workspace.tab.value.entries.some(item => item.identity === entry.identity)))
const unresolved = computed(() => entries.value.filter(entry => unresolvedWorkspaceRequest(snapshots.value[entry.identity])))
const unfinished = computed(() => entries.value.filter(entry => entry.identity !== selected.value?.identity && (snapshots.value[entry.identity]?.status ?? summaries.value.find(item => item.id === entry.recordId)?.status ?? 'draft') === 'draft'))
const confirmation = computed(() => selectedDetail.value?.confirmations.find(item => item.id === selectedDetail.value?.item.confirmation_id))
const resultLink = computed(() => workspaceResultLink(confirmation.value?.result))
function summary(entry: WorkspaceEntry) { return summaries.value.find(item => item.id === entry.recordId) }
function status(entry: WorkspaceEntry) { return workspaceStatus(snapshots.value[entry.identity], summary(entry)?.status, entry.recordId) }
function inputText(entry: WorkspaceEntry) { const value = snapshots.value[entry.identity]?.payload; return value ? businessInputText(value) : summary(entry)?.material_name ?? '입력을 선택해 확인하세요' }
async function add(kind: WorkspaceKind) { if (workspace.add(kind)) { await nextTick(); heading.value?.focus(); heading.value?.scrollIntoView({ block: 'start' }) } }
async function select(entry: WorkspaceEntry) { const pending = workspace.select(entry); await nextTick(); heading.value?.focus(); await pending }
function backToList() { listPanel.value?.querySelector<HTMLElement>(`button[data-workspace-id="${selected.value?.identity}"]`)?.focus(); listPanel.value?.scrollIntoView({ block: 'start' }) }
useHead({ title: '모아 입력 | 순창한상' })
</script>
<template>
  <div class="page-heading"><div><h1>모아 입력</h1><p>판매·생산·폐기·실사를 한곳에서 작성합니다. 기록 하나씩 임시 저장하고, 미리보기를 확인한 뒤 확정하세요.</p></div></div>
  <p class="workspace-guidance">입력 시각은 자동으로 기록합니다. 임시 저장 전 입력은 이 탭에만 남고, 임시 저장한 초안은 앱을 다시 열어도 이어서 작성할 수 있습니다.</p>
  <div class="button-row workspace-add" aria-label="새 작업 추가"><button v-for="(label, kind) in workspaceKinds" :key="kind" type="button" class="secondary-button" :disabled="connection !== 'connected' || !latestState" @click="add(kind)">{{ label }} 추가</button></div>
  <p v-if="storageFailure" class="notice warning" role="alert">{{ storageFailure }}</p><p v-if="failure" class="notice warning" role="alert">{{ failure }} <button type="button" class="text-button" :disabled="loading || connection !== 'connected'" @click="workspace.loadList">목록 다시 불러오기</button></p>
  <div v-if="unresolved.length" class="notice warning workspace-pending" role="status"><p>처리 결과를 확인할 작업 {{ unresolved.length }}개가 있습니다. 필요한 선행 작업의 결과를 먼저 확인한 뒤 실사를 미리보기하세요.</p><div class="button-row"><button v-for="entry in unresolved" :key="entry.identity" type="button" class="text-button" @click="select(entry)">{{ workspaceKinds[entry.kind] }} 요청 확인 · {{ entry.identity.slice(-8) }}</button></div></div>
  <div class="workspace-layout">
    <section ref="listPanel" class="workspace-list" aria-label="작업 목록"><div class="list-heading"><h2>작업 목록 <span class="field-hint">{{ visibleEntries.length }}개</span></h2><button type="button" class="text-button" :disabled="loading || connection !== 'connected'" @click="workspace.loadList">목록 새로 확인</button></div><div class="field"><label for="workspace-filter">표시할 작업</label><select id="workspace-filter" v-model="filter"><option value="open">진행 중인 입력</option><option value="all">모든 기록</option></select></div>
      <p v-if="loading && !entries.length" class="field-hint">저장한 작업을 불러오고 있습니다.</p><p v-else-if="!visibleEntries.length" class="empty-state">입력할 작업을 위에서 추가하세요. 저장한 초안은 이 목록에서 다시 열 수 있습니다.</p>
      <ol class="workspace-items"><li v-for="entry in visibleEntries" :key="entry.identity" class="workspace-item"><button type="button" :data-workspace-id="entry.identity" :aria-pressed="selected?.identity === entry.identity" @click="select(entry)"><span class="workspace-item-title">{{ workspaceKinds[entry.kind] }} · {{ summary(entry)?.material_name ?? entry.identity.slice(-8) }}</span><span class="workspace-item-status">{{ status(entry) }}</span><span class="workspace-item-input">{{ inputText(entry) }}</span><time v-if="summary(entry)?.updated_at">{{ formatCreatedAt(summary(entry)!.updated_at) }}</time></button></li></ol>
    </section>
    <section class="workspace-content" aria-label="선택한 작업"><h2 ref="heading" class="workspace-editor-heading" tabindex="-1">{{ selected ? `${workspaceKinds[selected.kind]} 입력` : '작업을 선택하세요' }}</h2>
      <template v-if="selected"><div class="workspace-selection-bar"><span class="status-label">{{ status(selected) }}</span><button class="text-button" type="button" @click="backToList">작업 목록으로</button></div>
        <section v-if="snapshots[selected.identity]" class="workspace-input-summary" aria-label="현재 화면 입력"><h3>현재 화면 입력</h3><p>{{ businessInputText(snapshots[selected.identity]!.payload) }}</p><p v-if="businessNote(snapshots[selected.identity]!.payload)" class="preserve-lines">{{ businessNote(snapshots[selected.identity]!.payload) }}</p><p v-if="snapshots[selected.identity]!.dirty" class="field-hint">바꾼 입력이 아직 임시 저장되지 않았습니다.</p></section>
        <section v-if="selectedDetail" class="workspace-saved-summary" aria-label="저장된 입력"><h3>저장된 입력</h3><p>{{ businessInputText(selectedDetail.head.payload) }}</p><p v-if="businessNote(selectedDetail.head.payload)" class="preserve-lines">{{ businessNote(selectedDetail.head.payload) }}</p><p class="field-hint">{{ formatCreatedAt(selectedDetail.head.recorded_at) }} · 기록 {{ selectedDetail.head.seq }}</p><NuxtLink :to="`/business/${selectedDetail.item.id}`">원업무 기록 보기</NuxtLink></section>
        <section v-if="selected.kind === 'stocktake'" class="workspace-order-note"><h3>실사 전에 확인할 기록</h3><p>확정 입고·생산·반품·폐기는 실사 기준에 반영됩니다. 판매는 실사 전에 확정한 기록이 이번 소모 배분 대상입니다.</p><p v-if="unfinished.length">다른 입력 {{ unfinished.length }}개가 아직 확정되지 않았습니다. 필요한 기록만 먼저 확정하고 미리보기를 새로 확인하세요.</p><p class="field-hint">미확정 생산·폐기는 정산 기준에, 미확정 판매는 소모 배분에 포함되지 않습니다. 다른 초안을 모두 확정할 필요는 없습니다.</p><div class="button-row"><NuxtLink to="/receipts">입고 기록 확인</NuxtLink><NuxtLink to="/purchase-returns">반품 기록 확인</NuxtLink></div></section>
        <p v-if="selected.recordId && recordFailures[selected.recordId]" class="notice warning" role="alert">{{ recordFailures[selected.recordId] }} <button type="button" class="text-button" :disabled="connection !== 'connected'" @click="workspace.loadRecord(selected)">작업 다시 불러오기</button></p>
        <BusinessWorkspaceEditor v-if="initial" :key="selected.identity" :identity="selected.identity" :initial="initial" :host="workspace.hostFor(selected.identity)" :refresh="workspace.refreshFor(selected)" />
        <p v-else class="field-hint">선택한 기록을 불러오고 있습니다.</p>
        <WorkspaceConfirmedResult v-if="confirmation" :key="confirmation.id" :confirmation-id="confirmation.id" :revision="`${latestState?.epoch}:${latestState?.revision}`" />
        <NuxtLink v-if="resultLink" class="secondary-link workspace-result-link" :to="resultLink.to">{{ resultLink.label }}</NuxtLink>
      </template><p v-else class="empty-state">새 작업을 추가하거나 저장한 입력을 선택하세요. 확정 전에는 수량과 보유 가치가 바뀌지 않습니다.</p>
    </section>
  </div>
</template>
