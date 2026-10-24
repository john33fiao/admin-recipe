import type { CatalogState } from './material'
import type { OrderVersion } from './purchase-order'
import type { HoldingAmounts, HoldingBasisRef, HoldingObservation, OrderReceiptPreview } from './receipt'

export interface ReceiptSourceKey { kind: 'receipt_inflow' | 'observation_initial' | 'observation_excess'; id: string; position: 0 }
export type InclusionSourceSelection = { kind: 'all_available' } | { kind: 'source'; source_key: ReceiptSourceKey }
export interface ReceiptInclusionSelection { observation_version_id: string | null; source: InclusionSourceSelection | null; included_quantity: string }
export interface SourceShare { numerator: string; denominator: string; unit: string }
export interface ReceiptInclusionSource {
  source_key: ReceiptSourceKey; source_version_id: string; source_material_id: string; source_material_name: string
  source_unit: string; source_quantity: string; reserved_source_share: SourceShare; remaining_source_share: SourceShare
  available_share: SourceShare; available_quantity: string
}
export interface ReceiptInclusionObservation {
  stocktake_id: string; observation_id: string; observation_version_id: string; material_id: string; material_version_id: string
  material_name: string; unit: string; quantity: string; value: string; available_share: SourceShare; available_quantity: string
  recorded_at: string; seq: string; sources: ReceiptInclusionSource[]
}
export interface ReceiptInclusionCandidates { items: ReceiptInclusionObservation[]; state: CatalogState }
export interface InclusionPortionEffect {
  position: number; source_key: ReceiptSourceKey; source_version_id: string; included_share: SourceShare
  reserved_source_share: SourceShare; booked_value: string; actual_included_value: string
}
export interface ReceiptInclusionEffect {
  position: number; stocktake_id: string; observation_id: string; observation_version_id: string; source: InclusionSourceSelection
  included_quantity: string; unit: string; booked_value: string; actual_included_value: string; value_difference: string; portions: InclusionPortionEffect[]
}
export interface ReconciliationMaterialEffect {
  material_id: string; material_version_id: string; material_name: string; unit: string; before: HoldingAmounts | null; after: HoldingAmounts
  basis_before: HoldingBasisRef | null; observation: HoldingObservation | null; value_delta: string; inflow_quantity: string; inflow_value: string
}
export interface ReconciliationValueTrace {
  kind: 'inflow' | 'observation' | 'production' | 'waste'; event_id: string; seq: string; material_id: string
  quantity_before: string; quantity_after: string; base_value_before: string; base_value_after: string
  proposed_value_before: string; proposed_value_after: string; base_closed_value: string; proposed_closed_value: string
}
export interface ReconciledReceiptEffect {
  material_id: string; material_version_id: string; material_name: string; quantity: string; amount: string; unit: string
  input_quantity: string; input_unit: string; receipt_date: string; included_quantity: string; unreflected_quantity: string
  included_amount: string; unreflected_amount: string; booked_value: string; value_difference: string; separate_adjustment: string
  selections: ReceiptInclusionEffect[]; materials: ReconciliationMaterialEffect[]; trace: ReconciliationValueTrace[]
  order_before: OrderVersion | null; order_after: OrderReceiptPreview | null
}
export interface ReceiptReconciliation {
  id: string; receipt_id: string; confirmation_id: string; business_record_id: string; effect: ReconciledReceiptEffect
  selection_results: Array<{ selection_position: number; portion_position: number; link_id: string; link_version_id: string; reservation_id: string }>
  material_results: Array<{ material_id: string; effect_id: string; holding_version_id: string }>; recorded_at: string; seq: string
}
export interface ReconciledReceiptBusinessEffect { kind: 'reconciled_receipt'; receipt: ReconciledReceiptEffect }
export interface ReconciledReceiptBusinessResult { kind: 'reconciled_receipt'; receipt_id: string; reconciliation_id: string; material_id: string; material_version_id: string; order_id: string | null; order_version_id: string | null }
