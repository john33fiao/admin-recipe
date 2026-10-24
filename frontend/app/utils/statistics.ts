import type { AppliedStatisticsRange, StatisticsCoverage, StatisticsOrigin, StatisticsSale } from '../types/statistics.ts'
import { displayDecimal } from './catalog.ts'

export function currentTimeZone(): string { return Intl.DateTimeFormat().resolvedOptions().timeZone }
export function validTimeZone(zone: string): boolean {
  try { new Intl.DateTimeFormat('ko-KR', { timeZone: zone }).format(); return true } catch { return false }
}
export function localDateText(date = new Date()): string {
  return `${date.getFullYear().toString().padStart(4, '0')}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`
}
function dateParts(text: string): [number, number, number] {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(text)) throw Error('시작 날짜와 마지막 날짜를 입력해 주세요.')
  const [y, m, d] = text.split('-').map(Number) as [number, number, number]
  const check = new Date(0); check.setUTCFullYear(y, m - 1, d); check.setUTCHours(0, 0, 0, 0)
  if (y < 1 || check.getUTCFullYear() !== y || check.getUTCMonth() !== m - 1 || check.getUTCDate() !== d) throw Error('올바른 날짜를 입력해 주세요.')
  return [y, m, d]
}
export function localDateRange(start: string, last: string): AppliedStatisticsRange {
  const [sy, sm, sd] = dateParts(start), [ey, em, ed] = dateParts(last)
  if (start > last) throw Error('마지막 날짜는 시작 날짜보다 앞설 수 없습니다.')
  // Construct both civil boundaries independently: DST days need not be 24 hours.
  const boundary = (year: number, month: number, day: number) => { const value = new Date(0); value.setFullYear(year, month - 1, day); value.setHours(0, 0, 0, 0); return value }
  const from = boundary(sy, sm, sd), until = boundary(ey, em, ed + 1)
  if (until.getFullYear() > 9999) throw Error('기간 끝의 다음날을 네 자리 연도로 표현할 수 있는 날짜를 선택해 주세요.')
  return { from: from.toISOString(), until: until.toISOString(), zone: currentTimeZone() }
}
function instant(text: string): bigint {
  const match = /^(\d{4}-\d{2}-\d{2})T(\d{2}):(\d{2}):(\d{2})(?:\.(\d{1,9}))?(Z|[+-]\d{2}:\d{2})$/.exec(text)
  if (!match) throw Error('URL의 기간에는 시간대가 포함된 시작·끝 시각이 필요합니다.')
  dateParts(match[1]!)
  if (Number(match[2]) > 23 || Number(match[3]) > 59 || Number(match[4]) > 59) throw Error('URL의 시각을 확인해 주세요.')
  const offset = match[6]!
  if (offset !== 'Z' && (Number(offset.slice(1, 3)) > 23 || Number(offset.slice(4)) > 59)) throw Error('URL의 시간대를 확인해 주세요.')
  const ms = Date.parse(`${match[1]}T${match[2]}:${match[3]}:${match[4]}${offset}`)
  if (!Number.isFinite(ms)) throw Error('URL의 시각을 확인해 주세요.')
  return BigInt(ms) * 1_000_000n + BigInt((match[5] ?? '').padEnd(9, '0'))
}
export function rangeFromQuery(query: Record<string, unknown>): AppliedStatisticsRange | null {
  if (query.from === undefined && query.until === undefined && query.zone === undefined) return null
  const zone = query.zone === undefined ? 'UTC' : query.zone
  if (typeof query.from !== 'string' || typeof query.until !== 'string' || typeof zone !== 'string' || !validTimeZone(zone)) throw Error('URL의 기간과 표시 시간대를 확인해 주세요. 날짜를 골라 다시 조회할 수 있습니다.')
  if (instant(query.from) >= instant(query.until)) throw Error('URL의 기간 끝은 시작보다 뒤여야 합니다.')
  return { from: query.from, until: query.until, zone }
}
export function rangeDateFields(range: AppliedStatisticsRange): { start: string; last: string } {
  const date = (value: Date) => {
    const fields = new Intl.DateTimeFormat('en-CA', { timeZone: range.zone, year: 'numeric', month: '2-digit', day: '2-digit' }).formatToParts(value)
    const field = (type: string) => fields.find(part => part.type === type)!.value
    return `${field('year').padStart(4, '0')}-${field('month')}-${field('day')}`
  }
  const lastNanosecond = instant(range.until) - 1n
  const lastMillisecond = lastNanosecond >= 0n ? lastNanosecond / 1_000_000n : (lastNanosecond - 999_999n) / 1_000_000n
  return { start: date(new Date(range.from)), last: date(new Date(Number(lastMillisecond))) }
}
export function statisticsPath(range: AppliedStatisticsRange): string {
  return `/api/statistics?${new URLSearchParams({ from: range.from, until: range.until })}`
}
export const operationLabels: Record<StatisticsOrigin['kind'], string> = { receipt: '입고', production: '생산', sale_count: '판매', waste: '폐기 · 직원식', stocktake: '실사', purchase_return: '구입 반품' }
export function originPath(origin: Pick<StatisticsOrigin, 'kind' | 'id'>): string {
  const paths = { receipt: 'receipts', production: 'productions', sale_count: 'sales', waste: 'wastes', stocktake: 'stocktakes', purchase_return: 'purchase-returns' }
  return `/${paths[origin.kind]}/${encodeURIComponent(origin.id)}`
}
export function coverageMessage(value: StatisticsCoverage, noun: string): string {
  return value.all_time_records === 0 ? `아직 확정한 ${noun} 기록이 없습니다.` : value.matching_records === 0 ? `이 기간에 ${noun} 기록이 없습니다. 다른 기간에 ${value.all_time_records}건이 있습니다.` : `이 기간의 원기록 ${value.matching_records}건`
}
export type SalesMetric = 'count' | 'amount'
function decimalMicros(value: string): bigint {
  const [whole, fraction = ''] = value.split('.')
  return BigInt(whole!) * 1_000_000n + BigInt(fraction.padEnd(6, '0'))
}
export function saleChartRows(rows: StatisticsSale[], metric: SalesMetric) {
  const points = rows.map(row => {
    const raw = row.status === 'void' ? null : row.value?.[metric] ?? null
    return { id: row.origin.id, time: row.origin.recorded_at, name: row.original.material_name, raw, label: row.status === 'void' ? '취소됨' : raw === null ? '미계산' : `${displayDecimal(raw)}${metric === 'count' ? '개' : '원'}` }
  })
  const values = points.flatMap(point => point.raw === null ? [] : [decimalMicros(point.raw)])
  const max = values.reduce((a, b) => a > b ? a : b, 0n)
  const times = points.map(point => Date.parse(point.time)), minTime = Math.min(...times), maxTime = Math.max(...times)
  return points.map((point, index) => ({ ...point, x: maxTime === minTime ? 500 : 50 + (times[index]! - minTime) / (maxTime - minTime) * 900, y: point.raw === null ? null : 180 - (max === 0n ? 0 : Number(decimalMicros(point.raw) * 140_000n / max) / 1000) }))
}
