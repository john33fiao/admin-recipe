import type { CatalogState, Material, MutationMeta } from './material'

export interface PhotoAsset { sha256: string; asset_key: string; format: 'jpeg' | 'png'; byte_length: number; width: number; height: number }
export interface ProfilePhoto { id: string; position: number; asset: PhotoAsset; caption: string; is_primary: boolean }
export interface ProfileSupplier { url: string | null; description: string }
export interface ProfileVersion {
  id: string; material_id: string; material_version_id: string; previous_version_id: string | null; copied_from_version_id: string | null
  photos: ProfilePhoto[]; suppliers: (ProfileSupplier & { position: number })[]; recorded_at: string; seq: string
}
export interface ProfileDetail { material: Material; version: ProfileVersion | null; state: CatalogState }
export interface ProfileHistory {
  material_id: string; state: CatalogState
  versions: { id: string; material_version_id: string; previous_version_id: string | null; copied_from_version_id: string | null; photo_count: number; supplier_count: number; primary_photo_id: string | null; recorded_at: string; seq: string }[]
}
export type ProfilePhotoInput = { source: 'existing'; photo_id: string; caption: string } | { source: 'upload'; caption: string }
export interface ProfileInput { photos: ProfilePhotoInput[]; primary_position: number | null; suppliers: ProfileSupplier[] }
export interface ProfileRequest extends ProfileInput { meta: MutationMeta; expected_material_version_id: string; expected_profile_version_id: string | null }
export interface ProfileFile { sha256: string; byte_length: number; format: 'jpeg' | 'png' }
export interface ProfilePending { material_id: string; body: ProfileRequest; file: ProfileFile | null }
export interface ProfileBasis { state: CatalogState; material_version_id: string; profile_version_id: string | null }
