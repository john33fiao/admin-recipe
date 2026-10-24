# T-009 검증 기록

- 상태: 완료 — 판매 구현·검증 READY, 05의 실제 실사 후 배분 표시도 T-010에서 검증
- 기준: [티켓](T-009-sales-consumption.md), [계획과 RTD Before](T-009-plan.md)
- 기록일: 2026-10-25

## 설계 검토

저장·순수/HTTP·UI 담당이 공개 계획을 독립 검토했다. 당시 원가 행의 품목명, 현재 owner 버전과 과거 recipe owner 버전 차이, 가격 없음과 0, 원본 판매 구간과 as-of 개정 선택, count-only 정정 및 T-010/T-012 후속 경계를 대조했고 차단 누락이 없었다. 기존 unknown kind 검사에서 미래 종류 예시로 사용하던 `sale`은 구현 후 실제 미지원 종류로 바꿔 같은 검증 의미를 유지한다.

## 실행 결과

`cargo test --locked --test sale_http`는 정상 컴파일 뒤 1통과·4실패(exit101)였다. 판매 GET/SPA 404, 판매 업무 필터400, 초안 생성의 미지원 종류400을 확인했다. 기존 세션/Host/Origin·16KiB·preview/confirm 보호 및 상태 무변경 검사는 통과했다. 이는 미구현 행동의 확인이며 T-009 기능 통과가 아니다.

`cargo test --locked --test sale_storage`는 정상 컴파일 뒤 schema 기대9/실제8로 1실패(exit101)였다. 이전 release의 정상 API로 가상13개 변경을 캡처했고 원요청13개 재생 시 원응답 바이트·state·outbox/JSON이 일치했다. 품목·발주 연결 입고·전량 생산과 마지막 단가·레시피/복제·미완성 초안을 포함한다. 이 RED에서 실패 뒤 변환 단언은 아직 실행되지 않았다.

독립 설계 검토와 두 행동 RED를 확인한 뒤 제품 구현을 시작했다. 최종 명령·종료 코드·수용 기준별 결과와 RTD After는 실제 결과가 생긴 뒤 추가한다.

UI의 `npm --prefix frontend test`도 기존43통과·신규1실패(exit1)를 확인했다. 판매 개수 0·큰 revision·원정정 confirmation을 가진 pending 요청 복원이 null로 거절되던 행동을 재현했다. 해당 요청 전체를 보존하도록 판매 타입과 복원 경계를 확장 중이다.

## 첫 구현과 보완 검증

`npm --prefix frontend test` 48통과 및 `npm --prefix frontend run typecheck` exit0 뒤 `node scripts/build.mjs`를 실행해 UI 생성·자산 정합성·Rust release·개인 경로 검사를 통과했다. 이어 `cargo test --locked --no-fail-fast --lib --test sale --test sale_http --test sale_storage --test business_payload`에서 57통과·2실패(exit101)를 확인했다. lib14/입력9/순수판매12/HTTP9는 모두 통과했고 저장15 중 아래 두 회귀가 실패했다.

| 발견 | 변경 전 실제 결과 | 보완 방향 |
| --- | --- | --- |
| 원가 상한 초과 레시피의 판매 초안 | 계산 Validation이 초안 저장을 취소함 | 계산 입력 오류는 미리보기 issue로 남기고 입력 저장을 허용하며 확정만 차단. 실제 저장 무결성 오류는 계속 전파 |
| 판매 confirmation의 요청 연결 | 없는 UUID·다른 순번의 실제 UUID·비정규 UUID 3모드를 outbox/JSON에 동일 반영했을 때 모두 수용 | canonical UUID와 같은 순번·business.confirm·request_id의 유일 연결 확인 |

정적 검토에서는 보관 품목의 명시적 현재 참조에 판매 금지를 추가했던 조건을 제거했다. 아카이브의 기본 목록 노출 규칙을 확정 금지로 확대하지 않으며, 해당 판매/재시작 저장 회귀는 위 첫 실행에서 통과했다. 이 변경은 실행 RED 후 수정이라고 표시하지 않는다.

또한 실제 업무 입력 페이지의 script를 실행한 UI 회귀에서, 정정 생성 응답을 잃은 뒤 대상이 다른 탭에서 대체되면 같은 요청 재시도 화면을 열 수 없는 실패를 확인했다(미전송 오래된 대상 차단1통과·원래 pending 복구1실패). 원래 요청에 한해서 복구를 먼저 허용하도록 보완했다. 아래 최종 결과에 저장·UI 회귀 재검증과 실제 브라우저 결과를 기록한다.

