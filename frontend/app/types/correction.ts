import type { BusinessConfirmation } from './business'
import type { CatalogState, ResolvedPrice } from './material'
import type { ProductionPayload, ProductionPreview } from './production'
import type { HoldingAmounts, ReceiptPayload, OrderReceiptPreview } from './receipt'
import type { ReconciledReceiptEffect, ReceiptSourceKey, SourceShare } from './reconciliation'
import type { SaleProposal } from './sale'
import type { ConsumptionAllocation, StocktakeExcessDraft, StocktakeRowEffect, StocktakeValueDraft } from './stocktake'
import type { WastePayload, WasteEffect } from './waste'

export type ObservationCorrectionAction = { kind: 'void' } | { kind: 'replace'; quantity: string; unit: string; initial_value: StocktakeValueDraft | null; excess: StocktakeExcessDraft | null }
export interface StocktakeCorrectionRow { observation_id: string | null; expected_version_id: string | null; action: ObservationCorrectionAction }
export type CorrectionReplacement = { kind: 'receipt'; data: ReceiptPayload['data'] } | { kind: 'production'; data: ProductionPayload['data'] } | { kind: 'sale_count'; data: { count: string } } | { kind: 'waste'; data: WastePayload['data'] } | { kind: 'stocktake'; data: { rows: StocktakeCorrectionRow[] } }
export interface ObservationReview { observation_id: string | null; expected_version_id: string | null; initial_value: StocktakeValueDraft | null; excess: StocktakeExcessDraft | null }
export interface LinkedReceiptChange { receipt_id: string | null; expected_version_id: string | null; action: { kind: 'void' } | { kind: 'replace'; data: ReceiptPayload['data'] } }
export interface DraftCorrectionV1 {
  expected_target_version_id: string | null; reason: string
  action: { kind: 'void' } | { kind: 'replace'; replacement: CorrectionReplacement }
  observation_reviews: ObservationReview[]; linked_receipt_changes: LinkedReceiptChange[]
}
export interface CorrectionPayload { kind: 'correction'; version: 1; data: DraftCorrectionV1 }
export type CorrectionSubjectKind = CorrectionReplacement['kind']
export interface CorrectionEventRef {
  kind: CorrectionSubjectKind; original_id: string; original_confirmation_id: string; confirmation_id: string; version_id: string
  business_record_id: string; logical_seq: string; logical_position: number; original_recorded_at: string; recorded_at: string; seq: string
}
export type CorrectionRefKind = 'receipt' | 'production' | 'sale' | 'waste' | 'stocktake' | 'observation' | 'inclusion_link' | 'material' | 'price_basis'
export type CorrectionPlanRef = { kind: 'existing'; entity: CorrectionRefKind; id: string; version_id: string } | { kind: 'proposed'; entity: CorrectionRefKind; id: string; expected_version_id: string | null }
export interface CorrectionInclusionSourceCandidate { source_key: ReceiptSourceKey; source_ref: CorrectionPlanRef; source_material_id: string; source_material_name: string; source_unit: string; source_quantity: string; reserved_source_share: SourceShare; remaining_source_share: SourceShare; available_share: SourceShare; available_quantity: string }
export interface CorrectionInclusionCandidate { observation: CorrectionPlanRef; stocktake_id: string; material_id: string; material_version_id: string; material_name: string; unit: string; quantity: string; value: string; available_share: SourceShare; available_quantity: string; logical_seq: string; sources: CorrectionInclusionSourceCandidate[] }
export type CorrectionEventValue = { kind: 'receipt'; input: ReceiptPayload['data']; reconciliation: ReconciledReceiptEffect | null } | { kind: 'production'; input: ProductionPayload['data']; result: ProductionPreview } | { kind: 'sale_count'; input: { count: string }; result: SaleProposal } | { kind: 'waste'; input: WastePayload['data']; result: WasteEffect } | { kind: 'stocktake'; input: { rows: StocktakeCorrectionRow[] } }
export interface CorrectionObservationBoundary { reference: CorrectionPlanRef; stocktake_id: string; logical_seq: string; observed_at: string; quantity: string }
export interface CorrectionPriceBasis { amount: string; quantity: string; source: CorrectionPlanRef; logical_seq: string }
export type CorrectionHoldingState = { kind: 'not_established' } | { kind: 'known'; amounts: HoldingAmounts; projection_seq: string | null; last_valid_price: CorrectionPriceBasis | null; observation_boundary: CorrectionObservationBoundary | null; initial_boundary: { reference: CorrectionPlanRef; logical_seq: string } | null }
export type CorrectionReviewRequest = { kind: 'observation_valuation'; field: string; observation: CorrectionPlanRef; material_id: string; material_name: string; unit: string; measured_quantity: string; proposed_before: CorrectionHoldingState; proposed_price: ResolvedPrice; mode: 'initial' | 'excess'; excess_quantity: string | null; message: string } | { kind: 'receipt_inclusions'; field: string; receipt_id: string; expected_version_id: string; candidates: CorrectionInclusionCandidate[]; message: string }
export interface CorrectionPlan {
  target: CorrectionEventRef
  events: Array<{ reference: CorrectionEventRef; before: CorrectionEventValue | null; after: CorrectionEventValue | null }>
  materials: Array<{ material_id: string; material_version_id: string; material_name: string; unit: string; before: CorrectionHoldingState; after: CorrectionHoldingState; quantity_delta: string; value_delta: string }>
  observations: Array<{ reference: CorrectionPlanRef; stocktake_id: string; logical_seq: string; before: StocktakeRowEffect | null; after: StocktakeRowEffect | null; valuation_basis: CorrectionPriceBasis | null }>
  allocations: Array<{ observation: CorrectionPlanRef; expected_consumption_version_id: string | null; before: ConsumptionAllocation | null; after: ConsumptionAllocation | null }>
  inclusions: Array<{ receipt_id: string; expected_version_id: string; before: ReconciledReceiptEffect | null; after: ReconciledReceiptEffect | null }>
  separate_adjustments: Array<{ receipt_id: string; before: string; after: string; delta: string }>
  order_closures: Array<{ order_id: string; expected_order_version_id: string; before: OrderReceiptPreview | null; after: OrderReceiptPreview | null }>
  review_requests: CorrectionReviewRequest[]
}
export interface CorrectionSeedVersion { reference: CorrectionEventRef; input: CorrectionReplacement | null; value: CorrectionEventValue | null }
export interface CorrectionSeed { requested_confirmation_id: string; requested: CorrectionSeedVersion; original: CorrectionSeedVersion; current: CorrectionSeedVersion; replacement_template: CorrectionReplacement; history: Array<{ confirmation: BusinessConfirmation; reason: string }>; draft: DraftCorrectionV1; state: CatalogState }
export interface CorrectionRecord { id: string; original_id: string; target_confirmation_id: string; previous_version_id: string | null; confirmation_id: string; business_record_id: string; reason: string; input: DraftCorrectionV1; plan: CorrectionPlan; resolved_refs: Array<{ reference: CorrectionPlanRef; version_id: string }>; recorded_at: string; seq: string }
export interface CorrectionDetail { item: CorrectionRecord; versions: CorrectionRecord[]; state: CatalogState }
export interface CorrectionBusinessEffect { kind: 'correction'; correction: CorrectionPlan }
export interface CorrectionBusinessResult { kind: 'correction'; correction_id: string; original_id: string; target_confirmation_id: string }
