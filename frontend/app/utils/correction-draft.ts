import type { CorrectionSeed } from '../types/correction.ts'
import { restoreBusinessPending, validBusinessPayload } from './business.ts'

// A prior sent request/local edit retains its target version until explicitly
// reviewed. A later completed correction receives a fresh editor identity.
export function correctionDraftIdentity(seed: CorrectionSeed, entries: Array<[string, string]>) {
  const target = seed.original.reference.original_confirmation_id
  const base = `new.correction.${target}.`
  for (const suffix of ['.pending.v1', '.draft.v1']) for (const [key, raw] of entries) {
    if (!key.startsWith(`sunchang.business.${base}`) || !key.endsWith(suffix)) continue
    try {
      const pending = suffix === '.pending.v1' ? restoreBusinessPending(raw) : null
      if (pending?.action === 'create' && pending.body.corrects_confirmation_id === target && pending.body.payload.kind === 'correction') return key.slice('sunchang.business.'.length, -suffix.length)
      const stored = suffix === '.draft.v1' ? JSON.parse(raw)?.payload : null
      if (validBusinessPayload(stored) && stored.kind === 'correction') return key.slice('sunchang.business.'.length, -suffix.length)
    } catch { /* Invalid local values cannot choose another target. */ }
  }
  return `${base}${seed.current.reference.version_id}`
}
