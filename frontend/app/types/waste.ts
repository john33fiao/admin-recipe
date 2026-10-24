import type { CatalogState } from './material'
import type { HoldingAmounts, HoldingBasisRef, HoldingObservation } from './receipt'
export type WasteReason = 'discard' | 'staff_meal'
export interface WastePayload { kind: 'waste'; version: 1; data: { material_id: string | null; material_version_id: string | null; quantity: string; unit: string; reason: WasteReason; note: string } }
export interface WasteEffect { material_id: string; material_version_id: string; material_name: string; unit: string; input_quantity: string; input_unit: string; quantity: string; reason: WasteReason; before: HoldingAmounts; after: HoldingAmounts; basis_before: HoldingBasisRef; observation: HoldingObservation | null; loss_value: string }
export interface WasteBusinessEffect { kind: 'waste'; waste: WasteEffect }
export interface WasteBusinessResult { kind: 'waste'; waste_id: string }
export interface WasteRecord { id: string; confirmation_id: string; business_record_id: string; effect: WasteEffect; note: string; recorded_at: string; seq: string }
export interface WasteList { items: WasteRecord[]; state: CatalogState }
export interface WasteDetail { item: WasteRecord; state: CatalogState }
