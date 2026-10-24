# 작업 티켓

모든 활성 요구를 아래 티켓에 연결했다. T-001은 기존 구현·검증 티켓이고 T-002~T-030은 2026-10-24에 요구사항을 전환한 신규 티켓이다. T-002의 설계·독립 검토를 완료했으며 후속 기능 구현과 구분한다.

## 상태와 실행 순서

- **착수 가능:** 미완료 선행 티켓이 없는 1차 작업. 구현 전 관련 설계를 확인한다.
- **대기:** 선행 완료 후 수행한다. 작성된 티켓을 기능 완료로 간주하지 않는다.
- **2차·백로그·후속 검토:** 범위 보존/구체화 작업이다. 1차 실행 목록에 자동 포함하지 않는다. 각 티켓의 남은 결정이 구체화되면 구현 작업을 나눈다.
- **진행 중:** 구현·검증을 수행하고 있거나 필수 미검증이 남은 작업.
- **완료:** 해당 티켓의 수용 기준·검증·변경 전후 검토를 완료한 상태. 설계 티켓 완료는 제품 기능 완료가 아니다.

1차 순차 실행안: [T-002](T-002-domain-contract.md) → [T-020](T-020-ui-foundation.md) → [T-003](T-003-materials-and-units.md) → [T-005](T-005-business-history.md) → [T-006](T-006-purchase-orders.md) → [T-007](T-007-receipts.md) → [T-004](T-004-recipes-and-costs.md) → [T-008](T-008-production.md) → [T-009](T-009-sales-consumption.md) → [T-010](T-010-stocktake-and-waste.md) → [T-011](T-011-late-receipt-reconciliation.md) → [T-012](T-012-corrections-and-versions.md) → [T-013](T-013-purchase-returns.md) → [T-014](T-014-closing-workspace.md) → [T-015](T-015-photos-and-suppliers.md) → [T-016](T-016-statistics-and-history.md) → [T-017](T-017-operations-overview.md) → [T-018](T-018-full-state-restore.md) → [T-019](T-019-data-compatibility.md) → [T-021](T-021-first-release-verification.md).

T-002는 계산·데이터 계약, T-020은 공통 UI 기반으로 서로 독립적이다. T-001에서 이미 구현한 실행/저장 기반을 이용하되 별도 장비·실제 절전 등 남은 검증을 완료한 것으로 바꾸지 않는다. T-021은 T-017·T-019 완료 후 통합 검증에 착수할 수 있으며 T-001의 실환경 검증 미완료 때문에 실행 가능한 검증을 대기하지 않는다. T-021의 최종 완료·배포 준비 완료에는 T-001의 필수 잔여 검증도 필요하다.

