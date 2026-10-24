import type { CatalogState } from './material'
import type { HoldingAmounts, HoldingBasisRef, HoldingObservation } from './receipt'

export interface DraftProductionInput { material_id: string | null; material_version_id: string | null; quantity: string; unit: string }
export interface DraftProductionOutput extends DraftProductionInput { allocation_percent: string }
export interface ProductionPayload { kind: 'production'; version: 1; data: { inputs: DraftProductionInput[]; outputs: DraftProductionOutput[]; note: string } }
export interface ProductionInputRow { position: string; material_id: string; material_version_id: string; material_name: string; quantity: string; unit: string; input_quantity: string; input_unit: string; value: string }
export interface ProductionOutputRow extends ProductionInputRow { allocation_percent: string }
export interface ProductionMaterialEffect { material_id: string; material_version_id: string; unit: string; before: HoldingAmounts | null; after: HoldingAmounts; basis_before: HoldingBasisRef | null; observation: HoldingObservation | null; input_quantity: string; input_value: string; output_quantity: string; output_value: string }
export interface ProductionPreview { inputs: ProductionInputRow[]; outputs: ProductionOutputRow[]; materials: ProductionMaterialEffect[]; total_input_value: string }
export interface ProductionBusinessEffect extends ProductionPreview { kind: 'production' }
export interface ProductionBusinessResult { kind: 'production'; production_id: string }
export interface ProductionRecord { id: string; confirmation_id: string; business_record_id: string; note: string; recorded_at: string; seq: string; total_input_value: string; inputs: ProductionInputRow[]; outputs: ProductionOutputRow[] }
export interface ProductionMaterialResult extends ProductionMaterialEffect { id: string }
export interface ProductionDetail { item: ProductionRecord; materials: ProductionMaterialResult[]; state: CatalogState }
export interface ProductionSummary { id: string; confirmation_id: string; business_record_id: string; first_output_name: string; total_input_value: string; recorded_at: string; seq: string; input_count: number; output_count: number }
export interface ProductionList { items: ProductionSummary[]; state: CatalogState }
