# T-031~T-033 일괄 입력 검증

2026-10-25. 세 티켓을 [ADR-010](../decisions/ADR-010-unified-bulk-entry.md)의 공통 CSV/시트 편집표로 구현한다. [공통 계획의 Before4](../tickets/T-031-T-033-plan.md)는 세 티켓 각각에 적용한다. 아래 수용 기준은 실제 최종 후보의 관련 실행으로 확인했다. T-001/T-021의 외부 배포 조건은 별도다. 실제 볼트 연결 노트에도 범위·기술 선택·확인된 결과와 진행/미검증을 반영하고 재읽기·변경 역대조로 나머지 원문 보존을 확인했다.

## 구현과 수용 기준

| 기준 | 구현·검증할 경로 | 상태 |
| --- | --- | --- |
| T-031-01 | 재료·메뉴·입고 CSV 양식과 열/참조 도움말, Excel UTF-8 CSV 저장 안내 | PASS |
| T-031-02 | 업로드·참조/값 확인과 별도 등록 버튼, 확인 전 DB/JSON 무변경 | PASS |
| T-031-03 | 수량·단위·금액·입고일, 빈 값/0원, 실사 포함 선택·재가산/몫 재사용 거절 | PASS |
| T-031-04 | 기존 material/recipe/business writer의 등록·실제 상세 조회, 원요청/JSON·버전 보존 | PASS |
| T-031-05 | CSV 형식/상한·참조/단위 오류, 전체 사전검사·부분 성공 정지·재업로드 | PASS |
| T-032-01 | 파일 없는 행 추가/삭제·셀 수정·여러 행 붙여넣기 | PASS |
| T-032-02 | 오류 표시·최종 확인·등록/조회, 편집/취소의 확정 효과0 | PASS |
| T-032-03 | 공통 UUID·단위·0/빈 값·날짜·버전·JSON·재시도·실사 포함 계약 | PASS |
| T-032-04 | 닫힌 열과 참조 선택·중복/부분 성공 안내, Tab·Shift+Tab·6화면 | PASS |
| T-033-01 | CSV 결과를 같은 표에서 직접 수정 | PASS |
| T-033-02 | 편집 뒤 확인 무효화·재검증과 최종 수정본 실제 저장 | PASS |
| T-033-03 | 세 대상과 기존 단위/가격/날짜/포함·버전·JSON·원요청 계약 | PASS |
| T-033-04 | 편집·취소/확정 구별, 오류 수정·성공행 고정·재시도/중복 방지 | PASS |
| T-033-05 | `/bulk`의 같은 표·검증·실행기로 세 티켓 통합 | PASS |

## 변경 경계

- CSV/표: `frontend/app/utils/bulk-csv.ts`, `types/bulk.ts`, `components/BulkSheet.vue`.
- 검토·보관·순차 실행: `utils/bulk.ts`, `bulk-runner.ts`, `composables/useBulkEntry.ts`, `pages/bulk.vue`, `BulkReceiptPreview.vue`, 공통 탐색·보존 입력.
- 입고 무기입 확인: `src/receipt.rs`, `src/store/receipt.rs`, `src/server.rs`, `tests/receipt_http.rs`.
- 복원된 UUID 참조: 기존 `utils/recipe.ts`에서 entity 참조와 신규 요청 UUID 검증을 분리하며 회귀 시험을 연결했다.
- 설계·절차: ADR-010·요구사항·구조·도메인 계약·README·세 티켓/색인. DB 스키마 v15·JSON v2·기존 writer와 API별 본문 상한(일반16KiB·레시피256KiB)은 유지한다.

## 검증 범위와 한계

단위 시험은 파서·검증/보관·순차 실행을, HTTP는 읽기 전용 입고 확인의 정상/오류·세션/Origin/크기·DB/파일 불변을 확인한다. 실제 Chromium 시나리오는 [bulk.spec.ts](../../frontend/e2e/bulk.spec.ts)의 정상 UI·API·JSON·오류 수정·재시도·부분 성공·복원·외부 초안 경계를 사용한다. 코드 읽기나 작성된 시험을 실제 통과로 취급하지 않는다.