| 티켓 | 단계·현재 상태 | 선행 |
| --- | --- | --- |
| [T-001: Rust·Nuxt 휴대 실행 검증과 첫 저장 흐름](T-001-portable-local-app.md) | 진행 중 — 실환경 미검증 유지 | 없음 |
| [T-002: 수량·원가와 소모 보정 계산 계약](T-002-domain-contract.md) | 1차 · 설계 완료 — 독립 검토 통과 | 없음 |
| [T-003: 재료·상태·단위와 사전 가격 관리](T-003-materials-and-units.md) | 1차 · 착수 가능 | [T-002](T-002-domain-contract.md), [T-020](T-020-ui-foundation.md) |
| [T-004: 레시피·메뉴 복제와 사전 원가 계산](T-004-recipes-and-costs.md) | 1차 · 대기 | [T-003](T-003-materials-and-units.md), [T-005](T-005-business-history.md) |
| [T-005: 업무 기록·임시 저장·확정과 JSON 기록 확장](T-005-business-history.md) | 1차 · 대기 | [T-002](T-002-domain-contract.md), [T-003](T-003-materials-and-units.md) |
| [T-006: 발주 내역과 미입고 주문량](T-006-purchase-orders.md) | 1차 · 대기 | [T-003](T-003-materials-and-units.md), [T-005](T-005-business-history.md) |
| [T-007: 실측 입고와 보유 평균 원가](T-007-receipts.md) | 1차 · 대기 | [T-006](T-006-purchase-orders.md) |
| [T-008: 실측 손질·비축 생산과 원가 이월](T-008-production.md) | 1차 · 대기 | [T-004](T-004-recipes-and-costs.md), [T-007](T-007-receipts.md) |
| [T-009: 판매 수량·금액과 실사 소모 연결](T-009-sales-consumption.md) | 1차 · 대기 | [T-004](T-004-recipes-and-costs.md), [T-008](T-008-production.md) |
| [T-010: 명시 폐기·선택 실사와 판매 소모 배분](T-010-stocktake-and-waste.md) | 1차 · 대기 | [T-008](T-008-production.md), [T-009](T-009-sales-consumption.md) |
| [T-011: 실사에 포함된 지연 입고 보정](T-011-late-receipt-reconciliation.md) | 1차 · 대기 | [T-007](T-007-receipts.md), [T-010](T-010-stocktake-and-waste.md) |
| [T-012: 업무 오입력 정정과 버전·아카이브 이력](T-012-corrections-and-versions.md) | 1차 · 대기 | [T-009](T-009-sales-consumption.md), [T-010](T-010-stocktake-and-waste.md), [T-011](T-011-late-receipt-reconciliation.md) |
| [T-013: 구매 취소·반품과 현재 재고 가치](T-013-purchase-returns.md) | 1차 · 대기 | [T-007](T-007-receipts.md), [T-012](T-012-corrections-and-versions.md) |
| [T-014: 판매·생산·폐기·실사를 모아 입력하는 작업 화면](T-014-closing-workspace.md) | 1차 · 대기 | [T-012](T-012-corrections-and-versions.md), [T-013](T-013-purchase-returns.md), [T-020](T-020-ui-foundation.md) |
| [T-015: 사진·대표 사진·설명과 구입처](T-015-photos-and-suppliers.md) | 1차 · 대기 | [T-004](T-004-recipes-and-costs.md), [T-005](T-005-business-history.md) |
| [T-016: 판매 통계·실사 소모·원가 이력 조회](T-016-statistics-and-history.md) | 1차 · 대기 | [T-014](T-014-closing-workspace.md) |
| [T-017: 발주·입고·마지막 실사 중심 운영 개요](T-017-operations-overview.md) | 1차 · 대기 | [T-006](T-006-purchase-orders.md), [T-007](T-007-receipts.md), [T-015](T-015-photos-and-suppliers.md), [T-016](T-016-statistics-and-history.md) |
| [T-018: 사진과 버전 이력을 포함한 전체 상태 복원](T-018-full-state-restore.md) | 1차 · 대기 | [T-014](T-014-closing-workspace.md), [T-015](T-015-photos-and-suppliers.md) |
| [T-019: 이전 DB·JSON 호환과 같은 OS 데이터 이동](T-019-data-compatibility.md) | 1차 · 대기 | [T-018](T-018-full-state-restore.md) |
| [T-020: 공통 탐색·입력 상태·라이트/다크와 반응형 UI](T-020-ui-foundation.md) | 1차 · 완료 — 구현·회귀 검증 통과 | 없음 |
| [T-021: 1차 업무 통합·복구·배포 준비 검증](T-021-first-release-verification.md) | 1차 · 대기 — 최종 완료에는 T-001 필수 검증 필요 | [T-017](T-017-operations-overview.md), [T-019](T-019-data-compatibility.md) |
| [T-022: 2차 내일 메뉴·공유 재료 계획 요구 구체화](T-022-next-day-menu-plan.md) | 2차 · 구체화/착수 대기 | [T-021](T-021-first-release-verification.md) |
| [T-023: 2차 실제 작업시간 기록과 시계열](T-023-work-time-history.md) | 2차 · 구체화/착수 대기 | [T-021](T-021-first-release-verification.md) |
| [T-024: 시설 점검 카테고리·항목·일일 기록](T-024-facility-checks.md) | 백로그 · 구체화/착수 대기 | 없음 |
| [T-025: 부분 입고와 미입고 잔량 추적](T-025-partial-receipts.md) | 백로그 · 구체화/착수 대기 | [T-006](T-006-purchase-orders.md), [T-007](T-007-receipts.md) |
| [T-026: 보관 위치별 재고·이동 요구 구체화](T-026-storage-locations.md) | 백로그 · 구체화/착수 대기 | 없음 |
| [T-027: Windows↔macOS 데이터 왕복 검증](T-027-cross-os-data-transfer.md) | 백로그 · 구체화/착수 대기 | [T-019](T-019-data-compatibility.md) |
| [T-028: 안정화 후 서버 이관 계획](T-028-server-migration.md) | 후속 검토 · 구체화/착수 대기 | [T-021](T-021-first-release-verification.md) |
| [T-029: 포장재·소모품·인건비 등 원가 확장 검토](T-029-extended-costs.md) | 후속 검토 · 구체화/착수 대기 | [T-021](T-021-first-release-verification.md), [T-023](T-023-work-time-history.md) |
| [T-030: 소모를 연결할 판매 직접 선택](T-030-manual-consumption-allocation.md) | 백로그 · 구체화/착수 대기 | [T-010](T-010-stocktake-and-waste.md), [T-012](T-012-corrections-and-versions.md) |

