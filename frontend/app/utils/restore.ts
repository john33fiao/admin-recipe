import type { RestoreArea, RestoreFile, RestoreRequest, RestoreSelection } from '../types/restore.ts'
import { catalogState } from './current-state.ts'

export const restoreFileLimit = 64 * 1024 * 1024
const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i
const sha = /^[0-9a-f]{64}$/
export const restoreAreas: Record<RestoreArea, string> = { materials: '품목', recipes: '레시피', business: '업무 기록', orders: '발주', receipts: '입고', productions: '생산', sales: '판매', stocktakes: '확인한 실사 품목', allocations: '소모 배분', wastes: '폐기 · 직원식', purchase_returns: '구입 반품', reconciliations: '입고 포함 보정', corrections: '정정', profiles: '사진 · 구입처', photos: '사진 파일' }
export function validRestoreSelection(value: unknown): value is RestoreSelection {
  if (!value || typeof value !== 'object') return false
  const data = value as RestoreSelection
  return typeof data.expected_sha256 === 'string' && sha.test(data.expected_sha256) && typeof data.target_id === 'string' && (/^journal:[1-9]\d*$/.test(data.target_id) || data.target_id === `import:${data.expected_sha256}`)
}
export function restorePendingRequest(raw: string | null): RestoreRequest | null {
  try {
    const value = JSON.parse(raw ?? 'null')
    if (!validRestoreSelection(value) || !value || !('meta' in value)) return null
    const meta = (value as RestoreRequest).meta
    if (!meta || !uuid.test(meta.request_id) || !uuid.test(meta.epoch) || !catalogState({ epoch: meta.epoch, revision: meta.expected_revision })) return null
    return { target_id: value.target_id, expected_sha256: value.expected_sha256, meta: { request_id: meta.request_id, epoch: meta.epoch, expected_revision: meta.expected_revision } }
  } catch { return null }
}
export function restoreFileDescriptor(raw: string | null): RestoreFile | null {
  try { const value = JSON.parse(raw ?? 'null'); return value && typeof value.name === 'string' && Number.isSafeInteger(value.size) && value.size > 0 && value.size <= restoreFileLimit && typeof value.sha256 === 'string' && sha.test(value.sha256) ? { name: value.name, size: value.size, sha256: value.sha256 } : null } catch { return null }
}
export async function describeRestoreFile(file: File): Promise<RestoreFile> {
  if (!file.size || file.size > restoreFileLimit) throw new Error('비어 있지 않은 64 MiB 이하 JSON 파일 하나를 선택해 주세요.')
  const hash = await crypto.subtle.digest('SHA-256', await file.arrayBuffer())
  return { name: file.name, size: file.size, sha256: [...new Uint8Array(hash)].map(value => value.toString(16).padStart(2, '0')).join('') }
}
export function sameRestoreFile(a: RestoreFile, b: RestoreFile) { return a.size === b.size && a.sha256 === b.sha256 }
export function exactBytes(value: string) { return `${BigInt(value).toLocaleString('ko-KR')} 바이트` }
