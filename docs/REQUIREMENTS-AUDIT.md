# 요구사항 일괄 재검사 · 2026-10-24

> 최신 상태: 같은 날 후속 인터뷰의 [ADR-007](decisions/ADR-007-consumption-and-ticket-scope.md)에서 실사로 소모 확정·판매 개수 비례 배분·현재 평균 단가 반품을 채택했다. 아래 기존 재검사는 당시 이력이다. 최신 범위는 문서 끝의 [티켓 대응표](#티켓-대응표--2026-10-24)와 [요구사항](REQUIREMENTS.md)을 따른다.

## 판정

**핵심 범위와 O-01~O-04의 네 운영 정책이 확정됐다.** 초기 요구사항·화면 설계·레포 초기화 채팅과 후속 사용자 결정을 32개 점검 묶음으로 대조했다. 축약된 조회·화면·후속 범위를 보완했고, 재검사 때 분리한 네 권장 방향은 2026-10-24 사용자 승인으로 모두 채택했다. 이후 기술스택 인터뷰로 이어졌다.

후속 기술 결정(2026-10-24): 인터뷰에서 Rust·Nuxt, 로컬 서버·브라우저 자동 열기·WebSocket heartbeat 종료, 새 앱의 이전 DB 지원을 확정했다. 앱 내부 업데이트·Electron은 현재 제외한다. [ADR-005](decisions/ADR-005-rust-nuxt-local-server.md)와 [T-001](tickets/T-001-portable-local-app.md)을 기준으로 세부 설계·실행 검증을 이어간다. 아래 32개 점검은 운영 요구사항 검토 이력이다.

이 검토는 문서의 반영 여부와 설계 적합성 평가다. 실행 가능한 앱·DB·복원 기능을 검증한 것은 아니다. 기준은 [REQUIREMENTS](REQUIREMENTS.md), [ADR-003](decisions/ADR-003-inventory-events-and-portable-data.md), [ADR-004](decisions/ADR-004-inventory-exception-policies.md)이며, 기존 기록과 다르면 최신 사용자 결정이 우선한다.

## 요청 대조표

| ID | 사용자 요청·현재 해석 | 요구사항 내 확인 위치 | 재검사 결과 |
| --- | --- | --- | --- |
| A-01 | 셰프 1명·단일 PC에서 사용하는 식당 운영 도구 | 목적과 개발 단계 | 반영 |
| A-02 | 1차 레시피·원가·발주·입고·판매·재고 | 목적과 개발 단계 | 반영 |
| A-03 | 내일 메뉴 계획·비정기 사후 작업시간과 시계열은 2차 | 후속 범위 보존 | 세부 요구 보완 |
| A-04 | 시설 점검은 백로그, 카테고리·하위 항목 편집 및 일일 완료 | 후속 범위 보존 | 세부 요구 보완 |
| A-05 | 부분 입고·보관 위치는 백로그 | 목적과 개발 단계 | 반영 |
| A-06 | 소비기한 추적·POS 연동 제외, 초기 자료 이관은 별도 요청 | 목적과 개발 단계 | 반영 |
| A-07 | 원물 → 손질 → 비축메뉴 → 판매메뉴, 원물/손질분 선택 | 재료·메뉴 | 반영 |
| A-08 | 판매 전 주방 식재료·음식은 모두 재고, 비축 생산량·잔량 관리 | 재료·메뉴 | 반영 |
| A-09 | 마감의 원물 차감·실제 결과물 추가, 수율 계산안 대체 | 입력 시점과 재고 처리 | 최신 결정 반영 |
| A-10 | 재료 고유 ID, 기본 g·자유 단위, 레시피에도 같은 관리 단위 | 재료·메뉴 | 단위 연결 명시 보완 |
| A-11 | 메뉴 1개 단위, 크기별 별도 등록, 독립 복제와 (2) 이름 | 재료·메뉴 | 반영 |
| A-12 | 메뉴·재료 검색과 양방향 연결 | 검색·조회·운영 개요 | 검색 명시 보완 |
| A-13 | 메뉴의 재료·분량·조리 정보·원가와 판매 통계 | 검색·조회·운영 개요 | 통계표·조리 정보 보완 |
| A-14 | 재료의 사용 메뉴·메뉴별 예상 사용량·소진 차이 조회 | 검색·조회·운영 개요 | 분류·예상/실제 구분 보완 |
| A-15 | 재료·메뉴 사진·설명·구입처 링크와 설명 | 재료·메뉴 | 반영 |
| A-16 | kg·L 등 실측 발주·입고, 주문/입고/현재고/예상 재고 구분 | 입력 시점과 재고 처리; 검색·조회·운영 개요 | 조회 구분 보완 |
| A-17 | 입고 수량·금액·과거 입고일 지정, 실제 입력 시각 분리 | 입력 시점과 재고 처리 | 반영; 현재 반영·소급 계산 없음 확정(O-01) |
| A-18 | 마감 판매 개수와 지정 재료 폐기, 직원식은 임의소진/폐기 | 입력 시점과 재고 처리 | 반영 |
| A-19 | 확인한 현재고만 입력, 미확인 유지, 날짜별 마감 불필요 | 입력 시점과 재고 처리 | 반영 |
| A-20 | 일괄 검사와 부족분 자동 폐기, 이중 차감 방지 | 일괄 재고검사와 자동 폐기 | 반영 |
| A-21 | 입고 수량·금액을 반영하는 현재 보유 평균 원가 | 원가 | 반영 |
| A-22 | 손질 후 원가 총액 이월, 3kg·3만원 → 2kg → 100g당 1,500원 | 원가 | 반영 |
| A-23 | 실사 부족의 손실 원가를 잔존 재고에 배분, 8kg·10만원 → 12,500원/kg | 원가 | 반영; 잔량 0은 별도 손실 종료 확정(O-02) |
| A-24 | 취소·반품 때 해당 수량·구입금액 차감 | 입력 시점과 재고 처리 | 반영 |
| A-25 | 바이너리 하나와 옆 db/ 하나, SQLite·JSON·사진 모두 db/ | 실행·파일·복원 | 최신 결정 반영 |
| A-26 | SQLite 기본 저장, 변경마다 JSON 기록·무기한 보관 | 실행·파일·복원 | 반영 |
| A-27 | 전체 상태 복원·고유명칭 사진·비밀번호 잠금 제외 | 실행·파일·복원 | 반영 |
| A-28 | 다른 PC·Mac에서 호환 빌드로 같은 데이터 사용, 안정화 후 서버 이관 | 실행·파일·복원 | 반영 |
| A-29 | 단순한 웹 UI, PC·태블릿·모바일 크기, 라이트·다크, 긴 세로 스크롤 | 화면 초안과 변경 관리 | 디자인 조건 명시 보완 |
| A-30 | Figma는 참고안, 필요하면 전면 재설계 | 화면 초안과 변경 관리 | 반영 |
| A-31 | 직접 개발·코드 공개, MIT와 원본 저장소 URL 고지 | README; LICENSE; ADR-001 | 파일 확인 |
| A-32 | 레포·옵시디언 병행 기록, 지정 프로젝트 폴더와 로컬 obsidian 스킬 | 화면 초안과 변경 관리; 로컬 연결 설정 | 경로·25개 스킬 파일·호환 링크 확인 |

화면 제작·엑셀 분석·문서 정리·EPUB 내보내기 같은 이전 작업 결과는 기획 노트의 이력으로 보존한다. 엑셀 수식을 그대로 구현하거나 과거 임시 Figma 정책을 채택한 요구로 보지 않는다. 취소된 로컬 지침 전문의 노트 복제도 재개하지 않는다.

## 바로잡은 부분

- 조회가 있다는 서술만으로 빠질 수 있던 **메뉴·재료 검색, 메뉴별 예상 사용량, 판매 통계표, 원가 구성, 조리 정보**를 명시했다.
- **1차 운영 개요**와 **2차 내일 메뉴 계획**을 분리했다. 발주 내역·입고 현황·현재 재고를 한곳에서 보는 원래 요구를 유지한다.
- 후속 범위도 이름만 남기지 않고 비정기 사후 작업시간·시설 점검의 설정 구조를 보존했다.
- 전량 폐기·실사 초과·지연 입고·정정 정책을 개발자만 정할 항목으로 묶던 표현을 바로잡았다. 재검사 당시 제안으로 구분한 뒤, 후속 사용자 승인에 따라 ADR-004의 확정 사항으로 갱신했다.
- 날짜별 마감·수율 선택·DB/JSON 직접 인접 저장은 대체 이력이다. 현재 파일 위치는 모두 db/이며 입고일 지정만 날짜 입력의 예외다.

## 적합성 검토

업무 구조는 실제 수량과 원가를 함께 관리하는 목적에 맞다. 손질·비축 생산에서 원가를 이월하고, 판매에서 직접 사용한 재고만 차감하면 하위 재료를 두 번 계산하지 않을 수 있다. 실사 부족분은 원인을 확정한 실물 폐기가 아니라 사용자가 선택한 **자동 폐기 분류**다. 메뉴별 추가 사용량으로 임의 배분하지 않고 실사 차이 이력으로 구분한다.

폐기 후 잔량이 남으면 수량만 감소하고 총원가가 유지되므로 남은 재료 단가가 올라간다. 전량 폐기는 남은 원가를 별도 손실로 종료하며 다음 입고에 이월하지 않는다(O-02). 이는 사용자가 선택한 운영 원가 규칙이며 그 자체를 오류로 수정하지 않는다. 구매 지출, 보유 원가, 판매 사용 원가, 실사 차이를 구분해 표시하고 폐기 금액을 잔존 재고에 포함하면서 다시 총비용에 중복 합산하지 않도록 설계해야 한다.

SQLite는 OS·아키텍처 간 이동 가능한 파일 형식을 제공하므로 현재의 데이터 이동 방향과 부합한다. 앱 스키마·사진 경로·버전 호환성은 별도 설계가 필요하다. [SQLite 파일 형식 설명](https://www.sqlite.org/appfileformat.html)

실행 중인 DB 파일을 임의 복사해도 항상 안전하다는 뜻은 아니다. WAL 모드에서는 WAL이 영속 상태의 일부이므로 정상 종료 후 일관된 파일을 옮기거나 안전한 내보내기 방법을 마련해야 한다. 이 보조 파일도 db/ 안에 두면 바깥의 바이너리+폴더 구성을 유지할 수 있다. [SQLite WAL](https://www.sqlite.org/wal.html), [SQLite Backup API](https://www.sqlite.org/backup.html)

변경마다 JSON 기록은 모든 변경의 복구 가능성을 뜻한다. 매번 전체 DB와 사진을 새로 복제하는 방식까지 확정한 것은 아니다. 저장이 확정된 업무 변경 단위, 스냅샷·변경 기록 조합, 장애 시 재시도·순서·무기한 이력 및 과거 사진 참조를 보존하는 방식을 설계한다. JSON을 SQLite와 독립적으로 양방향 편집하는 두 번째 원본으로 취급하지 않는 방향을 권장한다.

바이너리+db/ 요구는 UI 자산·실행 의존성·OS별 배포물을 검토할 기준이다. 특정 프레임워크가 이를 만족한다고 가정하지 않고 기술스택 인터뷰에서 공식 문서와 작은 실행 검증으로 확인한다. 인터뷰를 시작하기 위해 서버 운영·POS·초기 자료 이관·시설 점검을 결정할 필요는 없다.

## 운영 정책 확정과 기술 인터뷰

[요구사항의 O-01~O-04](REQUIREMENTS.md#확정한-운영-예외)와 [ADR-004](decisions/ADR-004-inventory-exception-policies.md)에 채택 내용을 기록했다. 지연 입고의 현재 반영·실사 중복 방지, 전량 폐기 원가 종료, 초과 재고 기존 단가 조정, 원기록 보존 정정·부족 입력 임시 저장을 다시 선택받지 않는다.

후속 인터뷰의 기술 선택은 ADR-005에 반영했다. 사용자 지정 스택을 재선택하지 않고 Nuxt 자산 내장·OS별 실행 의존성·heartbeat·DB 마이그레이션을 검증한다. 세부 CPU 사양은 배포 검증에서 확인한다. SQLite·단일 PC·db/·JSON 기록·자료 이관 제외 등의 확정 조건도 유지한다.

## 검증 범위

- 원본 채팅의 사용자 요청과 후속 변경을 중복 제거하여 대조했다.
- 공개 요구사항, 구조, TODO, 현재 ADR, 로컬 연결 설정, 기획 노트를 읽었다.
- 스킬 전체 트리 25개 파일과 호환 링크, 노트 위치, MIT·저장소 URL 고지를 확인했다.
- 문서 링크·결정 목록·예시 계산·UTF-8/LF·변경 diff·Git 인덱스 보존을 확인했다.
- 당시 앱 실행·빌드·재고 계산 구현·파일 복원은 미검증이었다. 이후 T-001의 실행/재료 저장 기반을 구현했으며 현재 상태는 해당 티켓을 따른다.

## 티켓 대응표 · 2026-10-24

아래가 최신 상태다. 위 A-01~A-32 표는 당시 재검사 이력이며 정책이 달라진 항목은 아래 대체 관계와 ADR-007을 따른다. T-002~T-030 작성은 구현 완료가 아니다.

| 기존 ID | 요구 묶음 | 담당 티켓 | 최신 처리 |
| --- | --- | --- | --- |
| A-01 | 단일 셰프·PC | [T-001](tickets/T-001-portable-local-app.md), [T-020](tickets/T-020-ui-foundation.md), [T-021](tickets/T-021-first-release-verification.md) | 유지 |
| A-02 | 1차 업무 전체 | [T-002](tickets/T-002-domain-contract.md), [T-003](tickets/T-003-materials-and-units.md), [T-004](tickets/T-004-recipes-and-costs.md), [T-005](tickets/T-005-business-history.md), [T-006](tickets/T-006-purchase-orders.md), [T-007](tickets/T-007-receipts.md), [T-008](tickets/T-008-production.md), [T-009](tickets/T-009-sales-consumption.md), [T-010](tickets/T-010-stocktake-and-waste.md), [T-011](tickets/T-011-late-receipt-reconciliation.md), [T-012](tickets/T-012-corrections-and-versions.md), [T-013](tickets/T-013-purchase-returns.md), [T-014](tickets/T-014-closing-workspace.md), [T-015](tickets/T-015-photos-and-suppliers.md), [T-016](tickets/T-016-statistics-and-history.md), [T-017](tickets/T-017-operations-overview.md), [T-018](tickets/T-018-full-state-restore.md), [T-019](tickets/T-019-data-compatibility.md), [T-020](tickets/T-020-ui-foundation.md), [T-021](tickets/T-021-first-release-verification.md) | 티켓 분해 |
| A-03 | 내일 메뉴 계획·작업시간 | [T-022](tickets/T-022-next-day-menu-plan.md), [T-023](tickets/T-023-work-time-history.md) | 2차 구체화 |
| A-04 | 시설 카테고리/항목·일일 완료 | [T-024](tickets/T-024-facility-checks.md) | 백로그 |
| A-05 | 부분 입고·보관 위치 | [T-025](tickets/T-025-partial-receipts.md), [T-026](tickets/T-026-storage-locations.md) | 백로그 |
| A-06 | 소비기한·POS·초기 자료 이관 제외 | — | 요구사항 제외 범위로 추적; 구현 티켓 없음 |
| A-07 | 원물·손질·비축·판매 직접 참조 | [T-002](tickets/T-002-domain-contract.md), [T-003](tickets/T-003-materials-and-units.md), [T-004](tickets/T-004-recipes-and-costs.md), [T-008](tickets/T-008-production.md) | 유지 |
| A-08 | 판매 전 원물·손질·비축 관리 | [T-003](tickets/T-003-materials-and-units.md), [T-008](tickets/T-008-production.md), [T-010](tickets/T-010-stocktake-and-waste.md) | 실사 중심 표시 |
| A-09 | 실제 수량 손질/생산 | [T-008](tickets/T-008-production.md) | 수율 강제 제외 유지 |
| A-10 | 고유 ID·단위·환산 | [T-002](tickets/T-002-domain-contract.md), [T-003](tickets/T-003-materials-and-units.md), [T-004](tickets/T-004-recipes-and-costs.md) | 유지 |
| A-11 | 메뉴 1개·크기·독립 복제 | [T-004](tickets/T-004-recipes-and-costs.md) | 유지 |
| A-12 | 이름 검색·양방향 연결 | [T-003](tickets/T-003-materials-and-units.md), [T-004](tickets/T-004-recipes-and-costs.md), [T-016](tickets/T-016-statistics-and-history.md) | 유지 |
| A-13 | 조리 정보·원가·판매 통계 | [T-004](tickets/T-004-recipes-and-costs.md), [T-009](tickets/T-009-sales-consumption.md), [T-016](tickets/T-016-statistics-and-history.md) | 판매가·금액·단순 원가율 추가 |
| A-14 | 재료별 메뉴 사용·소모 차이 | [T-010](tickets/T-010-stocktake-and-waste.md), [T-016](tickets/T-016-statistics-and-history.md) | 예상 차감 대신 실사 소모·판매 개수 비례 배분 |
| A-15 | 사진·설명·구입처 | [T-015](tickets/T-015-photos-and-suppliers.md) | 여러 사진/대표·여러 구입처 확정 |
| A-16 | 실측 발주/입고·조회 | [T-006](tickets/T-006-purchase-orders.md), [T-007](tickets/T-007-receipts.md), [T-017](tickets/T-017-operations-overview.md) | 예상 현재고 제거, 실사 수량/시각·확정 변동 구분 |
| A-17 | 입고일/입력 시각·지연 입고 | [T-007](tickets/T-007-receipts.md), [T-011](tickets/T-011-late-receipt-reconciliation.md) | 실사 포함 수량 확인·중복 방지 |
| A-18 | 판매·명시 폐기·직원식 | [T-009](tickets/T-009-sales-consumption.md), [T-010](tickets/T-010-stocktake-and-waste.md), [T-014](tickets/T-014-closing-workspace.md) | 판매는 기록, 실사에서 소모 확정 |
| A-19 | 선택 실사·미확인 유지 | [T-010](tickets/T-010-stocktake-and-waste.md), [T-014](tickets/T-014-closing-workspace.md) | 유지 |
| A-20 | 일괄 실사·차이 처리 | [T-010](tickets/T-010-stocktake-and-waste.md) | 자동 폐기 폐지, 판매 개수 배분/미배분 |
| A-21 | 보유 평균 원가 | [T-002](tickets/T-002-domain-contract.md), [T-003](tickets/T-003-materials-and-units.md), [T-004](tickets/T-004-recipes-and-costs.md), [T-007](tickets/T-007-receipts.md), [T-010](tickets/T-010-stocktake-and-waste.md) | 사전 가격·마지막 단가·0원 지원 추가 |
| A-22 | 손질 원가 이월 | [T-008](tickets/T-008-production.md) | 다중 결과물 주 100% 기본·수정 가능 |
| A-23 | 손실 원가 배분 | [T-010](tickets/T-010-stocktake-and-waste.md) | 명시 폐기에만 잔존 배분/전량 종료; 정상 소모는 가치 차감 |
| A-24 | 취소·반품 | [T-006](tickets/T-006-purchase-orders.md), [T-012](tickets/T-012-corrections-and-versions.md), [T-013](tickets/T-013-purchase-returns.md) | 반품은 현재 평균 단가; 원구입액 참고 보존 |
| A-25 | 바이너리+db/·사진 | [T-001](tickets/T-001-portable-local-app.md), [T-015](tickets/T-015-photos-and-suppliers.md), [T-019](tickets/T-019-data-compatibility.md) | 유지 |
| A-26 | 변경마다 JSON·무기한 | [T-001](tickets/T-001-portable-local-app.md), [T-005](tickets/T-005-business-history.md), [T-018](tickets/T-018-full-state-restore.md), [T-021](tickets/T-021-first-release-verification.md) | 업무·버전/배분까지 확장 |
| A-27 | 전체 복원·사진·잠금 제외 | [T-015](tickets/T-015-photos-and-suppliers.md), [T-018](tickets/T-018-full-state-restore.md) | 유지 |
| A-28 | OS 데이터 이동·서버 이관 | [T-019](tickets/T-019-data-compatibility.md), [T-027](tickets/T-027-cross-os-data-transfer.md), [T-028](tickets/T-028-server-migration.md) | 같은 OS 1차, OS 왕복 백로그, 서버 후속 |
| A-29 | 반응형·테마·스크롤 | [T-020](tickets/T-020-ui-foundation.md), [T-021](tickets/T-021-first-release-verification.md) | 유지 |
| A-30 | 기존 화면은 참고 | [T-020](tickets/T-020-ui-foundation.md) | 유지 |
| A-31 | 직접 개발·공개·MIT | [T-001](tickets/T-001-portable-local-app.md), [T-021](tickets/T-021-first-release-verification.md) | 각 티켓 공통 완료 조건으로 지속 검증 |
| A-32 | 공개 요구·연결 노트 동기화 | [T-021](tickets/T-021-first-release-verification.md) | 모든 티켓 공통 완료 조건으로 유지 |

추가 결정의 대응: P-01→T-004·T-009·T-016, P-02·P-11→T-003·T-004·T-009·T-012, P-03→T-002~T-004, P-04→T-008, P-05·P-06→T-006·T-007, P-07→T-011, P-08→T-009·T-010·T-014·T-016·T-017·T-030, P-09→T-009·T-012, P-10→T-013, P-12→T-015이다. 원가 확장 검토는 T-029에 보존한다.

신규 티켓 문서에는 목적·포함/제외·선행·수용 기준·검증 방법·남은 결정·미실행 상태를 담았다. 1차에 확정한 정책과 2차/백로그 착수 시 남은 결정을 구분한다. 제외 항목을 실행 가능한 기능 티켓으로 되살리지 않는다.

## 티켓 전환 검토 결과

검토 대상은 문서 전환이다. 앱 구현·실행 테스트·실환경 배포 검증은 수행하지 않았으며 T-001의 미검증 항목을 유지했다.

- [Step 1] 목적 적합성 - PASS: 기존 A-01~A-32와 후속 P-/Q- 결정을 담당 티켓 또는 명시적 제외 범위로 연결했다.
- [Step 2] 오류·중요 문제·보안 - PASS: 공개 문서의 개인 경로·비공개 자료 패턴을 점검했고 가상 사례만 추가했다.
- [Step 3] 회귀 - PASS: 대체된 정책은 ADR-007로 명시하고 실행/저장 범위·기존 검증 결과를 유지했다. 연결 노트의 메타데이터·기존 링크·엑셀 분석·화면 참고와 기존 변경을 보존했다.
- [Step 4] 파일·함수 크기 - PASS: 기능별 티켓과 공통 완료 조건으로 나눴으며 제품 코드를 변경하지 않았다.
- [Step 5] 재사용·통합 - PASS: 기존 요구사항·TODO·ADR·티켓 색인을 사용했다. 별도 작업 관리 서비스를 도입하지 않았다.
- [Step 6] 부작용 - PASS: 문서만 변경했다. 외부 전송·배포·백그라운드 서비스·DB/운영 자료 변경은 없다.
- [Step 7] 전체 변경 재검토 - PASS: 추적 diff와 신규 티켓을 재읽고 정책·의존 관계를 대조했다.
- [Step 8] 불필요한 잔여물 - PASS: 초안의 원구입액 차감 표현과 후속 기능을 선행 완료 조건으로 요구하던 문장을 수정했다.
- [Step 9] 품질 검증 - PASS: 상대 링크/앵커·UTF-8/LF·티켓 ID/의존 순환·실행 순서·요구 대응·제외 규칙·diff 공백 검사를 수행했다.
- [Step 10] 사용자·개발자 흐름 - PASS: README→요구사항/TODO→티켓→결정/검증으로 이동한다. 1차 순서와 후속 구체화 대기를 구분했다.
- [Step 11] 발견 문제의 원인 - PASS: 인터뷰 중 정책 변경으로 남은 초안 표현과 기능 간 완료 조건을 정리하고 관련 티켓을 다시 확인했다.
- [Step 12] READY 판단 - PASS: 요구사항의 티켓 전환은 READY다. 신규 기능 구현과 T-001 실환경 검증 완료를 뜻하지 않는다. 2차/백로그의 남은 정책은 각 티켓 착수 조건으로 보존한다.
- [Step 13] 커밋·PR 준비 - PASS: 요청 범위에 없으므로 스테이징·커밋·PR을 수행하지 않았다.
