# T-017 검증 기록

2026-10-25. 상태: 완료 — [티켓](T-017-operations-overview.md)의 여섯 수용 기준과 [설계·Before4](T-017-plan.md)를 검증했다. T-016 커밋 `597fd40`에서 이어갔으며 기존 사용자 변경과 개인 자료를 보존한다. 스키마 v14·JSON v2·writer·운영 정책은 유지한다.

## 최초 실패와 수정 근거

보존 T-016 실행파일의 정상 세션·가상 품목201 뒤 새 summary/list API와 목록 화면의 정규 요청3개가 기대200/실제404로 실패했다. 기존 상세·작업 시작·HEAD/Origin·401/403/404 보호는6개 통과였다. 이6통과/3실패·종료1은 미구현 행동의 근거다. 기존 실제 Vue 렌더에서도 보관 포함26개/반환25개를 별도 필터 목록2개로 잘못 표시하는 실패를 확인했다.

UI URL의 큰 offset 반올림 위험과 목록 내부 이동의 남은 handoff는 각각 실제1실패 뒤 수정했다. UI는 안전 정수를 넘는 주소를 명시 거절하며 API의 checked usize/빈 끝페이지 계약은 유지한다. 메인→목록 전달만 남겨 새 진입에 옛 조회 기준을 다시 쓰지 않는다.

관련 Rust45개 첫 실행은42통과/3실패·종료101이었다. HTTP 보조 함수가 Windows의 잠긴 루트 writer lock을 읽어 실패했다. 정확한 잠금 파일만 바이트 비교에서 제외하고 DB·JSON·나머지 파일 검사를 유지한 뒤 HTTP7개를 통과했다. 첫 Clippy는 두 큰 enum 변형 때문에 종료101이었다. 큰 두 필드만 Box로 옮겨 JSON 형식과 값을 유지했고 개요 순수6·Store9·HTTP7의22개 영향 시험과 최종 Clippy를 통과했다. 서로 다른45개 시험의 완료 근거이며 한 번의45개 통과로 표현하지 않는다.

신규 브라우저 첫 실행은6통과/1실패·2.3분이었다. E6의 마지막 단언이 기존 `/api/materials` 배열 응답에 잘못 `.items`를 붙였다. 실제 pending·UUID·본문 보존과 재시도 성공은 통과했으며 시험 접근만 수정했다.

별도 키보드 시연의 첫 실패는 겹치는 관측 선택자였으나, 범위를 좁힌 뒤 실제 제품 초점 경합을 발견했다. 창 focus에 따른 기존 연결 재시도→동일 목록 GET마다 제목으로 초점이 돌아와 Tab으로 복귀 링크에 도달하지 못했다. 실제 focus/GET 로그와 SFC lifecycle VM의 실패를 보존했다. 같은 scope·offset 자동 재조회는 초점을 보존하고 최초·실제 페이지/품목 이동·명시 재조회만 제목으로 이동하도록 수정했다. 지속 회귀와 동일 실제 키보드 시연이 통과했다. 연결 보호 흐름은 유지한다.

## 최종 실행

Windows x64·Rust/Cargo1.99.0·Node24.19.0·Chromium156.0.8078.4에서 정상 Store/API와 격리한 가상 DB를 사용했다. frontend 변경 후 두 차례 모두 표준 빌드로 UI 산출물과 release를 갱신한 다음 Cargo·브라우저를 실행했다.

| 실제 명령·범위 | 결과 |
| --- | --- |
| `node scripts/build.mjs` | 최종18개 경로·release·실행파일 경로 검사1개 PASS, 종료0 |
| `cargo test --locked --no-fail-fast --test overview --test overview_storage --test overview_http --test statistics --test statistics_storage --test statistics_http` | 첫42 PASS/3 FAIL·무시0·종료101 |
| `cargo test --locked --test overview_http` | 잠금 보조 함수 수정 뒤7 PASS, 종료0 |
| `cargo test --locked --no-fail-fast --test overview --test overview_storage --test overview_http` | Box 수정 뒤22 PASS, 종료0 |
| `cargo test --locked --test overview_http` | 최종 UI/release 빌드 뒤7 PASS, 종료0 |
| `cargo fmt --check` | PASS, 종료0 |
| `cargo clippy --locked --all-targets -- -D warnings` | 최종 PASS, 종료0 |
| `node scripts/ui-manifest.mjs --check` | 최종 입력·생성 자산 정합성 PASS, 종료0 |
| frontend `node --experimental-strip-types --test tests/*.test.ts` | 최종133 PASS, 종료0 |
| frontend `node node_modules/@nuxt/cli/bin/nuxi.mjs typecheck` | 최종 종료0 |
| 기존 `local-app.spec.ts`, `ui-foundation.spec.ts`, `statistics.spec.ts` | 19 PASS·실패0·6.3분·종료0 |
| 신규 `overview.spec.ts` | 첫6 PASS/1 FAIL 뒤 영향 E6/E7의2 PASS·종료0 |
| 실제 키보드 | Tab·Enter·Space, 변동 펼치기·전체 목록·원입고·재조회·복귀 PASS; 읽기 전후 state 불변 |
| 실제 화면 | 1440/768/390px × 라이트/다크6장 직접 검토; 수정 뒤390px 목록/메인 초점선도 확인 |

