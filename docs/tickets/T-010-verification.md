# T-010 검증 기록

- 상태: 실사·폐기 구현·검증 READY — 02의 실제 반품은 T-013 대기, T-009-05 실제 표시 통합 완료
- 기준: [티켓](T-010-stocktake-and-waste.md), [계획과 RTD Before](T-010-plan.md)
- 기록일: 2026-10-25

## 변경 전 확인

저장·순수/HTTP·UI 담당이 공개 계획을 독립 검토했다. 빈칸/0·초기 기준/실제 관측·0가중치 후보 null·부분/전량 폐기·관측 뒤 생산·정산 후 판매 정정 경계·frozen9와 holding/판매 FK 교체에 필수 충돌이 없었다. DTO 명칭과 관측 stable/version·batch 링크를 맞추고 구현을 시작했다.

`cargo test --locked --test stocktake_http --test waste_http --no-fail-fast`는 정상 컴파일 뒤 실사1통과/5실패, 폐기1통과/4실패(exit101)였다. 미지원 API/SPA404·업무 kind400·실사 전용 alias405를 확인했고 기존 세션/Host/Origin/16KiB 보호2개는 통과했다. 이는 현재 미구현 행동을 확인한 결과이며 새 기능 통과가 아니다.

현재 release의 정상 API로 가상 schema9 변경17건을 캡처했다. 같은 원요청17건 재생의 원응답 바이트·state 불변과 JSON17건 일치, 판매1/개수 개정2·실제 holding FK를 가진 원가2행·전량 생산 뒤 원물0과 마지막 유효 단가를 확인했다.

`cargo test --locked --test stocktake_storage`는 fixture 설치/Store 열기와 정상 컴파일 뒤 schema 실제9/기대10으로1실패(exit101)였다. 뒤에 있는 migration 관계·새 nullable 단가·옛 행/JSON 보존 단언은 이 RED에서 아직 실행되지 않았다. 최종 구현 뒤 다시 검증한다.

이 확인 뒤 제품 구현을 시작했다. frontend 변경 후 다음 Cargo/브라우저 검증 전에 표준 `node scripts/build.mjs`로 산출물을 갱신한다. 실제 명령·결과·수용 기준별 검증과 RTD After13은 실행 근거가 생긴 뒤 추가한다.

T-009-05의 실제 실사 후 표시, T-008 생산 부족 입력→실제 관측 연결은 이번 티켓의 실제 통합 검증에 포함한다. T-011 지연 입고·T-012 배분/관측 정정·T-013 반품 writer와 T-021 격리 시연, T-001 별도 장비·실제 절전은 완료로 표시하지 않는다.

## 구현·첫 실행과 보완

UI의 실제 중첩 초안 복사/복원 검사는 최초1통과·3실패(exit1)였다. 시작 가치와 초과 확인이 얕은 복사로 원본 초안/재시도 본문까지 바꾸고 새 종류를 복원하지 못했다. 명시적 중첩 복사와 엄격한 새 종류 복원으로 보완한 뒤 해당4개 및 전체 UI59개가 통과했다. 타입 검사도 exit0이다.

첫 `node scripts/build.mjs`는 UI 생성·자산 정합성·Rust release·개인 경로 잔존 검사까지 exit0이었다. 전체 Rust 실행에서 기존 검사 세 곳의 오래된 기대값이 드러났다. catalog와 storage의 현재 스키마 기대9를10으로, recipe의 이전 보유 행 뒤 추가 참조 null 기대2개를5개로 맞췄다. 기존 열/행과 과거 fixture 원문은 그대로 비교하며 제품 코드를 이 실패 때문에 변경하지 않았다.

새 브라우저8개는 처음 실행에서 모두 통과했다(exit0, 약2.1분). 빈칸/0·시작 가치 확인·미완성 초안의 실제 정상 종료/재시작, 부분/전량 폐기, 실사 후 판매 배분·미배분, 실제 관측 뒤 생산 재시도, 오래된 미리보기, 응답 유실/JSON 복구와 큰 초안을 확인했다. 기존 브라우저57개도 exit0으로 통과했다(약6.8분). 이는 가상 자료의 격리 앱 검사이며 T-021 지정 시연 자료를 실행한 결과가 아니다.