## 공통 완료 조건

1. [최신 요구사항](../REQUIREMENTS.md)과 [ADR-007](../decisions/ADR-007-consumption-and-ticket-scope.md)에 따른 수용 기준을 구현/검증 근거로 확인한다. 기존 자동 폐기·판매 즉시 차감 규칙을 재도입하지 않는다.
2. 각 티켓은 입력→저장→조회 및 잘못된 입력/빈 상태/오류/재시도까지 제공한다. 구현 위치는 기존 `src/`·`frontend/app/`·`tests/` 경계를 활용하고 필요 이상으로 구조를 바꾸지 않는다.
3. SQLite 변경·JSON 기록·요청 중복 방지·버전/정정 참조를 함께 검증한다. 스키마를 바꾸는 티켓마다 이전 DB와 JSON fixture를 갱신한다. 사진·전체 복원 최종 통합은 T-018~T-019에서 검증한다.
4. 예제·테스트는 가상 자료만 사용하고 MIT·제3자 라이선스 고지를 보존한다. 공개 코드/문서·정적 자산에 실제 운영 자료·인증자료·비공개 링크·개인 경로를 넣지 않는다.
5. 구현 전후 프로젝트 검토를 수행하고 요구사항·관련 결정·진행 상태를 함께 갱신한다. 중대한 변경은 연결 기획 노트에도 반영한다. 문서마다 같은 정책을 복제하기보다 정본과 티켓을 연결한다.
6. 변경 범위에 맞춰 아래 실제 명령과 수용 시나리오를 실행한다. 실행하지 못한 항목은 사유를 기록하고 통과로 표시하지 않는다.
7. 제품 변경은 코드·설정·README와 문서 상태의 불일치를 해결한다. 저장/버전 변경의 되돌리기는 검증된 보존본·복원 절차를 따르며 구버전 바이너리 실행만으로 DB를 되돌릴 수 있다고 가정하지 않는다.

| 변경 | 관련 검증 |
| --- | --- |
| 문서 | 변경 파일 재읽기, 상대 링크·UTF-8/LF, `git diff --check`, `git status --short`, `git check-ignore --no-index -v`; 미추적 파일 직접 검토 |
| UI 포함 | `npm --prefix frontend ci`(의존성 준비 필요 시), `node scripts/build.mjs` → `npm --prefix frontend test`, `npm --prefix frontend run typecheck`, 관련 브라우저 시나리오 |
| Rust·저장·계산 | 최신 UI 산출물 준비 뒤 `cargo fmt --check`, `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings` |
| 브라우저 | Chromium 준비 뒤 `npm --prefix frontend run test:e2e`. 필요한 시나리오를 추가하고 실제 OS/브라우저 기록 |
| 내장 자산·산출물 | `node --test scripts/ui-manifest.test.mjs`, `node scripts/ui-manifest.mjs --check`, `node --test scripts/release-artifact.test.mjs` |
| 실환경 | T-001 기준의 별도 Windows/Mac·절전·브라우저/권한/배포 시험. 일반 E2E와 구분 |

명령이 바뀌면 README와 검토 기준도 함께 갱신한다. 단순 문서 변경에 앱 빌드 성공을 주장하지 않는다.

## 범위 추적과 미결정

[A-01~A-32 대응표](../REQUIREMENTS-AUDIT.md#티켓-대응표--2026-10-24)가 누락 여부의 색인이다. 현재 1차 운영 인터뷰에서 남은 질문은 없다. T-002에서 구체화할 계산 정밀도·실사 구간/정정 알고리즘은 기술 설계다. 기존 정책으로 결과를 결정할 수 없는 새 사례가 나타나면 영향받는 부분만 보류하고 사례로 확인한다.

2차·백로그의 남은 정책은 해당 티켓에 명시했으며 확정된 구현 조건으로 취급하지 않는다. 소비기한·초기 자료 이관·공동 접속·잠금·POS·고객 환불·발주 전송 등 제외 항목은 구현 티켓을 만들지 않고 요구사항의 제외 범위로 추적한다.
