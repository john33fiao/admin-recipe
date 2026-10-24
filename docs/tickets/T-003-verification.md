# T-003 검증 기록

- 작성일: 2026-10-24
- 관련: [티켓](T-003-materials-and-units.md), [계획·RTD Before](T-003-plan.md)
- 상태: READY — 재료/사전 가격 구현·통합 검증 완료

## 구현 범위

6자리 고정소수 수량/금액과 g/kg·mL/L 단위, 품목 종류·설명·버전·아카이브·이름 검색, 사전 가격과 가격 출처 표시를 추가한다. 품목 원본 UUID와 최초 등록 응답은 보존하며 수정은 새 버전이다. 가격은 기준 금액/수량의 비율로 보존하고 0원과 가격 없음은 다르다. 가격 등록은 입고나 보유량을 만들지 않는다.

스키마 v3와 JSON v2의 전체 품목 상태·요청 응답을 같은 트랜잭션으로 기록한다. 기존 v1 JSON은 원문을 보존한다. 새 변경은 epoch/revision·품목 버전을 검사하며 이미 처리한 동일 요청은 당시 응답을 재생한다. 미처리 stale 상태와 동일 요청 ID의 다른 내용은 서로 다른 오류 코드다. JSON 발행을 끝내지 못하면 같은 요청을 복구하고 다음 변경을 막는다.

실제 보유 평균·마지막 유효 단가 연결은 T-007에서 검증한다. 이 티켓은 순수 가격 선택의 모든 분기와 사전 가격 저장·화면을 구현한다. 레시피 메뉴 선택·과거 업무 참조는 해당 후속 티켓에서 새 품목 ID/버전 계약을 사용한다.

## 검증 상태

Windows 11 Home x64, Node 24.19.0, Rust 1.99.0, Nuxt 4.6.2, Vue 3.5.44, Playwright 1.64.1, Chromium 156.0.8078.4에서 가상 자료와 격리 임시 DB로 실행했다. Windows 실행/파일 발행 권한이 필요한 빌드·저장·브라우저 테스트는 승인된 확장 권한으로 실행했으며 제품 보안/저장 처리를 완화하지 않았다.

- 순수 domain RED: 모듈 구현 전 `cargo test --locked --test domain`의 unresolved import로 exit 101. 구현 뒤 전체 Cargo에서 domain 4개 GREEN.
- HTTP RED: 신규 state 경로 미구현으로 4개 실패. 기존 등록 1개는 Windows 파일 발행 권한으로 503이어서 기능 RED와 구별한다.
- 저장 테스트 첫 실행은 UI 자산 해시 불일치로 빌드가 차단됐다. 테스트 본문 실패나 통과로 계산하지 않는다.
- UI unit 초기 9개와 추가 검토 경합 회귀 2개를 포함한 최종 11개·typecheck 통과.

| 실행 명령 | 현재 확인 결과 |
| --- | --- |
| `node scripts/build.mjs` | PASS — UI 경쟁 조건 수정 뒤 최종 Nuxt 생성·입력/자산 manifest·Rust release·경로 잔존 검사 |
| `cargo test --locked` | PASS — 54개(라이브러리 7, 실행파일 1, catalog HTTP 5, catalog 저장 8, domain 4, hot journal 4, 기존 HTTP 8, lifecycle 4, 기존 저장 13) |
| `cargo fmt --check` | PASS |
| `cargo clippy --locked --all-targets -- -D warnings` | PASS — 증분 캐시 hard link 불가로 파일 복사하는 환경 경고만 발생 |
| `npm --prefix frontend test` | PASS — 11개. 실제 Vue scope·composable의 등록/편집 검토 경합 2개 포함 |
| `npm --prefix frontend run typecheck` | PASS |
| `node --test scripts/ui-manifest.test.mjs` | PASS — 1개 |
| `node scripts/ui-manifest.mjs --check` | PASS |
| `node --test scripts/release-artifact.test.mjs` | PASS — 1개 |
| `npm --prefix frontend run test:e2e -- catalog.spec.ts --fully-parallel --workers=3 --output=../tmp/t003-catalog-final` | PASS — 5개, 2.1분. 실제 기본 120초 종료 유예·정상 종료 코드와 재시작 포함 |
| `npm --prefix frontend run test:e2e -- local-app.spec.ts ui-foundation.spec.ts --fully-parallel --workers=4 --output=../tmp/t003-existing-e2e` | PASS — 기존 12개, 4.0분. 재시도·JSON 실패·중복 실행·세션·정상 종료/경로 이동·공통 UI 회귀 |

최종 UI 자산을 만든 뒤 전체 Rust 54개를 다시 실행해 통과했다. 빌드의 기존 Nitro external 경고와 Node shell 옵션 사용 중단 예고는 남아 있으며 새 의존성이나 도구 설정을 추가하지 않았다.

