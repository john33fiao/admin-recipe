import test from 'node:test'
import assert from 'node:assert/strict'
import { apiRequest } from '../app/utils/api.ts'

test('photo multipart preserves original bytes and lets the browser supply its boundary', async () => {
  const original = globalThis.fetch
  const file = new File([Uint8Array.from([137, 80, 78, 71])], 'virtual.png', { type: 'image/png' })
  const body = new FormData()
  body.append('metadata', JSON.stringify({ photos: [{ source: 'upload', caption: '가상 사진' }] }))
  body.append('photo', file)
  let received: RequestInit | undefined
  globalThis.fetch = async (_url, options) => { received = options; return new Response('{}', { status: 201 }) }
  try {
    await apiRequest('/api/materials/virtual/profiles', body)
    assert.equal(received?.body, body)
    assert.equal(received?.headers, undefined)
    assert.equal(received?.credentials, 'same-origin')
    assert.deepEqual(new Uint8Array(await (body.get('photo') as File).arrayBuffer()), Uint8Array.from([137, 80, 78, 71]))
  } finally { globalThis.fetch = original }
})

test('existing JSON mutations keep their original encoding', async () => {
  const original = globalThis.fetch
  let received: RequestInit | undefined
  globalThis.fetch = async (_url, options) => { received = options; return new Response('{}', { status: 201 }) }
  try {
    await apiRequest('/api/materials', { name: '가상 기존 입력', unit: 'g' })
    assert.equal(received?.body, JSON.stringify({ name: '가상 기존 입력', unit: 'g' }))
    assert.deepEqual(received?.headers, { 'Content-Type': 'application/json' })
  } finally { globalThis.fetch = original }
})