키보드 Tab/Arrow/Enter/Space와 오류 포커스·명시0원/초과 확인은 실제 브라우저에서 통과했다. 화면 검토에서 공통 상세 스타일이 신규 결과 라벨을 좁히는 우선순위 충돌을 발견해 실사/폐기 결과 선택자 두 곳만 구체화했다. 그 뒤 UI59·타입 검사와 표준 빌드가 다시 통과했고, 최종 화면은 새 산출물로 다시 확인한다. 전체 Rust 완료·최종 화면·RTD 판단은 아래 최종 기록에서 확인한다.

`cargo fmt --all -- --check`와 `cargo clippy --locked --all-targets -- -D warnings`는 exit0이었다. 관측/폐기/판매 당시 참조·값 재계산, frozen9→10 경계, API 보호와 UI null/0 상태를 독립 검토했고 추가 차단 결함은 찾지 못했다.

판매 후보101건 검사는 실사 입력100행 제한과 판매 배분 후보 수를 구별하기 위해 정상 저장 경로로 실행한다. 누적 판매 초안/확정마다 모든 과거 전체 JSON을 재검증해 준비 비용이 커지는 점을 관측했다. 최종 종료 시간과 결과를 보존하며 누적 데이터 비용 검토는 T-021에서 이어간다.

## 수용 기준별 근거

| 기준 | 구현·실행 근거 |
| --- | --- |
| 01 선택/일괄 실사 | 순수 `blank_is_unchecked_but_zero_consumes_the_entire_known_value`, 저장 `mixed_batch_preserves_blank_rows_and_distinguishes_initial_zero_from_settlement`, HTTP 실제 묶음/잘못된 행, 정상 종료·재시작 E2E. 빈행은 원문으로 남고 확인한0만 관측 생성 |
| 02 확정 변동 기준 | 실제 입고→명시 폐기→실사→재실사, 실제 관측→생산 및 부족 입력 보존 통합. 판매 자체의 보유 불변·이전 효과 재가산 없음. 실제 반품 쓰기는 선행 순서상 T-013에서 추가 검증 |
| 03 판매 개수 배분 | 순수 비율/최소단위 잔여, 저장 직접0행·중복·비재귀·원본 구간/as-of 정정, 실제 A3/B1 배분 API/UI. 수량/가치 합계는 각각 보존 |
| 04 명시 폐기와 미배분 | 실제10kg→폐기1kg→실사6kg의 소모3kg, 판매 없음·가중치0 null 미배분·유효0 배분, 실사/판매 상세에서 근거와 당시 원가 분리 |
| 05 부분/전량 폐기 | 순수/HTTP/저장 부분V유지·전량손실V·0원·직원식·다음 입고 새 가치, 부족/미완성 입력 보존, 같은 요청 재생으로 중복 반영 없음 |
| 06 소모 원가 | 평균 K차감·전량 잔여 전부·Q=M=0·C0 실제 관측·가격0·반올림. 관측 후 폐기/생산이 같은 기준에서 이어지는 저장/HTTP 검사 |
| 07 초과/미확인 | 확인 checkbox·이유·기존 단가 또는 가격 없음의 명시 금액. 현재/마지막/사전 가격 출처, 미확인 행의 이전 관측 불변. 키보드·오류 포커스·오래된 미리보기 검증 |
| 08 첫 실사/중복 방지 | 알려진 입고/생산 기준의 첫 정산, 기준 없는 시작 가치/0 확인과 이전 판매 미배분, 실제 관측 뒤 생산 old preview409→새 preview→한 번 확정. 이전 소모와 생산 투입을 중복 처리하지 않음 |

T-009-05는 실제 실사 후 품목별 배분·미배분과 이전 미확정, 전체 일부/전체 확정 상태를 판매 목록/상세에서 확인했다. 당시 레시피 원가와 실사 배분 원가를 별도 영역으로 표시한다. 실사 후보로 연결된 판매의 개수 정정은0개·0소모·미배분도 T-012의 영향 정정이 필요하다고 안내한다. 이전 요청 성공의 재생은 이 차단보다 먼저 원응답을 돌려준다.

## 변경 파일과 경계