## 수용 기준별 증거

| 기준 | 구현·검증 근거 |
| --- | --- |
| T-003-01 | 같은 이름의 서로 다른 UUID 두 품목, 종류/단위 선택·조회와 상세/이력을 저장·HTTP·브라우저에서 확인. 실제 레시피 구성 선택은 T-004에 연결 |
| T-003-02 | 입고 없이 0원과 후속 가격 두 버전 저장·이력·재시작을 확인. 저장/API 검증에서 재고 수량/금액 필드를 생성하지 않으며 0원과 missing 분리 |
| T-003-03 | 순수 resolver의 보유 평균·마지막 유효·사전 가격·없음·미확인 가치·수량0/가치양수 오류 분기 검증. 실제 화면은 사전 가격/없음 표시; 실제 보유 원장 adapter는 T-007에서 검증 |
| T-003-04 | 정보 수정과 보관은 새 버전이며 원래 이름/단위·당시 가격 참조·JSON/요청 응답을 보존. 기본 active 목록 제외와 archived/all/detail 조회 확인 |
| T-003-05 | g→kg 표시는 변경하되 이전 g 버전/가격의 기준 수량을 유지. 질량↔부피 및 서로 다른 자유 단위는 입력 보존·오류·DB 변경0; 새 품목 등록 안내 |
| T-003-06 | 공백 trim·빈 검색·같은 이름·문자 그대로의 `%_` 검색과 보관 범위·정상 종료/재시작 지속성을 확인. 필터 결과와 시작 화면 active 전체 집계 분리 |

상세 화면 1440×900·768×1024·390×844의 라이트/다크 6장을 직접 열어 검토했고 독립 검토자가 대표 2장을 확인했다. 긴 이름·설명·ID/이력 줄바꿈, 모바일 한 열 배치, 키보드 포커스·주요 버튼·가로 넘침 없음을 확인했다. 화면 파일은 로컬 `tmp/t003-catalog-final/`에 보관한다.

## 독립 검토와 발견 문제

저장·마이그레이션·원문/요청 호환과 순수 수치/단위/가격 코드를 독립 검토했다. 마이그레이션은 한 트랜잭션의 순차 변환 뒤 검증하며, 미래/잘못된 옛 JSON을 거절한다. 기존 자료의 종류를 추정하지 않고 `unclassified`로 보존하며 출처 seq를 확인하지 못하면 null로 남긴다.

1. 상세 편집의 입력·기준은 인스턴스 상태인데 요청·진행 여부는 공유 상태여서, 저장 중 이탈 후 응답 전에 재진입하면 완료한 가격 입력과 옛 revision이 남았다. 실제 브라우저에서 지연 응답·이탈·재진입 후 저장 완료 문구는 나타나지만 금액 `1234`가 지워지지 않는 RED(exit 1)를 확인했다. 입력/기준을 공유하고 활성 화면에서 성공 갱신을 처리해 동일 시나리오의 빈 입력·가격 1개·후속 정보 수정 성공을 확인했다.
2. 오류 후 입력 검토의 비동기 조회를 여러 번 실행하면 늦은 조회가 새 요청을 지웠다. 실제 등록 컴포저블을 TypeScript로 변환해 VM에서 실행하고 지연 API 응답만 대체한 검증에서, 두 번째 조회 응답이 새 POST 진행 중 `pending`과 sessionStorage 요청 키를 삭제함을 확인했다. 등록과 상세 편집의 검토·저장을 공유 잠금으로 직렬화하고 요청 동일성을 확인한다. 실제 Vue effectScope로 페이지 해제를 재현한 독립 GREEN과 지속 단위 2개가 중복 검토/전송 차단 및 성공 전 pending 보존을 확인했다.

## 변경 파일

수치/단위/가격은 `src/domain/`과 `tests/domain.rs`, 품목 저장·DTO는 `src/store/catalog.rs`·`src/catalog.rs`·`src/store.rs`, HTTP는 `src/server.rs`, 모듈 등록은 `src/lib.rs`다. `tests/catalog_storage.rs`·`tests/catalog_http.rs`·v2 DB/v1 JSON fixture를 추가하고 기존 `tests/storage.rs`의 신규 출력 형식 검사만 조정했다. 과거 원문 호환 검사는 별도로 유지한다.

Nuxt 등록/목록/상세·시작 집계·DTO/오류·초안·스타일과 신규 편집/필터/composable·수치 helper를 연결했다. 단위 `catalog.test.ts`·`material-review.test.ts`, `catalog.spec.ts`를 추가하고 기존 E2E의 신규 snapshot 경로와 이력 추가로 중복된 단위 레이블 선택만 좁혔다. README·ARCHITECTURE·DOMAIN-CONTRACT·TODO·티켓 색인/계획/결과를 실제 구현 상태에 맞췄다.

