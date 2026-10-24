import type { CatalogState } from './material'
import type { CorrectionPlanRef } from './correction'
import type { HoldingAmounts, HoldingBasisRef, HoldingObservation, ReceiptRecord } from './receipt'

export interface DraftPurchaseReturnV1 { receipt_id: string | null; expected_receipt_version_id: string | null; quantity: string; unit: string; note: string }
export interface PurchaseReturnPayload { kind: 'purchase_return'; version: 1; data: DraftPurchaseReturnV1 }
export interface PurchaseReturnSourceState { reference: CorrectionPlanRef; material_id: string; material_version_id: string; material_name: string; unit: string; quantity: string; amount: string; returned_quantity: string; remaining_quantity: string; logical_seq: string; seq: string }
export interface PurchaseReturnSource { original: ReceiptRecord; current: PurchaseReturnSourceState | null; state: CatalogState }
export interface PurchaseReturnReceiptBasis { reference: CorrectionPlanRef; original_quantity: string; original_amount: string; original_unit: string; quantity: string; amount: string; returned_quantity: string; remaining_quantity: string }
export interface PurchaseReturnEffect { source: PurchaseReturnReceiptBasis; material_id: string; material_version_id: string; material_name: string; unit: string; input_quantity: string; input_unit: string; quantity: string; before: HoldingAmounts; after: HoldingAmounts; basis_before: HoldingBasisRef; observation: HoldingObservation | null; deducted_value: string }
export interface PurchaseReturnRecord { id: string; confirmation_id: string; business_record_id: string; effect: PurchaseReturnEffect; note: string; recorded_at: string; seq: string }
export type PurchaseReturnEffectiveState = { kind: 'active'; version_id: string; effect: PurchaseReturnEffect; note: string } | { kind: 'void'; version_id: string }
export interface PurchaseReturnSummary { item: PurchaseReturnRecord; effective?: PurchaseReturnEffectiveState }
export interface PurchaseReturnList { items: PurchaseReturnSummary[]; state: CatalogState }
export interface PurchaseReturnDetail extends PurchaseReturnSummary { state: CatalogState }
export interface PurchaseReturnBusinessEffect { kind: 'purchase_return'; purchase_return: PurchaseReturnEffect }
export interface PurchaseReturnBusinessResult { kind: 'purchase_return'; purchase_return_id: string }