- `src/stocktake.rs`, `src/waste.rs`: 닫힌 입력/조회 DTO·정규화·순수 관측/가치·폐기/배분 계산.
- `src/store/stocktake.rs`, `src/store/waste.rs`: 실제 원본·관측/배분 버전·폐기, 공통 확정 원자성·조회·원계산/참조 검증.
- `src/business.rs`, `src/sale.rs`, `src/server.rs`, `src/lib.rs`, `src/store.rs`, `src/store/{business,holding,production,receipt,sale,snapshot}.rs`, `src/holding.rs`, snapshot의 frozen8/9: 업무 등록·API 보호·스키마10·실제 관측/폐기·판매 표시/정정 가드와 이전 형식 보존.
- `frontend/app/`: 실사/폐기 입력·가치 확인·미리보기·목록/상세, 판매 배분·보유 관측과 확정 변동, 중첩 초안/재시도 보존과 탐색·스타일.
- `tests/stocktake*.rs`, `tests/waste*.rs`, 기존 payload/holding/저장 검사, schema9 가상 fixture, frontend unit/E2E: 계산·연속 원장·장애·이전 원문·UI 회귀.
- README·ARCHITECTURE·TODO·티켓/계획/검증 기록: 현재 구현과 후속 경계. 연결 노트에도 같은 C-/D- 항목·날짜·스키마/기능 경계를 추가하고 기존 사용자 변경·메타데이터·분석 자료를 보존했다.

T-011 지연 입고의 수량/가치 보정, T-012 일반 정정·배분 영향, T-013 실제 반품과 T-021 지정 격리 시연은 후속이다. 별도 깨끗한 Windows/macOS·실제 절전은 미검증이며 T-001·T-021·배포 준비·goal 완료로 확대하지 않는다. 누적 원장 비용도 T-021 검토에 남긴다.

## 최종 실행 결과

Windows x64, Rust/Cargo1.99.0·Node24.19.0·npm12.2.1, Playwright1.64.1의 Chromium156.0.8078.4에서 실행했다. 최초 전체 Rust는321통과·3실패(exit101)였고 세 실패는 위의 기존 기대값이었다. 해당 검사만 보완하고 관련 target을 다시 통과해 전체324개 각각의 성공 근거를 확보했다. 최초 전체 명령이 exit0이었다고 표시하지 않으며, 제품 변경 없이 같은 장시간 검사를 반복하지 않았다.

| 명령·검사 | 실제 결과 |
| --- | --- |
| `npm --prefix frontend test` | 최종59 PASS·exit0 |
| `npm --prefix frontend run typecheck` | 최종 exit0 |
| `node scripts/build.mjs` | 최초 및 CSS 보완 후 최종 exit0. UI 생성·자산 기록·Rust release·개인 경로 잔존 검사 포함 |
| `cargo test --locked --no-fail-fast` | 321 PASS/3 FAIL·exit101. 실사 저장14/순수14/HTTP10, 폐기 순수7/HTTP7/저장6 및 frozen9 판독3은 통과 |
| `cargo test --locked --no-fail-fast --test catalog_storage --test recipe_storage --test waste_storage` | 기대값 보완 후8+13+6=27 PASS·exit0 |
| `cargo test --locked --test storage` | 마지막 기대값 보완 후13 PASS·exit0 |
| `cargo fmt --all -- --check` | 최종 exit0 |
| `cargo clippy --locked --all-targets -- -D warnings` | exit0 |
| `npm --prefix frontend run test:e2e -- local-app.spec.ts ui-foundation.spec.ts catalog.spec.ts business.spec.ts orders.spec.ts receipts.spec.ts recipes.spec.ts productions.spec.ts sales.spec.ts --fully-parallel --workers=6` | 기존57 PASS·exit0, 약6.8분 |
| `npm --prefix frontend run test:e2e -- stocktakes.spec.ts --fully-parallel --workers=3` | 신규8 PASS·exit0, 약2.1분 |
| `node --test scripts/ui-manifest.test.mjs` · `node scripts/ui-manifest.mjs --check` | 자체1 PASS 및 현재 입력/산출물 정합성 exit0 |
| 실제 화면·키보드 | 입력/미리보기6장·최종 상세6장·판매/폐기390dark, Tab/Arrow/Enter/Space·오류 포커스 PASS. CSS 수정 후 상세 grid도390=1열·1440/768=2열 확인 |

실사 저장14개는101건 후보의 정상 초안/확정 누적을 포함해629.75초에 끝났다. 중단하거나 통과로 가정하지 않았으며 테스트 본문·실행 모드도 바꾸지 않았다. 이 debug 묶음 시간을 개별 저장 지연이나 release 성능으로 단정하지 않고 [T-021](T-021-first-release-verification.md)에 후속 측정 근거로 연결했다.