## 변경 후 RTD

- [Step 1] 목적 적합성 - PASS: 6개 수용 기준을 위 저장/API/UI 증거에 대응했다. 실제 보유 원장·레시피 연결은 선행 구현 범위와 후속 통합을 구별했다.
- [Step 2] 오류·중요 문제·보안 - PASS: checked 정수·문자열 JSON·단위/UUID/버전 검증, 요청 불변 재시도·복구 게이트·Host/Origin/세션·입력 한도를 유지했다. 공개 후보에서 개인 경로·실제 개발 자료가 섞이지 않음을 확인했다.
- [Step 3] 회귀 - PASS: 기존 DB/JSON fixture·원래 등록 응답·legacy 입력·UUID를 보존하고 기존 Rust/브라우저 회귀를 통과했다. 판매 즉시 차감 등 오래된 정책을 도입하지 않았다.
- [Step 4] 파일·함수 크기 - PASS: 순수 계산과 품목 SQL/DTO·HTTP·화면 상태의 역할을 분리했다. 저장 확장의 기존 단일 트랜잭션 helper는 다음 업무 티켓에서 재사용하며 관련 없는 전면 리팩터링을 추가하지 않았다.
- [Step 5] 재사용·통합 - PASS: Store의 잠금·recover/outbox·종료, 기존 UI 연결/테마/입력 보관을 재사용했다. 새 프레임워크·의존성·외부 자산은 없다.
- [Step 6] 부작용 - PASS: 기존 백업 후 한 트랜잭션으로 schema3을 적용한다. 버전/가격/요청·전체 JSON이 누적되며 기록량 비용은 T-021에서 측정한다. 새 외부 서비스·자동 동기화·전역 설정은 없다.
- [Step 7] 전체 변경 재검토 - PASS: 소유 범위별 구현과 독립 리뷰, 전체 diff/새 파일을 확인했다. 두 UI 경합을 실제 재현한 뒤 수정했고 공개 문서와 연결 노트의 자료 구조/후속 경계를 맞췄다.
- [Step 8] 불필요한 잔여물 - PASS: 임시 산출물·스크린샷·실제 개발 자료·노트 동기화 스크립트는 Git 제외 경로에 둔다. 미구현 재고 수량을 0이나 가짜 업무로 채우지 않는다.
- [Step 9] 품질 검증 - PASS: 최종 표준 빌드, Rust54·UI단위11·브라우저17·typecheck·fmt/clippy·manifest/산출물 검사를 실행했다. UTF-8/LF·상대 링크·diff·후보 ignore 규칙을 확인했다.
- [Step 10] 사용자·개발자 흐름 - PASS: 등록→상세 가격/수정→과거 이력→보관/검색과 정상 종료/재시작을 확인했다. 3너비×2테마를 검토하고 실행 명령·현재 형식을 공개 문서에서 찾을 수 있게 했다.
- [Step 11] 발견 문제의 원인 - PASS: 인스턴스 입력과 공유 요청 상태의 수명 차이, await 구간 검토 잠금 부재를 수정했다. navigation RED→GREEN과 실제 Vue 지속 단위 회귀로 확인했다. 환경상 관측하지 못한 저장 RED는 실행 성공으로 쓰지 않았다.
- [Step 12] READY 판단 - PASS: T-003의 품목/사전 가격·저장 확장 범위는 READY다. 실제 보유 원장 연결·전체 복원·실환경 배포·최종 시연 완료를 뜻하지 않는다.
- [Step 13] 요청된 커밋·PR 준비 - PASS: 현재 사용자의 명시적 승인 범위에서 공개 변경만 논리 단위로 준비한다. 기존 staged 변경이 없는지 확인하고 각 후보의 ignore·unstaged/staged diff를 대조한다. 로컬 자료/노트는 포함하지 않으며 PR·push·배포는 하지 않는다.

## 후속 경계

연결 기획 노트에 같은 날짜·C-/D- 식별자로 품목/가격·schema3/JSON2·초기 revision 0과 불변 원본/후속 경계를 추가했다. 양쪽을 재읽고 경로별 diff 및 추가 블록을 제외한 기존 바이트 일치를 확인했다. 노트의 기존 사용자 변경은 보존하고 커밋하지 않았다.

다음은 T-005 업무 초안·확정 기반이다. 실제 입고 원장 연결은 T-007, 전체 상태 복원은 T-018, 정식 이전 자료와 같은 OS 이동은 T-019의 후속 검증이다. T-001의 별도 깨끗한 Windows/Mac·실제 절전과 T-021 실제 시연은 완료하지 않았다. 신규 브라우저의 정상 종료 시나리오 외 fixture 정리는 강제 종료를 사용하며 별도의 정상 종료 증거로 계산하지 않는다.