Playwright는 frontend에서 `node node_modules/@playwright/test/cli.js test`에 해당 spec과 `--fully-parallel`을 사용했다. 기존19개는 workers3, 신규 첫7개와 영향2개는 workers2이며 영향 실행은 해당 두 시나리오의 grep 조건을 지정했다. 첫6건과 후속2건을 한 번의7개 통과로 표현하지 않는다. 기본120초 유예·정상 종료0 뒤 실제 재시작도 검증했다. 첫 화면6장은 배치가 같은 첫 release, 초점 수정 후 목록/메인과 실제 키보드는 최종 release 근거다.

공유 selector의 회귀는 이전 실행파일의 정상 API로 만든 입고·생산·반품·폐기·판매·실사·정정·보관 혼합 DB로 확인했다. 보존 DB의 격리 복사본에서 새 실행파일의 전체/판매 구간/빈 기간 통계와 연결/역참조5개 응답 전체가 같았다. GET 전후 state·DB/JSON 바이트와 정상 종료0을 확인했다. 캡처 보조 도구의 최초 잠금 파일 읽기 실패도 별도 보존했으며 정확한 writer lock만 제외한 새 DB 실행으로 검증했다. 이 비교는 공유 추출 후 첫 release에서 수행했고 이후 변경은 개요의 값 불변 Box와 목록 초점 처리다. 기존 통계23개도 통과했다.

## 수용 기준별 결과

| 기준 | 실제 확인 | 결과 |
| --- | --- | --- |
| T-017-01 | 미실측/Initial0/실사, 원확인 시각·현재 관측 버전, 정정·취소 시 앞선 활성 관측·복원; 예상 현재고 필드 없음 | PASS |
| T-017-02 | 관측10→입고15→원관측 정정30의 논리 순서, 같은 품목 생산 입출1행, 포함입고 R/I/U·dV·별도 보정, 반품·폐기; 미입고량 미합산 | PASS |
| T-017-03 | 입고 취소→주문 재개→새 유효 종료 차이, NoSales/ZeroWeight/명시0/Initial/미확정 판매·품목 쌍, DB/local/pending 구분 | PASS |
| T-017-04 | 열 개 범위·정확 건수→전체 목록→실제 stable 원기록/업무/정정 상세, 기존 판매·실사·주문·입고 시작 링크 | PASS |
| T-017-05 | 가짜 현재고·위험도·내일 메뉴·시설 자료 및 새 writer 없음 | PASS |
| T-017-06 | 26품목·21초안·21변동의 마지막 페이지·끝·scope/state409, 실패/끊김/늦은 응답·과거 결과 보존, 긴 이름·좁은 화면·키보드 | PASS |

strict query·미지/중복/누락·정규 숫자/UUID·4096바이트·세션/Host/Origin·GET/HEAD·정확 SPA/404와 읽기 무변경을 확인했다. 응답은 품목25개·품목별 변동20개·나머지 구역별20개로 제한하지만 정확 건수를 위한 전체 이력 읽기 시간은 일정하다고 보장하지 않는다. 실제 누적 release 비용은 T-021에서 측정한다.

## 변경 파일과 영향

`src/overview.rs`, `src/store/overview.rs`와 operations는 DTO·현재 관측/변동·상태 목록·paging을 담당한다. `src/store/effective_read.rs`는 기존 통계의 원참조·유효 관측/배분·입고 선택을 공유하며 statistics/totals는 기간 표시·합계를 유지한다. lib/store 모듈과 server에는 읽기 경로만 추가했다. 신규 Rust시험3개 파일로 순수6·Store9·HTTP7을 검증한다.

