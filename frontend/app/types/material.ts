export interface Material {
  id: string
  name: string
  unit: string
  created_at: string
  version_id: string
  canonical_unit: string
  dimension: 'mass' | 'volume' | 'custom'
  kind: MaterialKind
  description: string
  archived: boolean
  updated_at: string
  price: ResolvedPrice
}

export interface MaterialInput {
  name: string
  unit: string
}

export interface PendingMaterial extends MaterialInput {
  request_id: string
}

export type MaterialKind = 'raw' | 'prepared' | 'stocked' | 'menu' | 'unclassified'
export interface CatalogState { epoch: string; revision: string }
export interface MutationMeta { request_id: string; epoch: string; expected_revision: string }
export interface CatalogInput extends MaterialInput { kind: MaterialKind; description: string }
export interface CatalogRegistration extends CatalogInput, MutationMeta {}
export type RegistrationRequest = PendingMaterial | CatalogRegistration
export interface PriceInput { amount: string; quantity: string; unit: string }
export interface ResolvedPrice {
  source: 'holding_average' | 'last_valid' | 'preregistered' | 'missing'
  source_id: string | null
  amount: string | null
  quantity: string | null
  reason: string | null
}
export interface MaterialVersion extends CatalogInput {
  id: string
  material_id: string
  canonical_unit: string
  dimension: string
  archived: boolean
  recorded_at: string
  seq: string | null
  source: string
}
export interface MaterialPriceVersion extends PriceInput {
  id: string
  material_id: string
  material_version_id: string
  input_quantity: string
  input_unit: string
  recorded_at: string
  seq: string
}
export interface MaterialDetail extends Material, CatalogState { versions: MaterialVersion[]; prices: MaterialPriceVersion[] }
export type MutationResult<T> = T & CatalogState
export type MaterialMutation =
  | { action: 'version'; body: CatalogInput & MutationMeta & { expected_version_id: string } }
  | { action: 'price'; body: PriceInput & MutationMeta & { expected_version_id: string } }
  | { action: 'archive'; body: MutationMeta & { expected_version_id: string } }