실제 실사 writer와 판매 배분은 T-010, 일반 정정은 T-012의 후속 범위다. T-001 별도 장비·실제 절전과 T-021 실제 시연은 미검증으로 유지한다.

## 전체 검증과 최종 UI 확인

수정한 저장 회귀 두 건과 실제 페이지 복구 회귀를 포함해 전체 Rust258개·UI단위51개가 통과했다. UI 단위 중 판매 정정 seed 검증은 원래 pending 복구·미전송 오래된 대상 차단·다른 종류/대상 차단 3개다. frontend 변경 후 표준 빌드를 다시 완료하고 다음 검증을 실행했다.

| 명령·검사 | 실제 결과 |
| --- | --- |
| `npm --prefix frontend test` | 51 PASS, exit0. 최초 복원 RED와 페이지 script 회귀 보완 포함 |
| `npm --prefix frontend run typecheck` | exit0 |
| `node scripts/build.mjs` | UI 생성·입력/자산 정합성·Rust release·경로 잔존 시험 PASS. 마지막 스타일/테스트 대기 보완 뒤 다시 통과 |
| `cargo test --locked --no-fail-fast` | 258 PASS, 0 FAIL. 순수 판매12·판매 HTTP9·판매 저장15·schema8 닫힌 판독3 포함 |
| `cargo fmt --check` | PASS |
| `cargo clippy --locked --all-targets -- -D warnings` | 첫 실행은 테스트의 needless borrow 1건 실패. 불필요한 참조만 제거한 뒤 exit0 |
| 기존 `npm --prefix frontend run test:e2e -- local-app.spec.ts ui-foundation.spec.ts catalog.spec.ts business.spec.ts orders.spec.ts receipts.spec.ts recipes.spec.ts productions.spec.ts --fully-parallel --workers=6` | 50 PASS, exit0, 6.5분 |
| 신규 `npm --prefix frontend run test:e2e -- sales.spec.ts --fully-parallel --workers=3`와 `-- sales.spec.ts -g "unfinished sale survives" --workers=1` | 첫 실행 6 PASS/1 FAIL. 아래 테스트 경합 보완 뒤 영향1 재실행 1 PASS, exit0, 2.1분. 서로 다른 7개 사례 최종 PASS |
| `node --test scripts/ui-manifest.test.mjs`, `node scripts/ui-manifest.mjs --check` | 자체 시험1 PASS·현재 입력/산출물 일치 exit0 |
| 실제 키보드 입력 | Tab·방향키·Enter·Space, 빈 개수 오류 포커스, 메모 Enter 줄바꿈, 0개 초안→미리보기→확정→1개 정정 PASS |

새 브라우저 첫 실패는 확정 업무 화면과 판매 상세에 공통으로 있는 제목만 기다린 뒤 새로고침한 테스트 경합이었다. trace에서 링크 클릭 직후 reload URL이 아직 업무 상세임을 확인했다. 판매 상세 URL에 도달할 때까지 기다리는 단언을 추가했다. 실패 이전의 정상 재시작·입력 보존·판매 확정·당시 원가·재고 불변은 통과했고 제품의 저장/라우트 동작을 바꾸지는 않았다.

모바일 직접 화면 검토에서는 판매 상세의 긴 레이블이 좁은 칸에서 여러 줄로 꺾이는 점을 확인했다. 480px 이하의 판매 상세 설명 목록만 한 열로 바꿨다. 변경 후 단위51·typecheck·UTF-8/LF·표준 빌드·영향 브라우저1개를 다시 통과했다. 1440×900·768×1024·390×844의 두 테마 판매 상세 6개를 직접 검토했고 대표 데스크톱 라이트/모바일 다크를 독립 검토했다. 레이블·당시 원가 행·이력·탐색/버튼·가로 넘침 없음을 확인했다. 파일 감사에서 시험 파일 끝의 빈 줄 하나를 제거한 뒤에도 정식 빌드를 갱신했다. 이 공백 정리로 기능 검증을 중복 실행하지 않는다.

실행 환경은 Windows 11 Home x64, Rust/Cargo1.99.0, Node24.19.0, npm12.2.1, Playwright1.64.1·Chromium156.0.8078.4(revision1248)다. 로그·trace·화면과 검사 보조 파일은 Git 제외 경로에 보존한다. 정상 재시작 사례의 120초 heartbeat 종료 대기와 실패 주입/fixture 정리를 구분하며 별도 장비나 실제 절전 검증으로 계산하지 않는다.

## 수용 기준 대응

