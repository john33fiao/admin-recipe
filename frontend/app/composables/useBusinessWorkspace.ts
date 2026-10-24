import type { BusinessDetail, BusinessList, BusinessMutation, BusinessSummary } from '~/types/business'
import type { BusinessEditorInitial } from '~/composables/useBusinessDraft'
import type { CatalogState } from '~/types/material'
import type { BusinessEditorHost, BusinessEditorSnapshot, WorkspaceEntry, WorkspaceKind, WorkspacePayload, WorkspaceTab } from '~/types/workspace'
import { apiRequest } from '~/utils/api'
import { copyPayload, restoreBusinessPending, validBusinessPayload } from '~/utils/business'
import { advanceWorkspaceState, blankWorkspacePayload, isWorkspaceKind, linkWorkspaceRecord, newWorkspaceEntry, restoreWorkspaceTab, unresolvedWorkspaceRequest } from '~/utils/workspace'

const tabKey = 'sunchang.workspace.v1'
export function useBusinessWorkspace() {
  const connection = useConnection()
  const storageFailure = ref(''), failure = ref(''), loading = ref(false)
  const tab = useState<WorkspaceTab>(tabKey, () => {
    try { return restoreWorkspaceTab(sessionStorage.getItem(tabKey)) ?? { entries: [], selected: null } } catch { return { entries: [], selected: null } }
  })
  const summaries = ref<BusinessSummary[]>([]), details = ref<Record<string, BusinessDetail>>({})
  const snapshots = useState<Record<string, BusinessEditorSnapshot>>(`${tabKey}.snapshots`, () => ({}))
  // This is an observation of initialized editors, never a pending-state initializer.
  const liveSnapshots = useState<Record<string, BusinessEditorSnapshot>>('sunchang.business.editor-snapshots.v1', () => ({}))
  const latestState = ref<CatalogState | null>(null)
  const recordFailures = ref<Record<string, string>>({})
  const localSeeds = new Map<string, BusinessEditorInitial<WorkspacePayload>>()
  let active = true, listGeneration = 0
  const recordGenerations = new Map<string, number>()
  const recordLoading = new Set<string>()
  onScopeDispose(() => { active = false; listGeneration += 1 })
  const entries = computed<WorkspaceEntry[]>(() => {
    const known = new Set(tab.value.entries.map(entry => entry.identity))
    return [...tab.value.entries, ...summaries.value.filter(item => !known.has(item.id)).map(item => ({ identity: item.id, recordId: item.id, kind: item.kind as WorkspaceKind }))]
  })
  const selected = computed(() => entries.value.find(entry => entry.identity === tab.value.selected) ?? null)
  const selectedDetail = computed(() => selected.value?.recordId ? details.value[selected.value.recordId] ?? null : null)

  function persist(value: WorkspaceTab) {
    try { sessionStorage.setItem(tabKey, JSON.stringify(value)); storageFailure.value = ''; return true }
    catch { storageFailure.value = '이 탭의 작업 목록을 보관하지 못했습니다. 저장된 초안은 업무 기록에서 다시 열 수 있습니다. 새 입력을 추가하기 전에 브라우저 저장공간을 확인해 주세요.'; return false }
  }
  function restoreSnapshot(entry: WorkspaceEntry, detail?: BusinessDetail) {
    const live = liveSnapshots.value[entry.identity]
    if (live && live.payload.kind === entry.kind) { snapshots.value[entry.identity] = live; return }
    try {
      const key = `sunchang.business.${entry.identity}`
      const candidate = restoreBusinessPending(sessionStorage.getItem(`${key}.pending.v1`))
      const pending = candidate && candidate.record_id === entry.recordId && (candidate.action !== 'create' || candidate.body.corrects_confirmation_id === null) && (!('payload' in candidate.body) || candidate.body.payload.kind === entry.kind) ? candidate : null
      const stored = JSON.parse(sessionStorage.getItem(`${key}.draft.v1`) ?? 'null')
      const draft = validBusinessPayload(stored?.payload) && stored.payload.kind === entry.kind ? stored.payload : null
      const previous = snapshots.value[entry.identity], summary = summaries.value.find(item => item.id === entry.recordId)
      if (entry.recordId && !pending && !draft && !detail && !previous) return
      const payload = pending && 'payload' in pending.body ? pending.body.payload : draft ?? detail?.head.payload ?? previous?.payload ?? blankWorkspacePayload(entry.kind)
      snapshots.value[entry.identity] = { payload: copyPayload(payload), status: detail?.head.status ?? summary?.status ?? previous?.status ?? 'draft', dirty: !!draft, busy: false, pendingAction: pending?.action ?? null, recovery: pending ? 'retry' : '', failure: '', issueCount: detail?.issues.length ?? summary?.issue_count ?? 0, previewState: null }
    } catch { delete snapshots.value[entry.identity] }
  }
  // A normal detail page can resolve a request while this page is unmounted.
  for (const entry of tab.value.entries) restoreSnapshot(entry)
  watch(liveSnapshots, () => {
    for (const entry of entries.value) {
      const live = liveSnapshots.value[entry.identity]
      if (live && live.payload.kind === entry.kind) snapshots.value[entry.identity] = live
    }
  }, { deep: true, flush: 'sync' })
  function acceptRead(state: CatalogState, startedEpoch: string | null) {
    if (!latestState.value || latestState.value.epoch === startedEpoch && state.epoch !== startedEpoch) latestState.value = state
    else latestState.value = advanceWorkspaceState(latestState.value, state)
  }
  async function loadRecord(entry: WorkspaceEntry): Promise<BusinessDetail | null> {
    if (!entry.recordId || connection.value !== 'connected') return null
    const id = entry.recordId, generation = (recordGenerations.get(id) ?? 0) + 1, epoch = latestState.value?.epoch ?? null
    recordGenerations.set(id, generation)
    recordLoading.add(id)
    try {
      const result = await apiRequest<BusinessDetail>(`/api/business/${encodeURIComponent(id)}`)
      if (!active || recordGenerations.get(id) !== generation) return null
      if (result.head.payload.kind !== entry.kind || result.item.corrects_confirmation_id) throw new Error('unsupported-workspace-record')
      details.value[id] = result; recordFailures.value[id] = ''; acceptRead(result.state, epoch)
      // Do not overwrite an active editor's in-memory dirty or busy state.
      if (tab.value.selected !== entry.identity) restoreSnapshot(entry, result)
      return result
    } catch { if (active && recordGenerations.get(id) === generation) recordFailures.value[id] = '이 업무 기록을 불러오지 못했습니다. 입력과 요청은 보존했습니다.'; return null }
    finally { if (recordGenerations.get(id) === generation) recordLoading.delete(id) }
  }
  async function loadList() {
    if (connection.value !== 'connected') return
    const generation = ++listGeneration, epoch = latestState.value?.epoch ?? null
    loading.value = true; failure.value = ''
    try {
      const result = await apiRequest<BusinessList>('/api/business?status=all')
      if (!active || generation !== listGeneration) return
      summaries.value = result.items.filter(item => isWorkspaceKind(item.kind) && !item.corrects_confirmation_id)
      acceptRead(result.state, epoch)
      // A request from a normal detail page can finish after workspace mounting.
      // Refresh only inactive summaries from retained requests and actual DB status.
      for (const entry of entries.value) if (entry.identity !== tab.value.selected) restoreSnapshot(entry)
    } catch { if (active && generation === listGeneration) failure.value = '저장한 작업 목록을 불러오지 못했습니다. 이 탭의 입력은 보존했습니다.' }
    finally { if (active && generation === listGeneration) loading.value = false }
  }
  function add(kind: WorkspaceKind) {
    if (!latestState.value || connection.value !== 'connected') return null
    const entry = newWorkspaceEntry(kind, () => crypto.randomUUID())
    const next = { entries: [...tab.value.entries, entry], selected: entry.identity }
    if (!persist(next)) return null
    tab.value = next; restoreSnapshot(entry)
    return entry
  }
  async function select(entry: WorkspaceEntry) {
    const next = { entries: tab.value.entries.some(item => item.identity === entry.identity) ? tab.value.entries : [...tab.value.entries, entry], selected: entry.identity }
    tab.value = next; persist(next)
    if (entry.recordId) await loadRecord(entry)
  }
  const initial = computed<BusinessEditorInitial<WorkspacePayload> | null>(() => {
    const entry = selected.value
    if (!entry || !latestState.value) return null
    if (!entry.recordId) {
      if (!localSeeds.has(entry.identity)) localSeeds.set(entry.identity, { payload: blankWorkspacePayload(entry.kind), state: { ...latestState.value }, recordId: null, headId: null, status: 'draft', corrects: null, issues: [] })
      return localSeeds.get(entry.identity)!
    }
    const detail = details.value[entry.recordId]
    return detail && isWorkspaceKind(detail.head.payload.kind) ? { payload: detail.head.payload as WorkspacePayload, state: detail.state, recordId: detail.item.id, headId: detail.head.id, status: detail.head.status, corrects: null, issues: detail.issues } : null
  })
  function committed(identity: string, response: BusinessMutation) {
    const entry = tab.value.entries.find(item => item.identity === identity)
    if (!entry || !isWorkspaceKind(response.result.head.payload.kind)) return
    const id = response.result.item.id, old = details.value[id]
    const adoptingExisting = identity !== id && (old || snapshots.value[id] || tab.value.entries.some(item => item.identity === id))
    recordGenerations.set(id, (recordGenerations.get(id) ?? 0) + 1)
    // An uncertain create may already have been opened and edited by its real ID.
    // Adopting that ID must not reset the newer editor or replace its saved head.
    if (!adoptingExisting) {
      details.value[id] = { item: response.result.item, head: response.result.head, issues: response.result.issues, state: response.state, revisions: [...(old?.revisions ?? []), response.result.head], confirmations: response.result.confirmation ? [...(old?.confirmations ?? []).filter(item => item.id !== response.result.confirmation!.id), response.result.confirmation] : old?.confirmations ?? [], corrections: old?.corrections ?? [] }
      summaries.value = [response.result.item, ...summaries.value.filter(item => item.id !== id)]
      snapshots.value[id] = { payload: copyPayload(response.result.head.payload), status: response.result.head.status, dirty: false, busy: false, pendingAction: null, recovery: '', failure: '', issueCount: response.result.issues.length, previewState: null }
    }
    latestState.value = advanceWorkspaceState(latestState.value, response.state)
    tab.value = linkWorkspaceRecord(tab.value, identity, id); persist(tab.value)
    if (identity !== id) delete snapshots.value[identity]
    // Refresh authoritative current state; a successful replay may carry old state.
    if (active) { void loadList(); void loadRecord({ identity: id, recordId: id, kind: response.result.head.payload.kind }) }
  }
  const hosts = new Map<string, BusinessEditorHost>()
  function hostFor(identity: string): BusinessEditorHost {
    if (!hosts.has(identity)) hosts.set(identity, {
      notify: (from, snapshot) => {
        if (!tab.value.entries.some(entry => entry.identity === from)) return
        snapshots.value[from] = snapshot
        if (snapshot.previewState) latestState.value = advanceWorkspaceState(latestState.value, snapshot.previewState)
      }, committed,
      currentState: () => latestState.value,
      awaitingPrior: () => entries.value.some(entry => entry.identity !== identity && unresolvedWorkspaceRequest(snapshots.value[entry.identity])),
    })
    return hosts.get(identity)!
  }
  function refreshFor(entry: WorkspaceEntry) { return async () => { await loadList(); return entry.recordId ? loadRecord(entry) : null } }
  watch(() => tab.value.selected, () => {
    const entry = selected.value
    if (entry?.recordId && !details.value[entry.recordId] && !recordLoading.has(entry.recordId)) void loadRecord(entry)
  })
  watch(connection, () => { if (connection.value === 'connected') { void loadList(); if (selected.value?.recordId) void loadRecord(selected.value) } }, { immediate: true })
  return { tab, entries, selected, selectedDetail, summaries, snapshots, initial, latestState, loading, failure, recordFailures, storageFailure, add, select, loadList, loadRecord, hostFor, refreshFor }
}
