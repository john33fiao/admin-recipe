import type { BusinessMutation, BusinessPayload, BusinessPending, BusinessStatus } from './business'
import type { CatalogState } from './material'

export type WorkspaceKind = 'sale' | 'production' | 'waste' | 'stocktake'
export type WorkspacePayload = Extract<BusinessPayload, { kind: WorkspaceKind }>
export interface BusinessEditorSnapshot {
  payload: BusinessPayload
  status: BusinessStatus
  dirty: boolean
  busy: boolean
  pendingAction: BusinessPending['action'] | null
  recovery: string
  failure: string
  issueCount: number
  previewState: CatalogState | null
}
export interface BusinessEditorHost {
  notify: (identity: string, snapshot: BusinessEditorSnapshot) => void
  committed: (identity: string, response: BusinessMutation) => void
  currentState: () => CatalogState | null
  awaitingPrior: () => boolean
}
export interface WorkspaceEntry {
  identity: string
  kind: WorkspaceKind
  recordId: string | null
}
export interface WorkspaceTab { entries: WorkspaceEntry[]; selected: string | null }