| 기준 | 구현·실행 근거 | 남은 경계 |
| --- | --- | --- |
| T-009-01 | 저장/API에서 문자열 개수0~상한·자동 시각·미완성 초안·재시작·수정/확정 검증. 일반 UI 입력과 DB 초안 보존 확인 | 날짜별 마감·실제 재료량 입력을 요구하지 않음 |
| T-009-02 / T-005-02 | 판매 전후 inventory 전체 행 불변, 0수량 레시피·보유 미확인/부족·가격 없음 판매 확정. 참조·형식·범위 오류는 초안에 보존하고 확정 차단 | 판매 예상 차감이나 재고 생성 없음 |
| T-009-03 | 판매가 null/0·개수0·checked 곱셈/넓은 매출 합계. 품목 이름·레시피·가격 변경 뒤 당시 원가 출처/행·금액 유지, 정정에서 원가 재계산 없음 | 목록 소계와 미계산 건수 구별 |
| T-009-04 | 실제 저장 조회와 순수 계약으로 직접 참조0행 포함·중복 제거·비재귀·원본 열린 구간·as-of 유효 개정 검증 | 실제 실사 writer/배분은 T-010 |
| T-009-05 | 모든 현재 판매를 실사 소모 미확정으로 표시하고 당시 레시피 원가와 별도 영역에 제공. 합성 구간의 0가중치/미배분 계약 검증 | 실사 후 실제 배분 조회는 T-010에서 완성 |
| T-009-06 | 3→1→0 개수 정정과 이유·원본/현재 confirmation·버전 체인·원본 순번/당시 참조 유지. stale/변조/중간 저장 실패의 전체 rollback 검증 | 이미 배분된 판매 정정의 영향 처리는 T-012, 환불 없음 |
| T-005-04 | 같은 UUID 원응답 재생·다른 본문 충돌, 독립적인 동일 판매2건·매출 합산, 응답 유실·JSON 실패 뒤 같은 요청 재시도 | T-005에 남겨둔 실제 판매 조건 확인 |

## 변경 파일과 검토 범위

`src/sale.rs`는 엄격 입력·개수/매출·고정 원가 DTO와 직접 참조 계약을, `src/store/sale.rs`는 판매 원본/개정·당시 계산·조회·정정·snapshot 무결성을 담당한다. `src/store/recipe.rs`에서 기존 현재 원가 계산을 재사용 가능한 내부 helper로 분리했다. `src/business.rs`, `src/store/business.rs`, `src/server.rs`, `src/store.rs`, `src/lib.rs`에 공통 업무와 schema9·조회/SPA 경로를 연결했다.

`src/store/snapshot.rs`와 `snapshot/frozen8.rs`는 현재 네 판매 관계와 이전 schema8의 닫힌 판독을 구분한다. frozen6/7의 기존 입고·레시피 타입 가시성만 형제 모듈에 열었으며 옛 JSON 형식을 확대하지 않았다. schema8 가상 DB/JSON fixture와 판매 순수·저장·HTTP 테스트, 업무 payload와 기존 writer schema 기대값을 갱신했다.

Nuxt에는 판매 편집·당시 원가 컴포넌트, 목록/상세·개수 정정 진입, 공통 pending 복원과 업무 분기, 판매 타입/계산·단위/브라우저 검증을 추가했다. 기존 탐색·레시피 상세·스타일·문서도 실제 기능에 맞췄다. 새 의존성이나 외부 서비스를 추가하지 않았다.

저장·HTTP·UI 담당의 독립 검토와 root 검토는 원본 가격 우선순위, 엄격 중첩 JSON, 같은 seq/operation/request_id 연결, 요청 재생/CAS/복구 순서, 당시 owner와 recipe owner 버전 차이, 개수 정정, 공개 경계를 확인했다. 보완 후 추가 차단 결함은 발견하지 못했다.

## 동기화와 후속 작업

연결 기획 노트에 2026-10-25·C-02·D-02·D-04·D-09·P-01~P-03·P-08~P-09를 명시하고 판매 계약·schema9·실사 배분 후속 범위를 추가했다. 추가한 두 블록을 제거하면 기존 노트 전체 바이트와 일치하며 사용자 변경·메타데이터·분석을 보존한다. 양쪽 기록을 재읽고 노트의 경로별 diff와 staged 없음도 확인했다. 노트는 커밋하지 않는다.

## 변경 후 RTD

