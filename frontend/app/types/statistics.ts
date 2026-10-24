import type { CatalogState, Material } from './material'
import type { CorrectionEventRef, CorrectionEventValue, CorrectionSubjectKind } from './correction'
import type { SaleOriginal } from './sale'
import type { ConsumptionAllocation, StocktakeRowEffect } from './stocktake'
import type { RecipeLine } from './recipe'

export interface AppliedStatisticsRange { from: string; until: string; zone: string }
export interface StatisticsRange { from: string; until: string; time_basis: 'original_recorded_at'; correction_basis: 'current' }
export interface StatisticsCoverage { matching_records: number; all_time_records: number }
export interface StatisticsOrigin { kind: CorrectionSubjectKind; id: string; confirmation_id: string; business_record_id: string; recorded_at: string; seq: string }
export interface StatisticsChangeRef { kind: 'correction' | 'legacy_sale_revision'; id: string; business_record_id: string; confirmation_id: string; recorded_at: string; seq: string }
export interface StatisticsSaleConsumption {
  material_id: string; material_name: string; unit: string; status: 'unconfirmed' | 'allocated' | 'unallocated'
  observation_id: string | null; observation_version_id: string | null; stocktake_id: string | null; consumption_version_id: string | null; sale_version_id: string | null
  quantity: string | null; value: string | null; reason: 'no_effective_allocation' | 'initial_basis_missing' | 'zero_weight' | null
  observation_recorded_at: string | null; observation_in_period: boolean | null
}
export interface StatisticsSale { origin: StatisticsOrigin; effective_ref: CorrectionEventRef; changes: StatisticsChangeRef[]; status: 'active' | 'void'; original: SaleOriginal; value: { count: string; amount: string | null } | null; consumption: StatisticsSaleConsumption[] }
export interface SalesTotals { active_records: number; void_records: number; count: string; known_revenue: string; missing_revenue_records: number }
export interface StatisticsCandidateLink { position: number; sale_id: string; business_record_id: string; original_sale_recorded_at: string; sale_in_period: boolean }
export interface StatisticsObservation {
  origin: StatisticsOrigin; observation_id: string; material_id: string; material_version_id: string; material_name: string; unit: string; position: number
  effective_observation_version_id: string | null; tombstone_projection_id: string | null; changes: StatisticsChangeRef[]; status: 'initial' | 'settlement' | 'void'
  effect: StocktakeRowEffect | null; consumption_version_id: string | null; allocation: ConsumptionAllocation | null; candidate_links: StatisticsCandidateLink[]
}
export interface MaterialConsumptionTotals {
  material_id: string; material_name: string; label_material_version_id: string; unit: string; initial_records: number; settlement_records: number; void_records: number
  consumption_quantity: string; consumption_value: string; allocated_quantity: string; allocated_value: string; unallocated_quantity: string; unallocated_value: string; no_sales_records: number; zero_weight_records: number
}
export interface StatisticsReceiptBasis { material_id: string; material_version_id: string; material_name: string; quantity: string; unit: string; amount: string; receipt_date: string }
export interface StatisticsOperation { origin: StatisticsOrigin; effective_ref: CorrectionEventRef; changes: StatisticsChangeRef[]; status: 'active' | 'void'; value: CorrectionEventValue | null; receipt_basis: { original: StatisticsReceiptBasis; effective: StatisticsReceiptBasis | null } | null; observation_ids: string[] }
export interface StatisticsReport {
  range: StatisticsRange; state: CatalogState
  sales: { coverage: StatisticsCoverage; summary: SalesTotals; rows: StatisticsSale[] }
  observations: { coverage: StatisticsCoverage; summary_by_material: MaterialConsumptionTotals[]; rows: StatisticsObservation[] }
  operations: { coverage: StatisticsCoverage; rows: StatisticsOperation[] }
}
export interface RecipeConnectionStep { recipe_material_id: string; recipe_version_id: string; basis_count: string; line: RecipeLine; child_recipe_version_id: string | null }
export interface RecipeConnections { item: Material; root_recipe_version_id: string; basis: 'current_root_current_children'; direct: RecipeConnectionStep[]; paths: Array<{ steps: RecipeConnectionStep[] }>; state: CatalogState }
