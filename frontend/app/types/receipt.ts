import type { CatalogState, ResolvedPrice } from './material'
import type { OrderVersion } from './purchase-order'
import type { ReceiptInclusionSelection, ReceiptReconciliation } from './reconciliation'
export type ReceiptSourceDraft = { kind: 'direct'; material_id: string | null; material_version_id: string | null } | { kind: 'order'; order_id: string | null; expected_order_version_id: string | null; material_version_id: string | null }
export type ReceiptStockEffectDraft = { kind: 'unreflected' } | { kind: 'needs_reconciliation'; basis_id: string | null; included_quantity: string } | { kind: 'reconciled'; selections: ReceiptInclusionSelection[] }
export interface ReceiptPayload { kind: 'receipt'; version: 1; data: { source: ReceiptSourceDraft; quantity: string; unit: string; amount: string; receipt_date: string; note: string; stock_effect: ReceiptStockEffectDraft } }
export interface ReceiptRecord { id: string; confirmation_id: string; business_record_id: string; material_id: string; material_version_id: string; material_name: string; quantity: string; unit: string; input_quantity: string; input_unit: string; amount: string; receipt_date: string; note: string; recorded_at: string; seq: string; source: { kind: 'direct' } | { kind: 'order'; order_id: string; ordered_version_id: string } }
export interface ReceiptList { items: ReceiptRecord[]; state: CatalogState }
export interface OrderReceiptPreview { status: 'received'; order_id: string; initial_order_version_id: string; initial_ordered_quantity: string; ordered_version_id: string; ordered_quantity: string; received_quantity: string; difference: string; unit: string }
export interface OrderReceiptClosure extends OrderReceiptPreview { receipt_id: string; receipt_confirmation_id: string; receipt_material_version_id: string }
export interface ReceiptDetail { item: ReceiptRecord; holding_version_id: string | null; reconciliation?: ReceiptReconciliation; order_closure: OrderReceiptClosure | null; state: CatalogState }
export interface HoldingAmounts { quantity: string; value: string | null; source_id: string | null }
export interface HoldingBasisRef { kind: 'receipt' | 'observation' | 'production' | 'reconciliation'; id: string | null; seq: string | null }
export interface HoldingObservation { id: string; quantity: string; recorded_at: string; seq: string }
export interface ReceiptHoldingChange { kind: 'receipt'; event_id: string; quantity: string; value: string; recorded_at: string; seq: string }
export interface ProductionHoldingChange { kind: 'production'; event_id: string; production_id: string; input_quantity: string; input_value: string; output_quantity: string; output_value: string; recorded_at: string; seq: string }
export interface WasteHoldingChange { kind: 'waste'; event_id: string; quantity: string; loss_value: string; recorded_at: string; seq: string }
export interface ReconciliationHoldingChange { kind: 'reconciliation'; event_id: string; reconciliation_id: string; receipt_id: string; inflow_quantity: string; inflow_value: string; value_delta: string; recorded_at: string; seq: string }
export type HoldingChange = ReceiptHoldingChange | ProductionHoldingChange | WasteHoldingChange | ReconciliationHoldingChange
export interface PriceBasis { amount: string; quantity: string; source_id: string }
export interface HoldingDetail { material_id: string; material_version_id: string; unit: string; basis: HoldingBasisRef | null; observation: HoldingObservation | null; settlement_basis: HoldingAmounts | null; last_valid_price: PriceBasis | null; confirmed_changes: HoldingChange[]; price: ResolvedPrice; state: CatalogState }
export interface ReceiptBusinessResult { kind: 'receipt'; receipt_id: string; holding_version_id: string; material_id: string; material_version_id: string; order_id: string | null; order_version_id: string | null }
export interface ReceiptBusinessEffect { kind: 'receipt'; material_id: string; material_version_id: string; quantity: string; amount: string; unit: string; input_quantity: string; input_unit: string; receipt_date: string; before: HoldingAmounts | null; after: HoldingAmounts; basis_before: HoldingBasisRef | null; observation: HoldingObservation | null; order_before: OrderVersion | null; order_after: OrderReceiptPreview | null }