- [Step 1] 목적 적합성 - PASS: 판매 입력/당시 계산/재고 불변/직접 참조/개수 정정을 실제 저장·HTTP·UI 근거로 대응했다. 05의 실사 후 실제 표시는 T-010 후속 검증으로 열어 둔다.
- [Step 2] 오류·중요 문제·보안 - PASS: 엄격 종류/버전/중첩 입력·길이/정수 범위·UUID/참조, 동일 순번/요청 연결, 원자 저장/복구와 세션·Host·Origin·16KiB를 검증했다. 공개 후보의 개인 자료·경로 혼입을 검사한다.
- [Step 3] 회귀 - PASS: 기존 브라우저50개·전체 Rust258개로 이전 업무·UUID/원응답/JSON 바이트·당시 가격을 보존했다. 판매 즉시 차감이나 아카이브 강제 금지를 추가하지 않았다.
- [Step 4] 파일·함수 크기 - PASS: 순수 계약·판매 저장/무결성·HTTP·UI를 역할별로 분리하고 기존 원가 helper만 추출했다. 저장 검증이 큰 파일은 판매 불변 관계에 한정하며 무관한 리팩터링은 없다.
- [Step 5] 재사용·통합 - PASS: 공통 business/mutation·CAS·recover·정확 수치/가격·UI pending/연결/테마를 재사용했다. 새 의존성이나 범용 정정 엔진을 도입하지 않았다.
- [Step 6] 부작용 - PASS: schema9는 판매 관계를 추가하며 이전 데이터/JSON을 재작성하지 않는다. 누적 기록 비용은 T-021에서 측정한다. 외부 서비스·전역 설정·자동 노트 동기화는 없다.
- [Step 7] 전체 변경 재검토 - PASS: 저장·순수/HTTP·UI와 root의 독립 검토를 반영했다. 최종 diff·새 파일·연결 노트를 확인하고 실제 결과와 후속 범위를 구분했다.
- [Step 8] 불필요한 잔여물 - PASS: 원시 자료·로그·검증 보조·화면은 제외 경로에 보존한다. 가짜 실사 writer·예상 재고·운영 자료 fixture는 없다.
- [Step 9] 품질 검증 - PASS: 표준 build, Rust258·UI51·typecheck·fmt/clippy·기존/신규 브라우저57개·자산 검사와 UTF-8/LF·링크·diff·ignore 검사를 수행한다. 실행 실패와 재검증을 위에 구분했다.
- [Step 10] 사용자·개발자 흐름 - PASS: 실제 정상 종료/재시작·원문 복원·판매/정정/재시도·당시 원가·키보드와 3너비/2테마를 확인하고 README·티켓·TODO에 연결했다.
- [Step 11] 발견 문제의 원인 - PASS: 초안의 계산 Validation 전파·요청 연결 검증 누락·pending 정정 재진입·테스트 이동 대기 경합을 각각 재현하고 좁은 수정 뒤 관련 검증을 통과했다. 정적 검토의 아카이브 조건 제거와 공백 정리는 별도로 기록했다.
- [Step 12] READY 판단 - PASS: 판매 구현 범위는 READY다. 실제 실사 후 배분 표시, 배분 후 정정·전체 복원·격리 시연·별도 OS/절전·배포 준비 완료를 뜻하지 않는다.
- [Step 13] 요청된 커밋·PR 준비 - PASS: 명시적 승인에 따라 이번 공개 변경만 후보로 삼고 기존 staged·ignore·실제 unstaged/staged diff를 확인한다. 로컬 자료/노트는 제외하며 push·PR·배포는 하지 않는다.

다음 순차 작업은 T-010 명시 폐기·선택 실사·실제 판매 소모 배분이다. T-011 지연 입고, T-012 배분 후 정정, T-018 전체 복원, T-019 호환/이동과 T-021 통합·격리 시연은 아직 미완료다. T-001 별도 깨끗한 Windows/Mac·실제 절전, 배포 준비와 goal 최종 완료는 계속 열어 둔다.

## T-010 후속 통합 확인 · 2026-10-25

05의 남은 실제 배분 표시는 [T-010](T-010-verification.md)에서 확인했다. 실제 실사 전후 판매 상세/API의 품목별 미확정·배분·미배분과 전체 일부/전체 확정, null과 유효0, 당시 레시피 원가와 실사 배분 원가의 분리를 검증했다. 기존 판매 E2E7개를 포함한 기존57개와 새 실사/폐기8개가 통과했다. 이미 배분된 판매의 개수 정정은 현재 연결 실사의 영향 정정이 필요하다고 안내하며 T-012에서 원자 재배분으로 이어간다. 위 T-009 최초 검증 시점의 후속 경계는 이력으로 보존한다.
