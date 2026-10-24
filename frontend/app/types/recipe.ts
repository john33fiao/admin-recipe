import type { CatalogInput, CatalogState, Material, MutationMeta, ResolvedPrice } from './material'

export interface RecipeReferenceInput { quantity: string; unit: string }
export interface RecipeLineInput { material_id: string; material_version_id: string; quantity: string; unit: string; note: string }
export interface RecipeDataInput {
  basis_count: string
  reference_output: RecipeReferenceInput | null
  selling_price: string | null
  cooking_description: string
  process: string
  reference_time: string
  lines: RecipeLineInput[]
}
export type RecipeOwnerInput = { mode: 'existing'; material_id: string; expected_material_version_id: string } | ({ mode: 'new' } & CatalogInput)
export interface RecipeReferenceOutput extends RecipeReferenceInput { input_quantity: string; input_unit: string }
export interface RecipeLine extends RecipeLineInput { id: string; position: string; material_name: string; input_quantity: string; input_unit: string }
export interface RecipeVersion {
  id: string; material_id: string; material_version_id: string; material_name: string; material_unit: string
  previous_version_id: string | null; copied_from_version_id: string | null
  basis_count: string; reference_output: RecipeReferenceOutput | null; selling_price: string | null
  cooking_description: string; process: string; reference_time: string
  lines: RecipeLine[]; recorded_at: string; seq: string
}
export type RecipeVersionSummary = Pick<RecipeVersion, 'id' | 'material_version_id' | 'previous_version_id' | 'copied_from_version_id' | 'basis_count' | 'recorded_at' | 'seq'>
export interface RecipeSummary { item: Material; recipe_version_id: string; updated_at: string }
export interface RecipeList { items: RecipeSummary[]; state: CatalogState }
export interface RecipeDetail { item: Material; head: RecipeVersion; versions: RecipeVersionSummary[]; state: CatalogState }
export interface RecipeMutation { item: Material; version: RecipeVersion; state: CatalogState }
export interface RecipeCostRow { line_id: string; material_id: string; material_version_id: string; numerator_quantity: string; basis_count: string; unit: string; price: ResolvedPrice; cost: string | null }
export interface RecipeCost {
  recipe_version_id: string; basis_count: string; price_mode: 'current'; rows: RecipeCostRow[]
  missing_price_line_ids: string[]; known_subtotal: string; total_cost: string | null; selling_price: string | null
  cost_ratio: { numerator: string; denominator: string } | null; ratio_unavailable_reason: 'missing_selling_price' | 'zero_selling_price' | 'missing_cost' | null
  state: CatalogState
}
export type RecipePending =
  | { action: 'create'; material_id: null; body: { meta: MutationMeta; owner: RecipeOwnerInput; data: RecipeDataInput } }
  | { action: 'revise'; material_id: string; body: { meta: MutationMeta; expected_material_version_id: string; expected_recipe_version_id: string; data: RecipeDataInput } }
  | { action: 'copy'; material_id: string; body: { meta: MutationMeta; expected_material_version_id: string; source_recipe_version_id: string } }