기존 overview.vue를 overview/index.vue로 옮기고 list.vue, 공통 표시 컴포넌트4개·읽기 composable2개·타입/helper·기존 CSS를 추가했다. UI 단위2개 파일·브라우저1개 spec과 기존 UI 기반의 보관 포함 문구 단언을 갱신했다. README·구조·계획/티켓/검증·색인/TODO도 맞췄다. 스키마·writer·원JSON·운영 자료를 바꾸지 않았다.

## RTD After13

[Step 1] 목적 적합성 - PASS: 여섯 기준을 실제 관측·논리 후속 변동·상태 분리·근거 이동·전체 목록·실패/키보드 결과에 대응했다.

[Step 2] 오류·보안 - PASS: strict query·기존 세션/Host/Origin·동일 state/scope·일반 내부 오류 안내·제한된 응답을 확인했다. 새 외부 통신·파일 쓰기·개인 자료 노출이 없다.

[Step 3] 회귀 - PASS: 관련45개를 최초42/3 뒤 HTTP7 재검증으로 완료했다. Box 뒤22개·최종HTTP7, 통계5개 전체 JSON 동일성, UI133·기존 브라우저19와 입력 보존을 확인했다. 사용자8경로는 원문 해시로 보존한다.

[Step 4] 파일·함수 크기 - PASS: 조회 DTO·선택·상태 조립·업무 변동·작은 화면 컴포넌트로 역할을 나눴다. 새 범용 엔진이나 무관한 리팩터링은 추가하지 않았다.

[Step 5] 재사용·통합 - PASS: 기존 유효 이력·배분·원참조·디자인 토큰을 공유하고 원가/재고를 다시 계산하는 별도 원장을 만들지 않았다. 후보 판매의 기존 ID 색인도 유지했다.

[Step 6] 부작용 - PASS: GET 전후 state·DB/JSON·파일 바이트를 유지했다. 조회가 pending·초안·보유값을 변경하지 않는다. 잠금 파일의 접근 불가와 자료 불변 검사를 구분한다.

[Step 7] 전체 재검토 - PASS: 저장/API/화면의 교차 검토와 최종 보완 독립 읽기를 수행했다. 원논리 위치·현재 배분·유효 종료·scope/offset·초점 경계를 대조했다.

[Step 8] 불필요한 잔여물 - PASS: 공개 시험은 가상 자료다. 원시 로그·trace·격리 DB·캡처와 시연 원자료는 제외 경로에 보존한다.

[Step 9] 품질 검증 - PASS: 표준 최종 build·영향 Cargo·UI/타입·fmt/Clippy·manifest·실제 브라우저·키보드·화면을 확인했다. UTF-8/LF·링크·diff·ignore와 사용자 변경 보존을 검사한다.

[Step 10] 사용자·개발자 흐름 - PASS: 정확한 목록 범위·오류 뒤 이전 결과·명시 새 조회·원상세·정상 재시작·모바일/테마와 키보드를 확인했다. 공개 문서와 실제 명령을 연결했다.

[Step 11] 발견 원인 - PASS: 큰 offset·남은 handoff·자동 GET 초점 경합은 실제 실패와 지속 회귀로 수정했다. lock/배열 응답은 시험 보조 코드만 수정했고 두 큰 enum은 값 불변 Box로 보완했다. 보안·운영 규칙을 완화하지 않았다.

[Step 12] READY 판단 - READY: T-017 운영 개요 범위는 완료다. 전체 복원·실제 환경·지정 시연·배포 준비·goal의 완료를 뜻하지 않는다.

[Step 13] 커밋 준비 - PASS: 명시 승인된 공개 후보의 status·unstaged/staged diff·ignore를 확인한다. TODO/색인은 본 작업 hunk만 포함하며 기존 사용자 변경·로컬 자료는 제외한다. push·PR·배포는 하지 않는다.

## 남은 범위와 다음 작업

새 운영 정책·데이터 구조 변경이 없는 읽기 기능이므로 이번 티켓에서 Obsidian에 새로 동기화할 결정은 없다. 앞선 동기화와 사용자 변경을 보존하며 추가 사용자 결정은 필요하지 않다.

T-018 전체 복원→T-019 호환→T-021 통합 검증과 지정 실제 시연을 계속한다. T-001의 별도 깨끗한 Windows/Mac·실제 절전 등 필수 실환경 검증은 미완료이며 배포 준비와 goal을 완료 처리하지 않는다.