동일 내용 안내는 현재 탭의 보관 결과이며 서버의 영구 중복 원장이 아니다. 새 앱 주소에서는 실제 목록 확인이 필요하다. 전 배치를 단일 트랜잭션으로 되돌리지 않으며 이미 성공한 행은 기존 정정/전체 복원 계약을 따른다. xlsx·수식/서식 편집·운영 전체 자료 이관은 포함하지 않는다.

## 실제 실행 결과

환경은 macOS27.0.1 arm64, Rust/Cargo1.98.0, Node24.20.0·npm11.19.0, Playwright1.64.1·Chromium156.0.8078.4다. 최종 표준 `node scripts/build.mjs`는 종료0(77.81초), release SHA-256은 `fd360358df0f87eedf85a7c660082878cf6e6053cde4d77d01238824ab90d754`다. UI20경로 생성과 release의 홈/프로젝트 경로 검사1개를 통과했다.

| 실행 | 실제 결과 |
| --- | --- |
| `npm --prefix frontend test` | 164 PASS·실패0. 이전144 및 각 RED/GREEN 재실행과 합산하지 않음 |
| `npm --prefix frontend run typecheck` | 종료0 |
| `cargo test --locked --test receipt_http` | 최종 UI 갱신 후12 PASS·실패0. 신규 읽기 전용 경계4개 포함, 전체 Rust 회귀와 겹침 |
| `npm --prefix frontend run test:e2e -- bulk.spec.ts --output=../tmp/verification/bulk-browser-final/run-2-results --reporter=line` | 신규11 시나리오 PASS·실패0, 12.8초 |
| `cargo fmt --check` / `cargo clippy --locked --all-targets -- -D warnings` | 각각 종료0 |
| `node --test scripts/ui-manifest.test.mjs` / `node scripts/ui-manifest.mjs --check` | 시험1 PASS / 최종 산출물 정합성 종료0 |
| 1440×900·768×1024·390×844, 라이트/다크 | 스크린샷6개 직접 검토. 페이지 넘침0·표 안쪽 가로 스크롤·단계/버튼/문자 대비 확인 |
| 키보드·양식 | Tab/Shift+Tab 실제 초점 이동, 재료·메뉴·입고 양식 실제 다운로드와 가상 예제·헤더 확인 |

HTTP4개는 정상/미완성/0원·같은 기존 inspector의 효과 동등, canonical/닫힌 필드/오래된 세대·개정, 세션/Origin/16KiB/정확한 SPA, 지연 입고 수량 불변을 확인한다. 성공·실패·반복 미리보기에서 전체 DB/기록 파일 바이트를 대조하고, 일부 JSON을 지운 경우에도 미리보기가 이를 복구·재발행하지 않는 것을 검증했다.

브라우저11개는 CSV 최종 수정본과 HTML/수식 모양 문자열, 직접표/붙여넣기/행관리/취소/reload, 메뉴 그룹/여러 구성/0·없음 판매가, 입고 오류/0원/날짜/확인 전 쓰기0, 응답 유실/동일 요청/중복0, 실사 포함 편집/몫 재사용 거절, 같은 품목의 효과 변경/부분 성공/재업로드, 복원 중 늦은 성공, 복원 뒤 불명 요청 보존·빈 새 입력, 외부 수정/확정 초안, 성공행의 옛 초안 재적용 거절을 실제 UI·정상 API·JSON/조회로 확인한다. 시험의 자식 정리는 기본120초 자연종료의 증거로 사용하지 않는다. 자연종료·실제 이동은 T-019와 T-021 지정 시연 근거를 따른다.

관련 Rust 소스를 동결한 전체 회귀와 기존122 브라우저 회귀는 [T-021](../tickets/T-021-first-release-verification.md)에 별도 집계한다. UI의 마지막 변경은 표 위치 기준 CSS1개와 해당 브라우저 단언 보강이며, Rust4파일 해시는 그대로다. 이 문서의12·11·164 또는 반복 통합 단언을 전체 회귀와 합산하지 않는다.

## 최초 실패와 수정

