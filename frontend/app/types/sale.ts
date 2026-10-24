import type { CatalogState, ResolvedPrice } from './material'
import type { RecipeCostRow, RecipeCost } from './recipe'
export interface SalePayload { kind: 'sale'; version: 1; data: { material_id: string | null; material_version_id: string | null; recipe_version_id: string | null; count: string; note: string } }
export interface SaleRecipeCostRow extends RecipeCostRow { material_name: string; price: ResolvedPrice }
export interface SaleRecipeCost extends Omit<RecipeCost, 'state' | 'price_mode' | 'rows'> { rows: SaleRecipeCostRow[] }
export interface SaleOriginal { id: string; original_confirmation_id: string; business_record_id: string; material_id: string; material_version_id: string; material_name: string; recipe_version_id: string; recipe_cost: SaleRecipeCost; recorded_at: string; seq: string }
export interface SaleVersion { id: string; sale_id: string; previous_version_id: string | null; confirmation_id: string; business_record_id: string; count: string; amount: string | null; note: string; recorded_at: string; seq: string }
export interface SaleProposal { material_id: string; material_version_id: string; material_name: string; recipe_version_id: string; count: string; amount: string | null; recipe_cost: SaleRecipeCost }
export interface SaleBusinessEffect { kind: 'sale'; sale: SaleProposal; corrects_confirmation_id: string | null; previous: SaleVersion | null }
export interface SaleBusinessResult { kind: 'sale'; sale_id: string; sale_version_id: string }
export type SaleEffective = { kind: 'active'; version_id: string; count: string; amount: string | null } | { kind: 'void'; version_id: string }
export interface SaleSummary { effective?: SaleEffective; id: string; material_id: string; material_name: string; recipe_version_id: string; current_version_id: string; count: string; selling_price: string | null; amount: string | null; original_seq: string; recorded_at: string; updated_at: string; consumption_status: SaleConsumptionStatus }
export interface SaleList { items: SaleSummary[]; known_revenue: string; missing_revenue_count: number; state: CatalogState }
export type SaleConsumptionStatus = 'unconfirmed' | 'partially_settled' | 'settled' | 'not_applicable'
export interface SaleMaterialConsumption { material_id: string; material_name: string; unit: string; status: 'unconfirmed' | 'allocated' | 'unallocated'; stocktake_id: string | null; observation_id: string | null; observation_version_id: string | null; consumption_version_id: string | null; sale_version_id: string | null; quantity: string | null; value: string | null; reason: 'not_observed' | 'initial_basis_missing' | 'zero_weight' | null }
export interface SaleDetail { effective?: SaleEffective; item: SaleOriginal; head: SaleVersion; versions: SaleVersion[]; consumption: SaleMaterialConsumption[]; consumption_status: SaleConsumptionStatus; state: CatalogState }
