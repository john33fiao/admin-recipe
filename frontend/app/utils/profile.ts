import type { ProfileBasis, ProfileFile, ProfileInput, ProfilePending, ProfileVersion } from '../types/profile'

const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i
const object = (value: unknown): value is Record<string, unknown> => !!value && typeof value === 'object' && !Array.isArray(value)
const keys = (value: Record<string, unknown>, names: string[]) => Object.keys(value).length === names.length && names.every(name => name in value)
const isId = (value: unknown): value is string => typeof value === 'string' && uuid.test(value)
export function blankProfile(): ProfileInput { return { photos: [], primary_position: null, suppliers: [] } }
export function copyProfile(value: ProfileInput): ProfileInput { return { photos: value.photos.map(photo => ({ ...photo })), primary_position: value.primary_position, suppliers: value.suppliers.map(supplier => ({ ...supplier })) } }
export function sameProfile(a: ProfileInput, b: ProfileInput): boolean { return JSON.stringify(copyProfile(a)) === JSON.stringify(copyProfile(b)) }
export function profileFromVersion(version: ProfileVersion | null): ProfileInput {
  if (!version) return blankProfile()
  return { photos: version.photos.map(photo => ({ source: 'existing', photo_id: photo.id, caption: photo.caption })), primary_position: version.photos.length ? version.photos.findIndex(photo => photo.is_primary) : null, suppliers: version.suppliers.map(({ url, description }) => ({ url, description })) }
}
export function profileShape(value: unknown): value is ProfileInput {
  if (!object(value) || !keys(value, ['photos', 'primary_position', 'suppliers']) || !Array.isArray(value.photos) || !Array.isArray(value.suppliers)) return false
  return value.photos.length <= 20 && value.suppliers.length <= 20 && (value.primary_position === null || Number.isInteger(value.primary_position))
    && value.photos.every(photo => object(photo) && typeof photo.caption === 'string' && (photo.source === 'existing' ? keys(photo, ['source', 'photo_id', 'caption']) && isId(photo.photo_id) : photo.source === 'upload' && keys(photo, ['source', 'caption'])))
    && value.photos.filter(photo => photo.source === 'upload').length <= 1
    && value.suppliers.every(item => object(item) && keys(item, ['url', 'description']) && (item.url === null || typeof item.url === 'string') && typeof item.description === 'string')
}
export function profileBasisShape(value: unknown): value is ProfileBasis {
  return object(value) && keys(value, ['state', 'material_version_id', 'profile_version_id']) && object(value.state) && keys(value.state, ['epoch', 'revision']) && isId(value.state.epoch) && typeof value.state.revision === 'string' && /^\d+$/.test(value.state.revision) && isId(value.material_version_id) && (value.profile_version_id === null || isId(value.profile_version_id))
}
export function profileFileShape(value: unknown): value is ProfileFile {
  return object(value) && keys(value, ['sha256', 'byte_length', 'format']) && typeof value.sha256 === 'string' && /^[0-9a-f]{64}$/.test(value.sha256) && Number.isInteger(value.byte_length) && Number(value.byte_length) > 0 && Number(value.byte_length) <= 10 * 1024 * 1024 && (value.format === 'jpeg' || value.format === 'png')
}
export function restoreProfilePending(raw: string | null, materialId: string): ProfilePending | null {
  try {
    const value: unknown = JSON.parse(raw ?? 'null')
    if (!object(value) || !keys(value, ['material_id', 'body', 'file']) || value.material_id !== materialId || !isId(value.material_id) || !object(value.body)) return null
    const body = value.body
    if (!keys(body, ['meta', 'expected_material_version_id', 'expected_profile_version_id', 'photos', 'primary_position', 'suppliers']) || !object(body.meta) || !keys(body.meta, ['request_id', 'epoch', 'expected_revision'])) return null
    if (!isId(body.meta.request_id) || !profileBasisShape({ state: { epoch: body.meta.epoch, revision: body.meta.expected_revision }, material_version_id: body.expected_material_version_id, profile_version_id: body.expected_profile_version_id })) return null
    if (!profileShape({ photos: body.photos, primary_position: body.primary_position, suppliers: body.suppliers })) return null
    const uploads = (body.photos as ProfileInput['photos']).some(photo => photo.source === 'upload')
    if (uploads ? !profileFileShape(value.file) : value.file !== null) return null
    return value as unknown as ProfilePending
  } catch { return null }
}
export function normalizeProfile(value: ProfileInput): ProfileInput {
  const text = (value: string) => value.replace(/\r\n?/g, '\n').trim()
  return { photos: value.photos.map(photo => ({ ...photo, caption: text(photo.caption) })), primary_position: value.primary_position, suppliers: value.suppliers.map(item => ({ url: item.url?.trim() || null, description: text(item.description) })) }
}
export function safeSupplierUrl(value: string): boolean {
  if (!value || /[\u0000-\u0020\u007f-\u009f]/.test(value) || [...value].length > 2048 || !/^https?:\/\//i.test(value)) return false
  try { const url = new URL(value); return ['https:', 'http:'].includes(url.protocol) && !!url.hostname && !url.username && !url.password } catch { return false }
}
export function validateProfile(value: ProfileInput): Record<string, string> {
  const errors: Record<string, string> = {}
  if (value.photos.length > 20) errors.photos = '사진은 20장까지 연결할 수 있습니다.'
  if (value.photos.length ? value.primary_position === null || value.primary_position < 0 || value.primary_position >= value.photos.length : value.primary_position !== null) errors.photos = '현재 사진 중 대표 사진 한 장을 선택해 주세요.'
  value.photos.forEach((photo, index) => { if ([...photo.caption].length > 500 || /[\u0000-\u0009\u000b-\u001f\u007f-\u009f]/.test(photo.caption)) errors[`photo-${index}`] = '사진 설명은 제어문자 없이 500자 이내로 입력해 주세요.' })
  if (value.suppliers.length > 20) errors.suppliers = '구입처는 20건까지 등록할 수 있습니다.'
  value.suppliers.forEach((item, index) => {
    if (item.url && !safeSupplierUrl(item.url)) errors[`supplier-${index}-url`] = 'http 또는 https 주소를 입력해 주세요. 계정 정보나 제어문자를 포함할 수 없습니다.'
    if (!item.url && !item.description.trim()) errors[`supplier-${index}-description`] = '구입처 링크 또는 설명 중 하나를 입력해 주세요.'
    if ([...item.description].length > 2000 || /[\u0000-\u0009\u000b-\u001f\u007f-\u009f]/.test(item.description)) errors[`supplier-${index}-description`] = '구입처 설명은 제어문자 없이 2,000자 이내로 입력해 주세요.'
  })
  return errors
}
export function removeProfilePhoto(value: ProfileInput, index: number): ProfileInput {
  const result = copyProfile(value)
  result.photos.splice(index, 1)
  if (!result.photos.length) result.primary_position = null
  else if (result.primary_position === index || result.primary_position === null) result.primary_position = 0
  else if (result.primary_position > index) result.primary_position--
  return result
}
export function sameProfileFile(a: ProfileFile, b: ProfileFile): boolean { return a.sha256 === b.sha256 && a.byte_length === b.byte_length && a.format === b.format }
export async function describeProfileFile(file: File): Promise<ProfileFile> {
  if (!file.size || file.size > 10 * 1024 * 1024) throw new Error('사진은 비어 있지 않은 10MiB 이하 파일을 선택해 주세요.')
  const buffer = await file.arrayBuffer(), bytes = new Uint8Array(buffer)
  const format = bytes[0] === 137 && bytes[1] === 80 && bytes[2] === 78 && bytes[3] === 71 && bytes[4] === 13 && bytes[5] === 10 && bytes[6] === 26 && bytes[7] === 10 ? 'png' : bytes[0] === 255 && bytes[1] === 216 && bytes[2] === 255 ? 'jpeg' : null
  if (!format) throw new Error('JPEG 또는 PNG 사진을 선택해 주세요. 다른 형식은 지원하지 않습니다.')
  const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', buffer))
  return { sha256: [...digest].map(value => value.toString(16).padStart(2, '0')).join(''), byte_length: file.size, format }
}