- 이전 release의 입고 입력 미리보기는405였으며 기존 inspector를 연결한 읽기 전용 API와 HTTP 회귀를 추가했다. 최초 사설 실행기의 응답 관측 TypeError는 시험 코드 문제로 구분하고405 RED를 다시 확인했다.
- 단위 회귀로 새 재료의 미지정 종류, 복원 UUIDv8 참조, 외부 수정 초안의 head·명시 적용, 메모 공백/긴 잘못된 셀 보존, 성공행의 옛 초안 덮어쓰기를 재현한 뒤 수정했다. 타입 검사의 unknown/template narrowing 오류도 수정 후 종료0을 확인했다.
- 첫 실제 브라우저는9 PASS·2 FAIL이었다. 390px에서 숨김 라벨이 화면 기준으로 배치되어 넘친 제품 결함은 표 스크롤 영역에 `position: relative`를 주어 수정했다. 다른 실패는 API로 만든 참조를 시험에서 다시 불러오지 않아 포함 편집 버튼이 비활성화된 준비 누락이었다. 정상 UI의 참조 갱신을 넣었다. 수정 뒤 표준 빌드→최종11개 전체 PASS이며 최초9와 합산하지 않는다.
- 원실패 로그·trace·관측 화면·입력/요청·후속 결과는 Git 제외 시험 경로에 따로 보존했다. 공개 자료와 양식에는 가상 자료만 사용했다.

## 변경 후 RTD — 세 티켓 각각

- [Step 1] 목적 적합성 - PASS: 위14개 수용 기준을 공통 구현과 실제 실행에 대응했다.
- [Step 2] 오류·보안 - PASS: 크기/행/닫힌 CSV·DTO·참조·단위·세션/Origin·무기입 preview·원요청/복원·텍스트 표시와 공개 경계를 검토했다.
- [Step 3] 회귀 - PASS: 기존 writer·JSON/버전·정정/복원·단위/0·운영 규칙을 유지했다. 전체 배포 회귀의 잔여 상태는 T-021에 분리한다.
- [Step 4] 파일·함수 크기 - PASS: 파서·검증/보관·실행·화면 연결·표/효과 표시로 역할을 나누고 무관한 리팩터링을 추가하지 않았다.
- [Step 5] 재사용·통합 - PASS: 기존 catalog/recipe/business 검증과 입고 inspector·포함 편집기·정산 결과를 재사용한다.
- [Step 6] 부작용 - PASS: 새 스키마/의존성/외부 서비스/전역 설정 없음. 탭 저장공간 실패 시 전송 전 정지하며 부분 성공을 명시한다.
- [Step 7] 전체 재검토 - PASS: 독립 코드 검토와 실제 실패 재현을 대조하고 성공행·원요청 불변을 확인했다.
- [Step 8] 잔여물 - PASS: 임시 관측기·개인 자료·생성 산출물은 공개 후보에 넣지 않고 채택한 결정으로 오래된 미정 설명을 갱신했다.
- [Step 9] 품질 검증 - PASS: 위 실제 관련 빌드/단위/타입/HTTP/브라우저/정적·자산 검사와 문서 인코딩/링크/diff/제외 경계 검토를 수행했다.
- [Step 10] 사용자 흐름 - PASS: 세 입력 방식의 실제 오류 수정→확인→등록→조회와 키보드/6화면을 확인했고 README에서 절차·한계로 연결한다.
- [Step 11] 발견 원인 - PASS: 최초 실패·제품/시험 원인·최소 수정·후속 실행을 위에 구분했다.
- [Step 12] READY - PASS: T-031~T-033의 구현·관련 수용 검증 범위는 READY다. T-001/T-021·전체 배포 준비의 완료를 뜻하지 않는다.
- [Step 13] 커밋 준비 - PASS: 사용자 승인 범위의 공개35경로만 스테이징했고 기존 staged 변경은 없었다. 실제 status/unstaged/staged diff와 각 경로의 `git check-ignore --no-index -v`·공개 경계를 확인했다. T-021의 진행 문서/실행기와 모든 제외 자료는 이 커밋에서 분리한다.
