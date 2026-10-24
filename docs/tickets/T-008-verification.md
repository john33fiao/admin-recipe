# T-008 검증 기록

- 상태: 완료 — T-008 범위 READY, 후속 판매·실사·일반 정정과 실환경 검증은 별도
- 기준: [티켓](T-008-production.md), [설계와 RTD Before](T-008-plan.md)
- 기록일: 2026-10-25

## 변경 전 근거

T-004 완료 상태에서 새 기능의 행동 실패를 먼저 확인했다. 기존 기능의 정상 결과와 새 기능 미구현을 구분한다.

| 명령·대상 | 실제 결과 | 해석 |
| --- | --- | --- |
| `cargo test --locked --test production_http` | 컴파일 성공, 1통과·5실패, exit101 | 생산 조회/SPA 404, 생산 종류 미지원400, 전용 생성/수정 경로405. 기존 Host·Origin·세션·16KiB 제한 검사는 통과했다. |
| `cargo test --locked --test production_storage` | 컴파일 성공, schema 기대8/실제7로 1실패, exit101 | 이전 저장 형식의 정상 열기까지 확인했다. 이 RED 실행에서는 실패 뒤 마이그레이션 단언이 실행되지 않았다. |
| T-004 release 정상 API의 schema7 fixture | 가상 12개 변경과 원요청 재생 성공 | 품목3개, 레시피4버전/현재2개, 발주 연결 입고1개, 미완성 초안1개를 포함한다. 12개 요청의 원응답·revision·outbox/JSON 바이트가 일치한다. |
| `npm --prefix frontend test`의 생산 배열 회귀 | 기존37통과·신규2실패, exit1 | 행 복사 후 수정이 원본을 바꾸고, 별도 배열의 동일 내용을 다르다고 판단하는 동작을 재현했다. |

fixture 캡처 스크립트의 가격 출처 기대 문자열을 실제 계약인 `holding_average`로 고친 실행 실패는 기능 RED에 포함하지 않았다. 공개 fixture에는 가상 자료만 사용했다.

## 구현 중 실행

| 명령·대상 | 실제 결과 | 범위 |
| --- | --- | --- |
| `cargo test --locked --test allocation` | 6통과, exit0 | 최대잔여 배분의 전체 합, 0가중치·동률 순서, 작은 금액, 큰 정수와 잘못된 입력을 확인했다. 생산 전체 구현의 통과를 뜻하지 않는다. |
| `npm --prefix frontend test` | 43통과, exit0 | 배열 복사·구조 비교 수정 후 기존 및 생산 초안 회귀 검사. |
| `npm --prefix frontend run typecheck` | exit0 | 생산 입력·조회와 공통 업무/보유 타입 연결. |
| `node scripts/build.mjs` 첫 통합 | UI 생성·매니페스트 완료, Rust 컴파일 실패(exit101) | SQL 행 개수를 직접 `usize`로 읽은 두 곳의 타입 오류. 실행 검증 통과나 행동 RED로 집계하지 않는다. |
| `node scripts/build.mjs` 타입 수정 후 | exit0 | 행 개수를 `i64`로 읽고 검사 변환한 뒤 UI·매니페스트·release·개인 경로 잔존 검사를 통과했다. |
| `cargo test --locked --lib --test allocation --test holding --test production --test business_payload --test production_http` | 58통과, exit0 | 동결 reader·배분·보유·생산 계산·엄격 입력·정상 HTTP/재시도/누락 입고 뒤 새 근거를 확인했다. |
| 생산 metadata 변조 회귀 | 수정 전 5모드 수용으로 실패(exit101), 보완 후 동일 검사 통과(exit0) | 생산과 같은 순번의 품목 버전, 이관 외 순번 없음, 잘못된 시각·확정 UUID·0접두 순번을 거절하도록 검사했다. |
| `cargo test --locked --test production_storage` | 13통과, exit0 | 원자성 8단계, 원요청/JSON 재생·복구, 전량 사용/마지막 단가, 이전 DB 변환과 관계 변조14모드를 포함한다. |
| 최종 `node scripts/build.mjs` | exit0 | metadata 검사 강화와 브라우저 기대 문구 수정 후 생성·release·개인 경로 검사를 다시 통과했다. |

