import test from 'node:test'
import assert from 'node:assert/strict'
import { blankProfile, copyProfile, describeProfileFile, normalizeProfile, profileFromVersion, profileShape, removeProfilePhoto, restoreProfilePending, safeSupplierUrl, sameProfile, sameProfileFile, validateProfile } from '../app/utils/profile.ts'
const id = '11111111-1111-4111-8111-111111111111'
const png = Uint8Array.from([137, 80, 78, 71, 13, 10, 26, 10, 1])
function input() { return { photos: [{ source: 'existing' as const, photo_id: id, caption: '가상 사진' }, { source: 'upload' as const, caption: '새 사진' }], primary_position: 1, suppliers: [{ url: null, description: '가상 설명' }] } }
test('profile nested copy and primary removal preserve the original and distinguish an empty profile', () => {
  const original = input(), copy = copyProfile(original)
  copy.photos[0]!.caption = '다른 설명'; copy.suppliers[0]!.description = '다른 구입처'
  assert.equal(original.photos[0]!.caption, '가상 사진'); assert.equal(original.suppliers[0]!.description, '가상 설명')
  assert.equal(sameProfile(original, copy), false)
  const removed = removeProfilePhoto(original, 1)
  assert.equal(removed.primary_position, 0)
  assert.deepEqual(removeProfilePhoto(removed, 0).photos, [])
  assert.equal(removeProfilePhoto(removed, 0).primary_position, null)
  assert.deepEqual(profileFromVersion(null), blankProfile())
  assert.deepEqual(validateProfile(blankProfile()), {})
})
test('profile request recovery retains exact metadata and file identity and rejects unrelated or extra fields', async () => {
  const file = await describeProfileFile(new File([png], 'virtual.png', { type: 'image/png' }))
  const value = { material_id: id, body: { ...input(), meta: { request_id: id, epoch: id, expected_revision: '12' }, expected_material_version_id: id, expected_profile_version_id: null }, file }
  assert.deepEqual(restoreProfilePending(JSON.stringify(value), id), value)
  assert.equal(restoreProfilePending(JSON.stringify(value), '22222222-2222-4222-8222-222222222222'), null)
  assert.equal(restoreProfilePending(JSON.stringify({ ...value, file: null }), id), null)
  assert.equal(restoreProfilePending(JSON.stringify({ ...value, body: { ...value.body, guess: true } }), id), null)
  assert.equal(profileShape({ ...input(), photos: [{ source: 'upload', caption: '', blobUrl: 'blob:unknown' }] }), false)
})
test('identical bytes can be reselected under another name and different bytes cannot replace an uncertain file', async () => {
  const first = await describeProfileFile(new File([png], 'first.png'))
  const same = await describeProfileFile(new File([png], 'other.png'))
  const different = await describeProfileFile(new File([Uint8Array.from([...png, 2])], 'first.png'))
  assert.equal(sameProfileFile(first, same), true)
  assert.equal(sameProfileFile(first, different), false)
  await assert.rejects(describeProfileFile(new File(['<svg/>'], 'fake.png')), /JPEG 또는 PNG/)
  await assert.rejects(describeProfileFile(new File([], 'empty.png')), /10MiB/)
})
test('profile allows link-only or description-only suppliers and keeps hostile content as text', () => {
  const value = normalizeProfile({ ...blankProfile(), suppliers: [{ url: ' https://example.invalid/virtual ', description: '' }, { url: null, description: '<script>가상 설명</script>\r\n추가' }] })
  assert.deepEqual(validateProfile(value), {})
  assert.equal(value.suppliers[1]!.description, '<script>가상 설명</script>\n추가')
  assert.ok(validateProfile({ ...blankProfile(), suppliers: [{ url: null, description: '' }] })['supplier-0-description'])
  for (const url of ['javascript:alert(1)', '//example.invalid', 'file:///test', 'https://a:b@example.invalid', 'https://example.invalid/\npath']) assert.equal(safeSupplierUrl(url), false)
  assert.ok(validateProfile({ ...blankProfile(), photos: [{ source: 'upload', caption: '🍚'.repeat(501) }], primary_position: 0 })['photo-0'])
  assert.ok(validateProfile({ ...blankProfile(), photos: [{ source: 'upload', caption: '사진\t설명' }], primary_position: 0 })['photo-0'])
  assert.ok(validateProfile({ ...blankProfile(), suppliers: [{ url: null, description: '구입처\t설명' }] })['supplier-0-description'])
})
