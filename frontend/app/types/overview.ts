import type { CatalogState } from './material'
import type { BusinessPayload } from './business'
import type { OrderSummary } from './purchase-order'
import type { OrderReceiptClosure } from './receipt'
import type { CorrectionEventRef } from './correction'
import type { StatisticsOrigin, StatisticsChangeRef, StatisticsReceiptBasis } from './statistics'

export type OverviewSection = 'materials' | 'material_changes' | 'open_orders' | 'received_orders' | 'receipts' | 'sales' | 'observations' | 'unallocated' | 'unconfirmed' | 'drafts'
export interface OverviewScope { section: OverviewSection; material_id: string | null }
export interface OverviewTarget extends OverviewScope { offset: string }
export interface OverviewPage<T> { total_count: number; returned_count: number; offset: number; page_size: number; next_offset: number | null; rows: T[] }
export interface OverviewEffectMaterial { id: string; version_id: string; name: string; unit: string }
export interface OverviewMaterialRef extends OverviewEffectMaterial { archived: boolean }
export interface OverviewConsumption { consumption_version_id: string; quantity: string; value: string; excess_quantity: string; excess_value: string; unallocated_quantity: string; unallocated_value: string; reason: 'no_sales' | 'zero_weight' | null; candidate_count: number }
export interface OverviewObservation { origin: StatisticsOrigin; observation_id: string; observation_version_id: string; position: number; material_id: string; material_version_id: string; material_name: string; unit: string; quantity: string; observed_at: string; logical_seq: string; mode: 'initial' | 'settlement'; consumption: OverviewConsumption | null; changes: StatisticsChangeRef[] }
export type OverviewChangeBoundary = { kind: 'observation'; observation_id: string; observation_version_id: string; stocktake_id: string; logical_seq: string; observed_at: string } | { kind: 'not_observed' }
export interface OverviewMaterial { material: OverviewMaterialRef; last_observation: OverviewObservation | null; changes_boundary: OverviewChangeBoundary; changes_after_observation: OverviewPage<OverviewMaterialChange> }
export type OverviewMaterialEffect =
  | { kind: 'receipt'; quantity: string; amount: string; receipt_date: string }
  | { kind: 'reconciled_receipt'; receipt: StatisticsReceiptBasis; included_quantity: string; included_amount: string; inflow_quantity: string; inflow_value: string; value_delta: string; separate_adjustment: string }
  | { kind: 'production'; input_quantity: string; input_value: string; output_quantity: string; output_value: string }
  | { kind: 'purchase_return'; quantity: string; deducted_value: string; source_receipt_id: string }
  | { kind: 'waste'; quantity: string; loss_value: string }
export interface OverviewMaterialChange { origin: StatisticsOrigin; effective_ref: CorrectionEventRef; changes: StatisticsChangeRef[]; material: OverviewEffectMaterial; effect: OverviewMaterialEffect }
export interface OverviewReceipt { origin: StatisticsOrigin; effective_ref: CorrectionEventRef; changes: StatisticsChangeRef[]; original: StatisticsReceiptBasis; effective: StatisticsReceiptBasis; reconciliation: { included_quantity: string; included_amount: string; unreflected_quantity: string; unreflected_amount: string; value_difference: string; separate_adjustment: string; affected_material_count: number; selection_count: number } | null }
export interface OverviewSale { origin: StatisticsOrigin; effective_ref: CorrectionEventRef; changes: StatisticsChangeRef[]; material_id: string; material_version_id: string; material_name: string; recipe_version_id: string; count: string; amount: string | null; consumption_counts: { allocated: number; unallocated: number; unconfirmed: number } }
export interface OverviewReceivedOrder { order: OrderSummary; closure: OrderReceiptClosure }
export interface OverviewUnallocated { observation: OverviewObservation }
export interface OverviewUnconfirmed { sale: { origin: StatisticsOrigin; effective_ref: CorrectionEventRef; material_id: string; material_name: string; count: string }; material: OverviewEffectMaterial; reason: 'no_effective_allocation' | 'initial_basis_missing'; initial_observation: { stocktake_id: string; observation_id: string; observation_version_id: string; observed_at: string } | null }
export interface OverviewDraft { id: string; kind: BusinessPayload['kind']; head_id: string; created_at: string; updated_at: string; corrects_confirmation_id: string | null; material_name: string | null; validation: 'not_evaluated' }
export interface OverviewPages { materials: OverviewPage<OverviewMaterial>; open_orders: OverviewPage<OrderSummary>; received_orders: OverviewPage<OverviewReceivedOrder>; receipts: OverviewPage<OverviewReceipt>; sales: OverviewPage<OverviewSale>; observations: OverviewPage<OverviewObservation>; unallocated: OverviewPage<OverviewUnallocated>; unconfirmed: OverviewPage<OverviewUnconfirmed>; drafts: OverviewPage<OverviewDraft> }
export interface OperationsOverview extends OverviewPages { state: CatalogState; order_counts: { open: number; received: number; cancelled: number } }
export type OverviewList = { [K in keyof OverviewPages]: { section: K; page: OverviewPages[K] } }[keyof OverviewPages] | { section: 'material_changes'; material: OverviewMaterialRef; boundary: OverviewChangeBoundary; page: OverviewPage<OverviewMaterialChange> }
export interface OverviewListResponse { state: CatalogState; scope: OverviewScope; list: OverviewList }
