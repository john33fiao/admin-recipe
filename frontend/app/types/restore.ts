import type { CatalogState, MutationMeta } from './material'

export type RestoreSource = 'journal' | 'import'
export interface RestoreTarget { target_id: string; source: RestoreSource; source_kind: 'journal' | 'before_restore'; sha256: string; byte_length: string; format_version: number; schema_version: number | null; sequence: string; request_id: string | null; recorded_at: string }
export interface RestoreTargetPage { state: CatalogState; source: RestoreSource; rows: RestoreTarget[]; next_cursor: string | null }
export interface RestoreSelection { target_id: string; expected_sha256: string }
export interface RestoreRequest extends RestoreSelection { meta: MutationMeta }
export type RestoreArea = 'materials' | 'recipes' | 'business' | 'orders' | 'receipts' | 'productions' | 'sales' | 'stocktakes' | 'allocations' | 'wastes' | 'purchase_returns' | 'reconciliations' | 'corrections' | 'profiles' | 'photos'
export interface RestoreSummary { areas: Array<{ area: RestoreArea; records: number; versions: number }>; drafts: number; photo_bytes: string }
export interface RestorePreview { target: RestoreTarget; state: CatalogState; before: RestoreSummary; after: RestoreSummary; photos: { required: number; verified: number; missing: number; corrupt: number; unsafe_path: number; examples: Array<{ sha256: string; problem: 'missing' | 'corrupt' | 'unsafe_path' }>; examples_truncated: boolean }; blockers: Array<{ code: 'missing_assets' | 'invalid_assets' | 'request_conflict' | 'sequence_exhausted'; count: number; message: string }>; can_restore: boolean }
export interface RestoreResult { restore_id: string; target: RestoreTarget; previous_target: RestoreTarget; recorded_at: string; state: CatalogState }
export interface RestoreFile { name: string; size: number; sha256: string }
