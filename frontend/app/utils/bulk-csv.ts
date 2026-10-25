import type { BulkColumn, BulkKind } from '../types/bulk.ts'

export const bulkLimits = { bytes: 1024 * 1024, rows: 200 }
export const bulkLabels: Record<BulkKind, string> = { material: '재료', menu: '메뉴', receipt: '입고' }
export const bulkColumns: Record<BulkKind, BulkColumn[]> = {
  material: [{ key: 'name', label: '이름', required: true }, { key: 'unit', label: '단위', required: true }, { key: 'kind', label: '종류', required: true }, { key: 'description', label: '설명' }],
  menu: [{ key: 'group', label: '메뉴 묶음', required: true }, { key: 'name', label: '이름', required: true }, { key: 'unit', label: '단위', required: true }, { key: 'description', label: '설명' }, { key: 'basis_count', label: '기준 개수', required: true }, { key: 'selling_price', label: '판매가' }, { key: 'material_id', label: '구성 재료 ID' }, { key: 'quantity', label: '구성 수량' }, { key: 'component_unit', label: '구성 단위' }, { key: 'note', label: '구성 메모' }],
  receipt: [{ key: 'material_id', label: '재료 ID', required: true }, { key: 'quantity', label: '수량', required: true }, { key: 'unit', label: '단위', required: true }, { key: 'amount', label: '구입금액', required: true }, { key: 'receipt_date', label: '입고일', required: true }, { key: 'note', label: '메모' }, { key: 'stock_effect', label: '재고 반영', required: true }, { key: 'observation_version_id', label: '실사 버전 ID' }, { key: 'included_quantity', label: '포함 수량' }],
}
function bounded(raw: string) {
  if (new TextEncoder().encode(raw).length > bulkLimits.bytes) throw new Error('파일과 붙여넣기는 1MiB 이내로 준비해 주세요.')
  if (raw.includes('\u0000')) throw new Error('CSV에 사용할 수 없는 문자가 있습니다.')
}
function table(raw: string, delimiter: string): string[][] {
  bounded(raw)
  const result: string[][] = [], row: string[] = []
  let cell = '', quoted = false, closed = false, atStart = true
  const pushCell = () => { row.push(cell); cell = ''; closed = false; atStart = true }
  const pushRow = () => { pushCell(); result.push(row.splice(0)); if (result.length > bulkLimits.rows + 1) throw new Error('한 번에 200행까지 입력할 수 있습니다.') }
  raw = raw.replace(/^\ufeff/, '')
  for (let i = 0; i < raw.length; i++) {
    const c = raw[i]!
    if (quoted) {
      if (c === '"') { if (raw[i + 1] === '"') { cell += '"'; i++ } else { quoted = false; closed = true } }
      else cell += c === '\r' ? raw[i + 1] === '\n' ? (i++, '\n') : '\n' : c
    } else if (c === delimiter) pushCell()
    else if (c === '\n' || c === '\r') { if (c === '\r' && raw[i + 1] === '\n') i++; pushRow() }
    else if (closed) throw new Error('닫는 인용부호 뒤에는 구분자나 줄바꿈만 사용할 수 있습니다.')
    else if (c === '"') { if (!atStart) throw new Error('인용부호는 셀의 처음에만 사용할 수 있습니다.'); quoted = true; atStart = false }
    else { cell += c; atStart = false }
  }
  if (quoted) throw new Error('CSV의 인용부호가 닫히지 않았습니다.')
  if (cell !== '' || row.length || closed) pushRow()
  return result
}
export function parseBulkCsv(kind: BulkKind, raw: string): Record<string, string>[] {
  const rows = table(raw, ',')
  const header = rows.shift()
  const columns = bulkColumns[kind]
  if (!header || header.length !== columns.length || new Set(header).size !== header.length || header.some(label => !columns.some(column => column.label === label))) throw new Error('양식의 열 이름을 확인해 주세요. 빠진 열·알 수 없는 열·같은 이름의 열은 등록할 수 없습니다.')
  if (!rows.length) throw new Error('제목 아래에 등록할 행을 입력해 주세요.')
  return rows.map((row, index) => {
    if (row.length !== header.length) throw new Error(`${index + 2}번째 줄의 열 개수가 양식과 다릅니다.`)
    return Object.fromEntries(columns.map(column => [column.key, row[header.indexOf(column.label)]!]))
  })
}
export function parseBulkPaste(kind: BulkKind, raw: string): Record<string, string>[] {
  const rows = table(raw, '\t')
  if (!rows.length || rows.length > bulkLimits.rows) throw new Error('1~200행을 붙여넣어 주세요.')
  const columns = bulkColumns[kind]
  return rows.map(row => { if (row.length > columns.length) throw new Error('표보다 열이 많습니다. 양식의 열 순서로 붙여넣어 주세요.'); return Object.fromEntries(columns.map((column, i) => [column.key, row[i] ?? ''])) })
}
export function bulkTemplate(kind: BulkKind): string {
  const examples: Record<BulkKind, string[][]> = {
    material: [['가상 재료', 'g', 'raw', '예제입니다. 이름과 단위를 바꾸세요.']],
    menu: [['가상-A', '가상 메뉴', '개', '', '1', '0', '', '', '', '']],
    receipt: [['11111111-1111-4111-8111-111111111111', '100', 'g', '0', '2026-10-20', '가상 예제 · 실제 재료를 선택하세요', 'unreflected', '', '']],
  }
  const escape = (value: string) => /[",\r\n]/.test(value) ? `"${value.replaceAll('"', '""')}"` : value
  return '\ufeff' + [bulkColumns[kind].map(column => column.label), ...examples[kind]].map(row => row.map(escape).join(',')).join('\r\n') + '\r\n'
}
