import type { MaterialInput } from '../types/material.ts'

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
