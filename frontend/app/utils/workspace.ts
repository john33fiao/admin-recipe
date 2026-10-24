import type { BusinessEditorSnapshot, WorkspaceEntry, WorkspaceKind, WorkspacePayload, WorkspaceTab } from '../types/workspace'
import type { CatalogState } from '../types/material'
import type { BusinessConfirmation } from '../types/business'
import { blankSaleDraft } from './sale.ts'
import { blankProductionDraft } from './production.ts'
import { blankStocktakeDraft, blankWasteDraft } from './stocktake.ts'

export const workspaceKinds: Record<WorkspaceKind, string> = { sale: '판매', production: '생산', waste: '폐기 · 직원식', stocktake: '실사' }
export const isWorkspaceKind = (kind: unknown): kind is WorkspaceKind => typeof kind === 'string' && Object.hasOwn(workspaceKinds, kind)
const uuid = (value: unknown): value is string => typeof value === 'string' && /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(value)
export function blankWorkspacePayload(kind: WorkspaceKind): WorkspacePayload {
  return kind === 'sale' ? blankSaleDraft() : kind === 'production' ? blankProductionDraft() : kind === 'stocktake' ? blankStocktakeDraft() : blankWasteDraft()
}
export function newWorkspaceEntry(kind: WorkspaceKind, generate: () => string): WorkspaceEntry { return { identity: `workspace.local.${generate()}`, kind, recordId: null } }
export function restoreWorkspaceTab(text: string | null): WorkspaceTab | null {
  try {
    const value = JSON.parse(text ?? 'null')
    if (!value || !Array.isArray(value.entries) || !(value.selected === null || typeof value.selected === 'string')) return null
    const ids = new Set<string>(), entries: WorkspaceEntry[] = []
    for (const entry of value.entries) {
      if (!entry || !isWorkspaceKind(entry.kind) || typeof entry.identity !== 'string' || ids.has(entry.identity)) return null
      if (entry.recordId === null ? !entry.identity.startsWith('workspace.local.') || !uuid(entry.identity.slice('workspace.local.'.length)) : !uuid(entry.recordId) || entry.identity !== entry.recordId) return null
      ids.add(entry.identity); entries.push({ identity: entry.identity, kind: entry.kind, recordId: entry.recordId })
    }
    return { entries, selected: ids.has(value.selected) ? value.selected : null }
  } catch { return null }
}
export function linkWorkspaceRecord(tab: WorkspaceTab, identity: string, recordId: string): WorkspaceTab {
  const entry = tab.entries.find(item => item.identity === identity)
  if (!entry) return tab
  return { entries: tab.entries.filter(item => item.identity !== recordId || item.identity === identity).map(item => item.identity === identity ? { ...entry, identity: recordId, recordId } : item), selected: tab.selected === identity ? recordId : tab.selected }
}
export function advanceWorkspaceState(current: CatalogState | null, incoming: CatalogState): CatalogState {
  if (!current) return incoming
  // A replay can return an older epoch or revision. Only a fresh GET may replace the epoch.
  return current.epoch === incoming.epoch && BigInt(incoming.revision) > BigInt(current.revision) ? incoming : current
}
export function unresolvedWorkspaceRequest(snapshot: BusinessEditorSnapshot | undefined) {
  return !!snapshot?.pendingAction && (snapshot.busy || snapshot.recovery === '' || snapshot.recovery === 'retry' || snapshot.recovery === 'blocked')
}
export function workspaceStatus(snapshot: BusinessEditorSnapshot | undefined, savedStatus: string | undefined, recordId: string | null) {
  if (snapshot?.busy && snapshot.pendingAction) return '처리 중'
  if (unresolvedWorkspaceRequest(snapshot)) return '처리 결과 확인 필요'
  if (snapshot?.failure) return '입력 검토 필요'
  if (snapshot?.status === 'confirmed' || savedStatus === 'confirmed') return snapshot?.dirty ? '확정됨 · 미저장 입력 남음' : '확정됨'
  if (snapshot?.status === 'archived' || savedStatus === 'archived') return '보관됨'
  if (!recordId) return '이 탭에만 입력'
  return snapshot?.dirty ? '수정 내용 미저장' : '임시 저장됨'
}
export function workspaceResultLink(result: BusinessConfirmation['result'] | undefined) {
  if (!result) return null
  if (result.kind === 'sale') return { to: `/sales/${result.sale_id}`, label: '판매와 소모 배분 보기' }
  if (result.kind === 'production') return { to: `/productions/${result.production_id}`, label: '생산 결과 보기' }
  if (result.kind === 'waste') return { to: `/wastes/${result.waste_id}`, label: '폐기 · 직원식 결과 보기' }
  if (result.kind === 'stocktake') return { to: `/stocktakes/${result.stocktake_id}`, label: '실사와 배분 결과 보기' }
  return null
}
