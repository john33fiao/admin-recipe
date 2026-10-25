import test from 'node:test'
import assert from 'node:assert/strict'
import { parseBulkCsv, bulkTemplate, parseBulkPaste } from '../app/utils/bulk-csv.ts'

// A split(',') implementation corrupts the quoted description and comma in the name.
test('bulk CSV keeps quoted comma, escaped quote, newline, BOM and explicit zero', () => {
  const rows = parseBulkCsv('material', '\ufeff이름,단위,종류,설명\r\n"가상, 재료",g,raw,"첫 줄\n""둘째"" 줄"\r\n')
  assert.deepEqual(rows, [{ name: '가상, 재료', unit: 'g', kind: 'raw', description: '첫 줄\n"둘째" 줄' }])
  const receipts = parseBulkCsv('receipt', '재료 ID,수량,단위,구입금액,입고일,메모,재고 반영,실사 버전 ID,포함 수량\na,1,g,0,2026-10-20,,unreflected,,')
  assert.equal(receipts[0]!.amount, '0')
  assert.equal(receipts[0]!.note, '')
})
test('malformed, unknown or duplicate columns and oversized input never become editable rows', () => {
  for (const raw of ['이름,단위,종류,설명\n"열린,g,raw,', '이름,단위,종류,설명\na"b,g,raw,', '이름,단위,종류,설명\n"a"x,g,raw,', '이름,이름,종류,설명\na,g,raw,', '이름,단위,종류,비밀\na,g,raw,', '이름,단위,종류,설명\na,g,raw,,']) assert.throws(() => parseBulkCsv('material', raw))
  assert.throws(() => parseBulkCsv('material', `이름,단위,종류,설명\n${Array.from({ length: 201 }, () => 'a,g,raw,').join('\n')}`))
  assert.throws(() => parseBulkCsv('material', `이름,단위,종류,설명\n${'가'.repeat(400_000)},g,raw,`))
})
test('downloadable examples roundtrip and tab paste preserves blank cells', () => {
  for (const kind of ['material', 'menu', 'receipt'] as const) assert.ok(parseBulkCsv(kind, bulkTemplate(kind)).length)
  assert.deepEqual(parseBulkPaste('material', '가상\tg\traw\t\n다른 가상\tkg\tprepared\t설명'), [{ name: '가상', unit: 'g', kind: 'raw', description: '' }, { name: '다른 가상', unit: 'kg', kind: 'prepared', description: '설명' }])
})