frontend 소스·설정·테스트 변경 뒤 정식 빌드로 산출물을 갱신하고 Cargo·브라우저 검증을 수행했다. 최초 전체 Cargo 실행에서 현재 schema 기대7과 schema6 보유 테이블의 고정 칼럼 수를 가정한 기존 테스트가 실패했다. 기대8 및 이전 칼럼의 동일성·새 생산 참조 두 칼럼의 null 보존을 검사하도록 수정했다. 제품의 보존 조건을 완화하지 않았다.

## 최종 실행

Windows x64, Node24.19.0·npm12.2.1·Rust/Cargo1.99.0, Playwright1.64.1·Chromium156.0.8078.4에서 가상 자료와 격리 DB로 실행했다.

| 명령·검사 | 결과 |
| --- | --- |
| `cargo test --locked --no-fail-fast` | 전체218통과, 실패0, exit0 |
| `cargo fmt --check` | PASS. 이전 기대값 수정 뒤 생긴 테스트 줄바꿈1곳을 rustfmt로 정리하고 재확인 |
| `cargo clippy --locked --all-targets -- -D warnings` | PASS, exit0 |
| `npm --prefix frontend test` / `npm --prefix frontend run typecheck` | 43통과 / PASS |
| `node scripts/build.mjs` | 최종 UI·manifest·release·실행파일 개인 경로 검사 PASS |
| `node --test scripts/ui-manifest.test.mjs` / `node scripts/ui-manifest.mjs --check` | 자체검사1통과 / 입력·산출물 해시 일치 |
| `npm --prefix frontend run test:e2e -- local-app.spec.ts ui-foundation.spec.ts catalog.spec.ts business.spec.ts orders.spec.ts receipts.spec.ts recipes.spec.ts --fully-parallel --workers=6` | 기존43통과, 6.4분, exit0 |
| `npm --prefix frontend run test:e2e -- productions.spec.ts --fully-parallel --workers=3` | 최초5통과·기대 문구2실패. 실제 천 단위 쉼표에 맞춰 테스트만 수정 |
| `npm --prefix frontend run test:e2e -- productions.spec.ts --grep "unfinished actual\|allocation defaults" --fully-parallel --workers=2` | 정식 재빌드 후 영향2통과, 2.1분, exit0. 신규7개 시나리오 모두 검증 |
| 1440/768/390 × light/dark, 실제 키보드 | 화면6종 직접 확인, Tab/Arrow/Enter/Space로 행 편집·오류/미리보기 focus·저장·확정·상세 이동 PASS |

기존 브라우저 회귀와 신규 최초 실행은 첫 통합 release에서 시작했다. 정상 UI를 바꾸지 않은 snapshot metadata 검사를 보완한 최종 release에서 영향2개를 재실행하고 전체 Rust를 확인했다. 키보드 보조 스크립트 최초 인증대기 누락401은 시험 준비 오류로 분리하고 인증 완료 대기 후 전체 흐름을 검증했다. 기존 실패 로그를 지우거나 처음부터 통과한 것으로 표시하지 않았다.

## 수용 기준별 결과

| 기준 | 상태 | 실행 근거와 후속 경계 |
| --- | --- | --- |
| T-008-01 실제 여러 투입·결과 | PASS | 순수12·저장13·HTTP9·신규UI7. 여러 행·중복 품목·생산 결과를 다시 투입·같은 품목 양쪽/순변동0을 확인 |
| T-008-02 실제량에 따른 원가 이월 | PASS | 가상3kg·30000→2kg/30000과15원/g, 투입 전량0의 마지막 단가 및 후속 원가 조회 |
| T-008-03 배분 총액·0원 결과 | PASS | 100:0·70:30·최소 단위 잔여·전체0원·중복행 한 번 계산, UI 초기 비율과 사용자 수정 보존 |
| T-008-04 한 확정·실패/재시도 | PASS | 쓰기8단계 rollback·원응답 재생·JSON 발행실패 복구·공통/전용 경로 동일 요청 재생 |
| T-008-05 알려진 확정 변동 | PASS(계약) | 실제 보유 변동을 입고/생산으로 구분하고 기준 효과를 재가산하지 않음. 실제 판매/실사 배분 통합은 T-009/T-010에서 추가 검증 |
| T-008-06 기록 충돌·근거 연결 | PASS | 부족 실제량 보존→정상 누락 입고→오래된 미리보기409→새 근거로 한 번 확정. 합성 관측/보정 출처·순번을 보존한 단일 적용도 확인 |

## 변경 파일과 문서 동기화