## RTD After

[Step 1] 목적 적합성 - PASS: 여덟 수용 기준의 선택 실사·소모/배분·부분/전량 폐기·초과/초기 기준과 실제 생산/판매 표시를 실행 근거에 대응했다.

[Step 2] 오류·중요 문제·보안 - PASS: strict DTO/크기/UUID·정수 상한·같은 요청·전역 CAS·원자 저장, 세션/Host/Origin과 공개 가상 자료 경계를 유지했다. 실사 alias만256KiB이고 공통16KiB를 보존했다.

[Step 3] 회귀 - PASS: 이전 원요청/응답/JSON·FK·원가 참조와 기존57 브라우저를 확인했다. 판매 즉시 차감·실사 자동 폐기·과거 원가 재계산을 도입하지 않았다.

[Step 4] 파일·함수 크기 - PASS: 순수 계산·저장/검증·고정 이전 reader, UI 입력/가치/결과/배분을 역할별로 분리했다. 같은 티켓과 무관한 리팩터링을 추가하지 않았다.

[Step 5] 재사용·통합 - PASS: 공통 business/mutation·정수/배분·보유/가격 helper·UI 초안/재시도/테마를 사용했다. 실제 관측을 생산과 판매에 연결했다.

[Step 6] 부작용 - PASS: 영속 변경은 db/와 JSON에 한 번 기록한다. 전역 서비스·자동 동기화·배포를 추가하지 않았고 누적 저장 비용은 실제 근거와 후속 측정으로 남겼다.

[Step 7] 전체 변경 재검토 - PASS: 저장·순수/API·UI 담당 독립 검토와 root의 공유 연결/저장·대표 화면 검토, 신규 파일/fixture 직접 읽기를 수행했다.

[Step 8] 불필요한 잔여물 - PASS: 임시 로그·캡처·개발 자료는 제외 경로에 보존했다. 제품에 티켓/스키마 구현 용어를 노출하지 않고 후속 기능 상태는 문서에 구분했다.

[Step 9] 품질 검증 - PASS: Rust324개 각각의 통과·UI59·브라우저65·타입/fmt/clippy·빌드/자산과 UTF-8/LF·상대 링크·diff·후보 제외 규칙을 확인했다. 최초 실패와 재검증을 구분했다.

[Step 10] 사용자·개발자 흐름 - PASS: README·색인·티켓·계획/검증이 연결되고 실제 초안/재시작·0/미확인·확정·배분·충돌/재시도·생산 재검토·키보드/두 테마를 확인했다.

[Step 11] 발견 문제의 원인 - PASS: 중첩 얕은 복사, CSS 우선순위와 기존 스키마 기대값을 각각 구분해 최소 보완·관련 재검증했다. 누적 JSON 검사 비용은 숨기거나 자료 삭제로 우회하지 않았다.

[Step 12] READY 판단 - PASS: 현재 실사·폐기 구현과 T-009-05의 통합은 READY다. T-010-02의 실제 반품 연결은 미검증으로 열어 두고 T-013에서 마친다. 지연 입고·일반 정정·T-021 시연과 T-001 실환경도 후속이며 티켓 전체·배포 준비·goal 완료로 확대하지 않는다.

[Step 13] 요청된 커밋 준비 - PASS: 사용자의 명시적 범위 승인에 따라 공개 후보의 ignore·기존 staged 상태·실제 diff를 검토한다. 로컬 노트/설정·개인 자료·로그/화면은 포함하지 않고 push·PR·배포는 수행하지 않는다.

## T-013 후속 연결 · 2026-10-25

[T-013 검증](T-013-verification.md)에서 실제 반품 이후 다음 실사가 반품을 다시 소모로 배분하지 않음을 Store/HTTP/브라우저로 확인했다. 지연 입고 금액 보정 전후의 반품·다음 실사도 수량과 가치를 보존한다. T-010-02를 포함한 여덟 기준을 모두 완료했다. 앞선 과거의 T-013 대기 표기는 T-010 당시 상태이며 현재 판정은 이 후속 기록을 따른다. T-001 외부 환경과 T-021 전체 시연은 별도 미완료다.
