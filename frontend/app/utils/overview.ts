import type { CatalogState } from '../types/material.ts'
import type { OperationsOverview, OverviewList, OverviewListResponse, OverviewPage, OverviewScope, OverviewSection, OverviewTarget } from '../types/overview.ts'

export const overviewLabels: Record<OverviewSection, string> = { materials: '마지막 확인 수량', material_changes: '확인 이후 변동', open_orders: '미입고 주문', received_orders: '입고 완료와 주문 차이', receipts: '최근 입고', sales: '최근 판매', observations: '최근 수량 확인', unallocated: '미배분 소모', unconfirmed: '소모 미확정', drafts: 'DB 임시 저장' }
export const overviewDescriptions: Record<OverviewSection, string> = {
  materials: '전체 품목(보관 포함)입니다. 현재 사용 품목 먼저, 이름순으로 표시합니다. 확인 수량에 변동을 더해 현재고를 계산하지 않습니다.',
  material_changes: '마지막 활성 수량 확인 뒤의 현재 유효한 변동입니다. 정정된 기록도 원래 발생한 순서에 둡니다.',
  open_orders: '아직 입고로 마치지 않은 주문입니다. 주문량은 실제 수량에 더하지 않습니다.',
  received_orders: '현재 유효한 입고로 마친 주문입니다. 입고량과 주문량의 차이를 함께 봅니다.',
  receipts: '현재 유효한 입고를 원확정 순서로 봅니다. 사용자가 입력한 입고일은 확정 시각과 구별합니다.',
  sales: '현재 유효한 판매를 원확정 순서로 봅니다. 0개도 포함하며 금액 미계산과 0원을 구별합니다.',
  observations: '현재 활성 수량 확인 행입니다. 한 실사에 여러 품목이 있으면 각각 한 건으로 셉니다.',
  unallocated: '판매 없음 또는 총 판매 개수 0으로 배분하지 못한 소모입니다. 초기 확인은 소모 0으로 세지 않습니다.',
  unconfirmed: '판매와 당시 직접 구성 품목의 조합 중 소모 근거가 연결되지 않은 건수입니다. 명시적으로 배분한 0과 다릅니다.',
  drafts: 'DB에 임시 저장한 업무입니다. 입력 내용의 확정 가능 여부는 아직 평가하지 않았습니다. 브라우저 미저장 입력·응답 미확인 요청은 모아 입력에서 확인하세요.',
}
export const overviewSections = Object.keys(overviewLabels).filter(key => key !== 'material_changes') as Array<keyof OperationsOverview & Exclude<OverviewSection, 'material_changes'>>
const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/
export function overviewTarget(query: Record<string, unknown>): OverviewTarget {
  const { section, material_id: material, offset = '0' } = query
  if (Object.keys(query).some(key => !['section', 'material_id', 'offset'].includes(key)) || typeof section !== 'string' || !Object.hasOwn(overviewLabels, section) || typeof offset !== 'string' || !/^(0|[1-9][0-9]*)$/.test(offset)) throw Error('목록 주소가 올바르지 않습니다. 작업 시작에서 목록을 다시 선택하세요.')
  if (!Number.isSafeInteger(Number(offset))) throw Error('목록 위치가 이 브라우저에서 정확히 표시할 수 있는 범위를 넘었습니다. 작업 시작에서 목록을 다시 선택하세요.')
  if (section === 'material_changes' ? typeof material !== 'string' || !uuid.test(material) : material !== undefined) throw Error('품목별 변동 목록의 주소를 다시 확인해 주세요.')
  return { section: section as OverviewSection, material_id: typeof material === 'string' ? material : null, offset }
}
export function overviewListUrl(scope: OverviewScope, offset: number | string = 0): string {
  const query = new URLSearchParams({ section: scope.section, offset: String(offset) })
  if (scope.material_id !== null) query.set('material_id', scope.material_id)
  return `/overview/list?${query}`
}
export function overviewListApi(target: OverviewTarget, state: CatalogState): string {
  const query = new URLSearchParams({ section: target.section, offset: target.offset, epoch: state.epoch, revision: state.revision })
  if (target.material_id !== null) query.set('material_id', target.material_id)
  return `/api/overview/list?${query}`
}
export function sameOverviewState(a: CatalogState, b: CatalogState): boolean { return a.epoch === b.epoch && a.revision === b.revision }
export function sameOverviewScope(a: OverviewScope, b: OverviewScope): boolean { return a.section === b.section && a.material_id === b.material_id }
export function overviewResponseMatches(value: OverviewListResponse, target: OverviewTarget, state: CatalogState): boolean {
  const page = value.list.page
  return [page.total_count, page.returned_count, page.offset, page.page_size, ...(page.next_offset === null ? [] : [page.next_offset])].every(value => Number.isSafeInteger(value) && value >= 0)
    && sameOverviewState(value.state, state) && sameOverviewScope(value.scope, target) && value.list.section === target.section
    && (value.list.section !== 'material_changes' || value.list.material.id === target.material_id)
    && String(page.offset) === target.offset && page.returned_count === page.rows.length
    && page.page_size === (target.section === 'materials' ? 25 : 20)
}
export function overviewListFrom(report: OperationsOverview, section: Exclude<OverviewSection, 'material_changes'>): OverviewList {
  return { section, page: report[section] } as OverviewList
}
export function overviewPageText(page: OverviewPage<unknown>): string {
  return page.returned_count === 0 ? `전체 ${page.total_count.toLocaleString('ko-KR')}건 · 이 페이지에 표시할 기록이 없습니다.` : `전체 ${page.total_count.toLocaleString('ko-KR')}건 중 ${(page.offset + 1).toLocaleString('ko-KR')}–${(page.offset + page.returned_count).toLocaleString('ko-KR')}건`
}
export function noAllocationText(reason: 'no_sales' | 'zero_weight' | null): string { return reason === 'no_sales' ? '판매 없음' : reason === 'zero_weight' ? '총 판매 개수 0' : '판매별 배분 근거 있음' }
