import type { CatalogState } from './material'
import type { HoldingAmounts, HoldingBasisRef, HoldingObservation } from './receipt'

export type StocktakeValueDraft = { kind: 'existing_price' } | { kind: 'manual_amount'; amount: string }
export interface StocktakeExcessDraft { reviewed: boolean; reason: string; value: StocktakeValueDraft | null }
export interface DraftStocktakeRow { material_id: string | null; material_version_id: string | null; quantity: string; unit: string; initial_value: StocktakeValueDraft | null; excess: StocktakeExcessDraft | null }
export interface StocktakePayload { kind: 'stocktake'; version: 1; data: { rows: DraftStocktakeRow[]; note: string } }
export type StocktakeValuation = { kind: 'existing_price'; source: 'holding_average' | 'last_valid' | 'preregistered'; source_id: string; amount: string; quantity: string } | { kind: 'manual_amount'; amount: string }
export interface SaleAllocationProposal { sale_id: string; sale_version_id: string; recipe_version_id: string; original_seq: string; count: string; position: number; quantity: string | null; value: string | null }
export interface ConsumptionAllocation { basis: HoldingBasisRef; end_seq: string; as_of_revision: string; total_quantity: string; total_value: string; candidates: SaleAllocationProposal[]; unallocated_quantity: string; unallocated_value: string; reason: 'no_sales' | 'zero_weight' | null }
export type StocktakeCalculation = { mode: 'initial'; value: string; valuation: StocktakeValuation } | { mode: 'settlement'; consumption_quantity: string; consumption_value: string; excess_quantity: string; excess_value: string; valuation: StocktakeValuation | null; allocation: ConsumptionAllocation }
export interface StocktakeRowEffect { position: number; material_id: string; material_version_id: string; material_name: string; unit: string; input_quantity: string; input_unit: string; measured_quantity: string; before: HoldingAmounts | null; after: HoldingAmounts; basis_before: HoldingBasisRef | null; observation_before: HoldingObservation | null; calculation: StocktakeCalculation }
export interface StocktakeBusinessEffect { kind: 'stocktake'; rows: StocktakeRowEffect[]; unchecked_positions: number[] }
export interface StocktakeBusinessResult { kind: 'stocktake'; stocktake_id: string }
export interface StocktakeRecord { id: string; confirmation_id: string; business_record_id: string; note: string; recorded_at: string; seq: string }
export interface StocktakeObservationRecord { id: string; version_id: string; previous_version_id: string | null; stocktake_id: string; previous_observation_version_id: string | null; consumption_version_id: string | null; effect: StocktakeRowEffect; recorded_at: string; seq: string }
export interface StocktakeSummary { id: string; confirmation_id: string; business_record_id: string; first_material_name: string; confirmed_count: number; recorded_at: string; seq: string }
export interface StocktakeList { items: StocktakeSummary[]; state: CatalogState }
export interface StocktakeDetail { item: StocktakeRecord; observations: StocktakeObservationRecord[]; state: CatalogState }
