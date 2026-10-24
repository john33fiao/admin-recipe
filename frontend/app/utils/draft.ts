import type { CatalogInput, MaterialInput, MaterialKind } from '../types/material.ts'

// A tab draft may be incomplete and is never an immutable submission request.
export function restoreMaterialDraft(raw: string | null): MaterialInput | null {
  if (!raw) return null
  try {
    const value: unknown = JSON.parse(raw)
    if (!value || typeof value !== 'object') return null
    const data = value as Record<string, unknown>
    if (typeof data.name !== 'string' || typeof data.unit !== 'string') return null
    return { name: data.name, unit: data.unit }
  } catch {
    return null
  }
}

export function restoreCatalogDraft(raw: string | null): CatalogInput | null {
  const basic = restoreMaterialDraft(raw)
  if (!basic || !raw) return null
  try {
    const data = JSON.parse(raw) as Record<string, unknown>
    const kind = typeof data.kind === 'string' && ['raw', 'prepared', 'stocked', 'menu', 'unclassified'].includes(data.kind) ? data.kind as MaterialKind : 'raw'
    return { ...basic, kind, description: typeof data.description === 'string' ? data.description : '' }
  } catch { return null }
}