`production`의 순수/저장 모듈과 DTO·HTTP·공통 업무, `holding`의 생산 원천·마지막 단가, `domain/allocation`, schema8와 frozen6/7 reader를 추가·연결했다. 프런트엔드는 생산 편집/원문/미리보기/목록/상세와 공통 배열 초안 보존을 추가했다. 생산/보유/입력·HTTP·저장·UI 검사, schema7 가상 fixture와 현재 schema 기대값을 갱신했다. README·ARCHITECTURE·요구사항의 상태 안내·TODO·색인·본 티켓/계획/검증 기록을 실제 상태와 맞췄다.

연결 기획 노트에는 2026-10-25·C-01~C-03·D-04~D-05와 실제 생산/원가/보유·schema8 계약 및 후속 범위를 추가했다. 기존 노트 바이트를 보존하고 양쪽을 재읽었다. 운영 정책 추가·원자료 이관·자동 동기화 서비스 설치는 없다.

## RTD After

[Step 1] 목적 적합성 - PASS: 여섯 수용 기준을 위 실행 근거와 대조했다. 실제 판매/실사·정정 후속 경계를 유지한다.

[Step 2] 오류·중요 문제·보안 - PASS: 구조/수량/배분/참조·본문 상한·세션/Host/Origin과 원자성을 검사했다. 공개 후보는 가상 자료이며 개인 경로·운영자료·인증정보가 없다.

[Step 3] 회귀 - PASS: 전체Rust218·기존브라우저43, schema6/7 원문·원요청 재생, 이전 입고 외형과 레시피 원가를 확인했다.

[Step 4] 파일·함수 크기 - PASS: 순수 계산·저장·HTTP·화면을 기존 모듈 경계로 나눴다. 동결 DTO는 과거 형식 보호에 필요한 별도 타입이다.

[Step 5] 재사용·통합 - PASS: 공통 mutation/업무 초안·확정·CAS·요청 재생·가격 해석과 UI 상태를 사용했다. 범용 원장 프레임워크를 추가하지 않았다.

[Step 6] 부작용 - PASS: DB는 schema8로 전환하고 변경 전 백업·실패 보존을 검증했다. 전체 JSON의 저장량 증가는 T-021에서 측정한다. 외부 서비스·전역 설정 변경은 없다.

[Step 7] 전체 변경 재검토 - PASS: 소유 범위별 재읽기와 독립 검토를 수행했다. 현재 공개 파일·새 파일과 기록을 대조했다.

[Step 8] 불필요한 잔여물 - PASS: 일회성 로그·화면·시연 준비 자료는 제외 경로에 남겼다. 제품에 테스트 전용 우회 경로나 기대값 적재가 없다.

[Step 9] 품질 검증 - PASS: 실제 명령·종료코드를 위에 기록했다. UTF-8/LF·상대 링크·diff·ignore 및 실행 산출물 정합성을 확인했다.

[Step 10] 사용자·개발자 흐름 - PASS: 정상 재시작 후 초안 편집→미리보기→확정→상세/품목 보유 조회, 화면6종과 키보드를 확인했다. 공개 문서만으로 범위·실행·남은 작업을 찾을 수 있다.

[Step 11] 발견 문제의 원인 - PASS: 얕은 배열 복사/비교와 metadata 관계만 비교하던 검증을 실제 실패 재현 후 보완했다. 컴파일/기대값/시험 준비 오류는 제품 행동 실패와 구분했다.

[Step 12] READY 판단 - PASS: T-008 구현 범위 READY. 별도 장비·실제 절전·최종 시연과 후속 업무의 미검증은 유지한다.

[Step 13] 요청된 커밋 준비 - PASS: 사용자의 명시적 승인 범위로 공개 변경만 검토한다. 기존 staged 변경이 없으며 후보 ignore·공개 경계를 확인했다. 실제 staged diff와 경로 일치를 확인한 후 논리 단위로 커밋한다.

다음은 T-009 판매 수량/금액·당시 버전/가격 고정과 소모 미확정 기록이다. T-005의 남은 실제 판매 수용 조건도 그 단계에서 확인한다.

T-001의 별도 장비·실제 절전과 T-021의 실제 시연은 이 티켓의 로컬 검증으로 완료 처리하지 않는다. T-010 실제 실사와 T-012 정정 연결도 이 티켓에서 구현했다고 표시하지 않는다.
