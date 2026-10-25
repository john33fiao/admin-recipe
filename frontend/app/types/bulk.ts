import type { CatalogInput, CatalogState, CatalogRegistration, Material } from './material'
import type { RecipeDataInput, RecipePending, RecipeOwnerInput } from './recipe'
import type { ReceiptInclusionSelection } from './reconciliation'
import type { ReceiptPayload } from './receipt'
import type { BusinessPending, BusinessPreview, BusinessIssue } from './business'

export type BulkKind = 'material' | 'menu' | 'receipt'
export interface BulkColumn { key: string; label: string; required?: boolean }
export interface BulkRow { id: string; values: Record<string, string>; reference: { material_id: string; material_version_id: string } | null; selections: ReceiptInclusionSelection[] }
export interface ReceiptInputPreview { state: CatalogState; can_confirm: boolean; issues: BusinessIssue[]; effect: BusinessPreview['effect'] }
export type BulkCommand = { kind: 'material'; input: CatalogInput } | { kind: 'menu'; owner: RecipeOwnerInput; data: RecipeDataInput } | { kind: 'receipt'; payload: ReceiptPayload }
export interface BulkItem {
  id: string; rowIds: string[]; label: string; command: BulkCommand
  effect: BusinessPreview['effect']; preview: BusinessPreview | null
  draft: { id: string; head_id: string } | null
  result: { id: string; href: string } | null
  pending: { kind: 'material'; body: CatalogRegistration } | { kind: 'menu'; request: Extract<RecipePending, { action: 'create' }> } | { kind: 'receipt'; request: BusinessPending } | null
  recovery: '' | 'retry' | 'review' | 'blocked' | 'effect'
  failure: string
}
export interface BulkReview { state: CatalogState; items: BulkItem[]; errors: Record<string, Record<string, string>>; warnings: string[] }
export interface BulkSession { version: 1; kind: BulkKind; rows: BulkRow[]; review: BulkReview | null; fingerprint: string; acceptedWarnings: boolean; duplicate: boolean }
export interface BulkContext { state: CatalogState; materials: Material[] }
