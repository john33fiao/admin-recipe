import type { StocktakePayload, StocktakeBusinessEffect, StocktakeBusinessResult } from './stocktake'
import type { WastePayload, WasteBusinessEffect, WasteBusinessResult } from './waste'
import type { CatalogState, MaterialPriceVersion, MutationMeta } from './material'
import type { OrderBusinessEffect, OrderBusinessResult, OrderPayload } from './purchase-order'
import type { ProductionBusinessEffect, ProductionBusinessResult, ProductionPayload } from './production'
import type { SaleBusinessEffect, SaleBusinessResult, SalePayload } from './sale'
import type { ReceiptBusinessEffect, ReceiptBusinessResult, ReceiptPayload } from './receipt'
import type { ReconciledReceiptBusinessEffect, ReconciledReceiptBusinessResult } from './reconciliation'

export interface DraftPriceV1 { material_id: string | null; material_version_id: string | null; quantity: string; unit: string; amount: string; note: string }
export interface CatalogPricePayload { kind: 'catalog_price'; version: 1; data: DraftPriceV1 }
export type BusinessPayload = CatalogPricePayload | OrderPayload | ReceiptPayload | ProductionPayload | SalePayload | StocktakePayload | WastePayload
export type BusinessStatus = 'draft' | 'confirmed' | 'archived'
export interface BusinessIssue { field: string; code: string; message: string }
export interface BusinessRevision { id: string; record_id: string; previous_revision_id: string | null; status: BusinessStatus; payload: BusinessPayload; recorded_at: string; seq: string }
export interface BusinessConfirmation {
  id: string; record_id: string; revision_id: string; request_id: string; recorded_at: string; seq: string
  result: { kind: 'catalog_price'; price_id: string; material_id: string; material_version_id: string } | OrderBusinessResult | ReceiptBusinessResult | ReconciledReceiptBusinessResult | ProductionBusinessResult | SaleBusinessResult | StocktakeBusinessResult | WasteBusinessResult
}
export interface BusinessSummary {
  id: string; kind: BusinessPayload['kind']; head_id: string; status: BusinessStatus; created_at: string; updated_at: string
  corrects_confirmation_id: string | null; confirmation_id: string | null; effective_confirmation_id: string | null
  issue_count: number; material_name: string | null
}
export interface BusinessDetail {
  item: BusinessSummary; head: BusinessRevision; revisions: BusinessRevision[]; confirmations: BusinessConfirmation[]
  corrections: Array<{ target_confirmation_id: string; replacement_confirmation_id: string }>
  issues: BusinessIssue[]; state: CatalogState
}
export interface BusinessList { items: BusinessSummary[]; state: CatalogState }
export interface BusinessWrite { item: BusinessSummary; head: BusinessRevision; confirmation: BusinessConfirmation | null; issues: BusinessIssue[] }
export interface BusinessMutation { result: BusinessWrite; state: CatalogState }
export interface BusinessPreview {
  record_id: string; head_id: string; state: CatalogState; can_confirm: boolean; issues: BusinessIssue[]
  effect: { kind: 'catalog_price'; material_id: string; material_version_id: string; quantity: string; amount: string; unit: string; corrects_confirmation_id: string | null; previous_price: MaterialPriceVersion | null; corrected_price: MaterialPriceVersion | null } | OrderBusinessEffect | ReceiptBusinessEffect | ReconciledReceiptBusinessEffect | ProductionBusinessEffect | SaleBusinessEffect | StocktakeBusinessEffect | WasteBusinessEffect | null
}
export type BusinessPending =
  | { action: 'create'; record_id: null; body: { meta: MutationMeta; payload: BusinessPayload; corrects_confirmation_id: string | null } }
  | { action: 'revise'; record_id: string; body: { meta: MutationMeta; expected_head_id: string; payload: BusinessPayload } }
  | { action: 'confirm' | 'archive'; record_id: string; body: { meta: MutationMeta; expected_head_id: string } }
