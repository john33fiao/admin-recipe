import test from 'node:test'
import assert from 'node:assert/strict'
import { formatCreatedAt } from '../app/utils/material.ts'
import { localDateRange, originPath, rangeDateFields, rangeFromQuery, saleChartRows, statisticsPath, coverageMessage } from '../app/utils/statistics.ts'
import type { StatisticsSale } from '../app/types/statistics.ts'

test('a report timestamp keeps its applied display zone when the PC zone changes', () => {
  const previous = process.env.TZ
  const timestamp = '2026-01-01T00:30:00Z'
  const expected = new Intl.DateTimeFormat('ko-KR', { dateStyle: 'medium', timeStyle: 'short', timeZone: 'UTC' }).format(new Date(timestamp))
  try {
    process.env.TZ = 'Asia/Seoul'
    const display = formatCreatedAt as (value: string, zone: string) => string
    assert.equal(display(timestamp, 'UTC'), expected)
    process.env.TZ = 'America/New_York'
    assert.equal(display(timestamp, 'UTC'), expected)
  } finally { if (previous === undefined) delete process.env.TZ; else process.env.TZ = previous }
})

test('civil day boundaries include the last date and independently respect 23 and 25 hour DST days', () => {
  const previous = process.env.TZ
  try {
    process.env.TZ = 'America/New_York'
    const spring = localDateRange('2026-03-08', '2026-03-08'), autumn = localDateRange('2026-11-01', '2026-11-01')
    assert.equal(spring.from, '2026-03-08T05:00:00.000Z'); assert.equal(spring.until, '2026-03-09T04:00:00.000Z')
    assert.equal(Date.parse(autumn.until) - Date.parse(autumn.from), 25 * 60 * 60 * 1000)
    assert.throws(() => localDateRange('2026-02-30', '2026-03-01'))
    assert.throws(() => localDateRange('2026-02-02', '2026-02-01'))
  } finally { if (previous === undefined) delete process.env.TZ; else process.env.TZ = previous }
})

test('supported early civil years keep their actual year without the Date constructor 1900 adjustment', () => {
  const previous = process.env.TZ
  try {
    process.env.TZ = 'UTC'
    assert.equal(localDateRange('0001-01-01', '0001-01-01').from, '0001-01-01T00:00:00.000Z')
    assert.deepEqual(rangeDateFields(localDateRange('0001-01-01', '0001-01-01')), { start: '0001-01-01', last: '0001-01-01' })
    const last = localDateRange('0099-12-31', '0099-12-31')
    assert.equal(last.from, '0099-12-31T00:00:00.000Z'); assert.equal(last.until, '0100-01-01T00:00:00.000Z')
  } finally { if (previous === undefined) delete process.env.TZ; else process.env.TZ = previous }
})

test('absolute URL range survives a changed PC zone and never sends display metadata to the API', () => {
  const previous = process.env.TZ
  const saved = { from: '2026-03-08T00:00:00-05:00', until: '2026-03-09T00:00:00-04:00', zone: 'America/New_York' }
  try {
    process.env.TZ = 'Asia/Seoul'
    assert.deepEqual(rangeFromQuery(saved), saved)
    assert.deepEqual(rangeFromQuery({ from: saved.from, until: saved.until }), { ...saved, zone: 'UTC' })
    assert.deepEqual(rangeDateFields(saved), { start: '2026-03-08', last: '2026-03-08' })
    const params = new URL(statisticsPath(saved), 'http://localhost').searchParams
    assert.deepEqual([...params.keys()], ['from', 'until']); assert.equal(params.get('from'), saved.from)
    assert.throws(() => rangeFromQuery({ ...saved, from: [saved.from, saved.from] }))
    assert.throws(() => rangeFromQuery({ ...saved, zone: 'not/a-zone' }))
    assert.throws(() => rangeFromQuery({ ...saved, from: '2026-03-08T00:00:00' }))
    assert.throws(() => rangeFromQuery({ ...saved, until: saved.from }))
  } finally { if (previous === undefined) delete process.env.TZ; else process.env.TZ = previous }
})

test('the exclusive end keeps a date included by even one nanosecond after its midnight', () => {
  assert.deepEqual(rangeDateFields({ from: '2026-01-01T00:00:00Z', until: '2026-01-02T00:00:00.000000001Z', zone: 'UTC' }), { start: '2026-01-01', last: '2026-01-02' })
  assert.deepEqual(rangeDateFields({ from: '2026-01-01T00:00:00Z', until: '2026-01-02T00:00:00Z', zone: 'UTC' }), { start: '2026-01-01', last: '2026-01-01' })
})

test('chart and table share exact i128-range strings while missing, zero and void remain distinct', () => {
  const row = (id: string, count: string, amount: string | null, status: 'active' | 'void' = 'active') => ({ origin: { id, recorded_at: '2026-01-01T00:00:00Z' }, original: { material_name: id }, status, value: status === 'void' ? null : { count, amount } }) as StatisticsSale
  const source = [row('large', '9007199254740993', '170141183460469231731687303715884.105727'), row('zero', '0', '0'), row('missing', '0', null), row('void', '0', '0', 'void')]
  const chart = saleChartRows(source, 'amount')
  assert.equal(chart[0]!.raw, source[0]!.value!.amount); assert.equal(chart[0]!.label, '170,141,183,460,469,231,731,687,303,715,884.105727원')
  assert.equal(chart[0]!.y, 40); assert.equal(chart[1]!.y, 180); assert.equal(chart[1]!.label, '0원')
  assert.equal(chart[2]!.y, null); assert.equal(chart[2]!.label, '미계산'); assert.equal(chart[3]!.y, null); assert.equal(chart[3]!.label, '취소됨')
  assert.equal(saleChartRows(source, 'count')[0]!.label, '9,007,199,254,740,993개')
  assert.deepEqual(saleChartRows([], 'amount'), [])
})

test('typed origin links use stable owners and empty coverage does not fabricate zero consumption', () => {
  assert.equal(originPath({ kind: 'stocktake', id: 'batch-id' }), '/stocktakes/batch-id')
  assert.equal(originPath({ kind: 'production', id: 'production-id' }), '/productions/production-id')
  assert.equal(originPath({ kind: 'sale_count', id: 'sale-id' }), '/sales/sale-id')
  assert.equal(coverageMessage({ matching_records: 0, all_time_records: 3 }, '판매'), '이 기간에 판매 기록이 없습니다. 다른 기간에 3건이 있습니다.')
  assert.equal(coverageMessage({ matching_records: 0, all_time_records: 0 }, '확인 수량'), '아직 확정한 확인 수량 기록이 없습니다.')
})
