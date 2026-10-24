# 할 일

## 현재 단계

2026-10-24에 기존 요구사항과 후속 사용자 결정을 T-001~T-030으로 연결했다. [티켓 색인](tickets/README.md)이 상세 상태와 의존 관계의 정본이며 [요구사항](REQUIREMENTS.md)·[ADR-007](decisions/ADR-007-consumption-and-ticket-scope.md)을 기준으로 구현한다.

T-001의 로컬 실행·재료 이름/단위 저장·조회는 구현했다. 별도 깨끗한 Windows/Mac·실제 절전 등 필수 검증은 미완료다. OS 간 데이터 왕복은 백로그이며 같은 OS 이동 등 다른 기준은 유지한다. 신규 업무 티켓은 미구현이다.

## 확정한 운영 기준

- 판매 입력만으로 재고를 차감하지 않고 실사에서 소모를 확정한다. 별도의 예상 현재고를 관리하지 않는다.
- 명시 폐기·생산 투입·반품을 제외한 소모를 해당 실사 구간의 메뉴 판매 개수 비율로 배분한다. 판매가 없으면 미배분으로 남긴다.
- 레시피 수량 0·0원 재료를 허용한다. 입고 전 가격 등록과 보유량 0일 때 마지막 유효 단가를 지원한다.
- 레시피 수정은 이후 판매부터 적용하고 과거 버전을 보존한다. 삭제 대신 아카이빙한다.
- 여러 생산 결과물은 주 결과물 원가 100%·나머지 0%를 기본으로 수정 가능하다.
- 반품은 현재 평균 단가로 보유 가치를 차감하고 원구입 금액은 참고로 보존한다.
- 판매가·단순 원가율·판매금액 합계, 여러 사진/대표 사진과 여러 구입처를 포함한다.
- 발주는 내역 저장이며 직접 입고를 허용한다. 1회 실입고로 주문을 종료하고 차이를 남긴다.
- 지연 입고의 이미 실사에 포함된 수량을 확인해 중복을 막는다. 저장·전체 복원·호환·단일 PC 정책은 유지한다.

## 1차 실행 순서

아래 순서는 선행 관계를 만족하는 한 가지 실행안이다. T-001의 잔여 실환경 검증은 환경이 준비되면 진행하며 T-021의 최종 완료 판단에 반영한다.

- [ ] 1. [T-002: 수량·원가와 소모 보정 계산 계약](tickets/T-002-domain-contract.md)
- [ ] 2. [T-020: 공통 탐색·입력 상태·라이트/다크와 반응형 UI](tickets/T-020-ui-foundation.md)
- [ ] 3. [T-003: 재료·상태·단위와 사전 가격 관리](tickets/T-003-materials-and-units.md)
- [ ] 4. [T-005: 업무 기록·임시 저장·확정과 JSON 기록 확장](tickets/T-005-business-history.md)
- [ ] 5. [T-006: 발주 내역과 미입고 주문량](tickets/T-006-purchase-orders.md)
- [ ] 6. [T-007: 실측 입고와 보유 평균 원가](tickets/T-007-receipts.md)
- [ ] 7. [T-004: 레시피·메뉴 복제와 사전 원가 계산](tickets/T-004-recipes-and-costs.md)
- [ ] 8. [T-008: 실측 손질·비축 생산과 원가 이월](tickets/T-008-production.md)
- [ ] 9. [T-009: 판매 수량·금액과 실사 소모 연결](tickets/T-009-sales-consumption.md)
- [ ] 10. [T-010: 명시 폐기·선택 실사와 판매 소모 배분](tickets/T-010-stocktake-and-waste.md)
- [ ] 11. [T-011: 실사에 포함된 지연 입고 보정](tickets/T-011-late-receipt-reconciliation.md)
- [ ] 12. [T-012: 업무 오입력 정정과 버전·아카이브 이력](tickets/T-012-corrections-and-versions.md)
- [ ] 13. [T-013: 구매 취소·반품과 현재 재고 가치](tickets/T-013-purchase-returns.md)
- [ ] 14. [T-014: 판매·생산·폐기·실사를 모아 입력하는 작업 화면](tickets/T-014-closing-workspace.md)
- [ ] 15. [T-015: 사진·대표 사진·설명과 구입처](tickets/T-015-photos-and-suppliers.md)
- [ ] 16. [T-016: 판매 통계·실사 소모·원가 이력 조회](tickets/T-016-statistics-and-history.md)
- [ ] 17. [T-017: 발주·입고·마지막 실사 중심 운영 개요](tickets/T-017-operations-overview.md)
- [ ] 18. [T-018: 사진과 버전 이력을 포함한 전체 상태 복원](tickets/T-018-full-state-restore.md)
- [ ] 19. [T-019: 이전 DB·JSON 호환과 같은 OS 데이터 이동](tickets/T-019-data-compatibility.md)
- [ ] 20. [T-021: 1차 업무 통합·복구·배포 준비 검증](tickets/T-021-first-release-verification.md)

## 기존 실행 검증

- [ ] [T-001](tickets/T-001-portable-local-app.md): 별도 장비·실제 절전 등 미검증 기준 완료. 기존 통과 결과는 [검증 기록](tickets/T-001-verification.md)에 보존한다.

## 2차

- [ ] [T-022: 2차 내일 메뉴·공유 재료 계획 요구 구체화](tickets/T-022-next-day-menu-plan.md)
- [ ] [T-023: 2차 실제 작업시간 기록과 시계열](tickets/T-023-work-time-history.md)

## 백로그

- [ ] [T-024: 시설 점검 카테고리·항목·일일 기록](tickets/T-024-facility-checks.md)
- [ ] [T-025: 부분 입고와 미입고 잔량 추적](tickets/T-025-partial-receipts.md)
- [ ] [T-026: 보관 위치별 재고·이동 요구 구체화](tickets/T-026-storage-locations.md)
- [ ] [T-027: Windows↔macOS 데이터 왕복 검증](tickets/T-027-cross-os-data-transfer.md)
- [ ] [T-030: 소모를 연결할 판매 직접 선택](tickets/T-030-manual-consumption-allocation.md)

## 안정화 이후 검토

- [ ] [T-028: 안정화 후 서버 이관 계획](tickets/T-028-server-migration.md)
- [ ] [T-029: 포장재·소모품·인건비 등 원가 확장 검토](tickets/T-029-extended-costs.md)

## 현재 범위에서 제외

소비기한 추적·초기 자료 이관·네트워크 공동 사용·비밀번호 잠금·POS·고객 환불·발주서 출력/외부 전송·세금 자동 분리·할인·결제 관리는 제외한다. 후속 아이디어와 과거 화면 가정을 신규 구현 요구로 취급하지 않는다.

## 검증과 관리

각 티켓의 수용 기준과 [공통 완료 조건](tickets/README.md#공통-완료-조건)을 따른다. 요구사항과 구현·실행 검증의 완료 상태를 구분한다. [기존 요구 대응표](REQUIREMENTS-AUDIT.md#티켓-대응표--2026-10-24)에서 전체 범위를 추적한다.
