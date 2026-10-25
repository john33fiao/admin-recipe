# T-019 호환·같은 OS 이동 검증

2026-10-25. 현재 스키마15·JSON v2의 기존 보존·복원 계약을 검증한다. 자동 업데이트·역방향 호환·OS 간 왕복은 범위에 포함하지 않는다.

## 변경 전 RTD

- [Step 1] 변경 계획 - PASS: 기존 호환/원본 보호 시험과 T-018의 검증된 근거를 재사용하고, release 실행파일의 전체 폴더 이동·원문/사진/정정/배분·보존 요청·재복원 결합 경계를 추가한다. `scripts/verify-compatibility.mjs`, 이 기록, T-019 티켓을 수정한다.
- [Step 2] 계획 검토 - PASS: 공개 가상 fixture와 격리 경로만 사용하며 기존 자료를 덮어쓰지 않는다. 이동할 원장은 정상 API의 사진 등록과 JSON 가져오기/복원으로 만들고 직접 DB 적재를 사용하지 않는다. DB-only 한계는 원본과 분리된 사본으로 검증한다.
- [Step 3] 검토 재검토 - PASS: 합성 schema1~3, 과거 실제 구현 API로 만든 가상 schema4~14, 정식 외부 배포를 구별한다. 같은 OS 경로 이동과 깨끗한 장비·절전·서명·OS 왕복을 구별한다. DB-only가 사진·수입 원문·직전 상태 파일까지 재구성한다는 가정은 하지 않는다.
- [Step 4] 계획 단순화 - PASS: 새 스키마/API/업데이트 프레임워크 없이 기존 Store·복원·사진 경로와 최소 통합 스크립트만 사용한다. 원시 결과는 Git 제외 `tmp/verification/compatibility/`에 남긴다.

## 이전 자료의 출처와 재사용 근거

fixture의 모든 업무·사진은 가상 자료다. 합성 fixture와 과거 실제 구현을 실행한 API 캡처는 구별하며 어느 쪽도 정식 외부 배포·운영 자료의 증거로 확대하지 않는다. JSON의 format 버전과 DB/schema 버전도 구별한다.

| 입력 | 출처 | 기존 실제 검증·현재 회귀 위치 |
| --- | --- | --- |
| DB1 | 출시되지 않은 합성 migration fixture | `tests/storage.rs`: 백업·자유 단위·실패 rollback |
| DB2 / JSON v1·schema2 | 합성 T-001 fixture | `tests/catalog_storage.rs`: 원 UUID·원응답·원문 보존 |
| DB3 / JSON v2·schema3 | 합성 catalog fixture | `tests/business_storage.rs`: 원열·원응답·JSON 보존 |
| DB4 / JSON v2·schema4 | 과거 가격 업무 정상 API의 가상 캡처5건 | `tests/purchase_order_storage.rs` |
| DB5 / JSON v2·schema5 | 과거 발주 정상 API의 가상 캡처11건 | `tests/receipt_storage.rs` |
| DB6 / JSON v2·schema6 | 과거 입고 정상 API의 가상 캡처18건 | `tests/recipe_storage.rs` |
| DB7 / JSON v2·schema7 | 과거 레시피 정상 API의 가상 캡처12건 | `tests/production_storage.rs` |
| DB8 / JSON v2·schema8 | 과거 생산 정상 API의 가상 캡처13건 | `tests/sale_storage.rs` |
| DB9 / JSON v2·schema9 | 과거 판매 정상 API의 가상 캡처17건 | `tests/stocktake_storage.rs` |
| DB10 / JSON v2·schema10 | 과거 실사 정상 API의 가상 캡처23건 | `tests/reconciliation_storage.rs` |
| DB11 / JSON v2·schema11 | 과거 지연 입고 정상 API의 가상 캡처27건 | `tests/correction_storage.rs` |
| DB12 / JSON v2·schema12 | 과거 일반 정정 정상 API의 가상 캡처34건 | `tests/purchase_return_storage.rs` |
| DB13 / JSON v2·schema13 | 과거 모아 입력 정상 API의 가상 캡처46건 | `tests/profile_compatibility.rs` |
| DB14 / JSON v2·schema14 | 과거 사진/운영 개요 정상 API의 가상 캡처50건·사진2장 | `tests/restore_storage.rs`: 전체 관계·원응답·원문·FK 및 미래 원문 거절 |
| 현재 DB15 / JSON v2·schema15 | T-018의 정상 저장·전체 복원 | 아래 실제 실행파일 이동·재복원 시나리오 |

각 공개 SQL fixture의 원 outbox와 고정 JSON을 보존한다. [T-018 검증 기록](../tickets/T-018-verification.md)의 최종 서로 다른 Rust629개·HTTP/저장·v1/schema3~14 decoder 결과를 재사용하고, 이번 Mac 회귀와 새 결합 시나리오는 별도로 기록한다. 과거 전체 회귀 수에 이번 시험을 더해 새로운 전체 시험 수로 표현하지 않는다.

## 실제 결합 시나리오

명령은 표준 의존성·빌드·Chromium 준비 후 저장소 루트의 `node scripts/verify-compatibility.mjs`다. 약6분이 필요하며 production의120초 종료 유예를 세 번 그대로 기다린다. 작업한 개발 장비에서 실행하며 깨끗한 장비·실제 절전·다운로드 실행/서명 검증을 대신하지 않는다.

1. 격리 폴더에서 현재 release 실행파일을 다른 CWD·빈 PATH로 시작한다. 정상 화면 접속·세션·heartbeat 뒤 정상 API로 가상 재료와 사진 두 장을 등록한다.
2. 공개 schema14 JSON을 정상 가져오기/미리보기/복원으로 적용하여 정정·배분·실사 포함 입고·반품·과거 사진·DB 초안의 관계를 재사용한다. 별도 후속 입력과 재복원으로 보존 요청·직전 상태 원문도 생성한다. 직접 DB 적재를 사용하지 않는다.
3. 보존 요청의 처리 사실/다른 본문 충돌과 업무 상세·보유·레시피·사진·복원 지점을 저장한다. 페이지를 빈 화면으로 이동해120초 뒤 종료 코드0·안전 종료 문구·signal 없음과 실제 시간을 확인한다.
4. 정지한 실행파일과 `db/` 전체를 한글·공백 경로로 실제 이동하고 같은 OS의 호환 실행파일을 수동 교체한다. 파일 해시와 재실행 뒤 API 전체값을 비교한다. 사진 원 bytes·JSON·imports 보존, 같은 복원 UUID 응답, 보존 요청 거절, 직전 상태로의 실제 재복원을 확인한다.
5. 정상 종료한 원본은 그대로 두고 DB 파일만 별도 사본에 복사한다. outbox의 원 JSON 재발행과 사진 누락으로 인한 시작 거절을 구별한다. 사진만 추가해 재실행하면 수입 JSON과 직전 상태 원문은 복구되지 않으며 해당 복원 미리보기가 명시적으로 실패해야 한다. 마지막 정상 종료 뒤 온전한 원본이 바뀌지 않았음을 확인한다.

## 실행 결과

최초 실행은 브라우저의 macOS Mach 포트 등록이 실행 샌드박스에 차단되어 앱을 시작하기 전에 실패했다. 브라우저 실행 권한이 허용된 환경에서 같은 격리 검증을 재실행해 종료0으로 통과했다. 이 최초 실패를 앱 오류나 통과로 계산하지 않는다. 제품 소스·스키마·복원 규칙은 변경하지 않았다.

환경은 macOS27.0.1·arm64(Darwin27.0.0), Node24.20.0, Playwright1.64.1, Chromium156.0.8078.4다. 표준 `npm --prefix frontend ci` → `node scripts/build.mjs` 성공 뒤 현재 Mac 실행파일로 검사했다. release SHA-256은 `a931caaefcf4227b1e38c3d56afc5397653466b7fc65264d14210d0da45e72a4`, schema14 JSON SHA-256은 `0f18b3b1f0d8061808cefa8b79f343ce362a164623a03f9fc322601c2917850e`다.

| 실제 검증 | 결과 |
| --- | --- |
| 전체 폴더 이동 전/후 자연 종료 | 각각120081ms·120012ms, 종료0·signal 없음·안전 종료 문구 |
| 다른 CWD·빈 PATH·한글/공백 경로 재실행 | PASS.59개 API의 전체값 동일: 원업무/정정7개·실사2개·보유·레시피·사진 연결·복원 지점 포함 |
| 이동 전 불변 파일 | 사진2장·원 JSON·imports를 합한11개 기존 파일의 SHA-256 동일. 기존 비활성 사진도 정상 조회 |
| 보존 요청·재복원 | 처리된 옛 UUID는 `already_processed`, 다른 본문은 충돌. 같은 복원 UUID는 원응답 재생. 직전 상태를 실제 재복원해 후속 품목을 회복하며 기존 원문 보존 |
| DB-only 사본 | outbox의7개 원 JSON 재발행. 사진 누락은 시작 거절. 사진만 추가해도 수입 JSON·직전 상태 원문은 재구성되지 않으며 미리보기503·상태 불변 |
| DB-only 사본의 마지막 자연 종료 |120077ms·종료0·signal 없음. 온전한 정지 원본의 전체 파일 SHA-256 불변 |
| 정지한 온전한 DB의 readonly 검사 | schema15, `integrity_check=ok`, FK 오류0, 검사 전/후 DB bytes 동일. 보존 요청57개·사진2개·imports4개·복원3개·소모행2개·정정7개·포함링크1개·업무22개 |

원시 결과·API 비교값·무결성 검사·사용한 격리 폴더는 Git 제외 경로의 실행별 기록에 보존했다. 성공 뒤 스크립트에 시작 거절의30초 대기 상한과 후보 바이너리 SHA 변경 거절을 보강했고 `node --check`를 다시 통과했다. 실제 실행이 사용한 후보 SHA와 보강 뒤 현재 후보 SHA도 동일하다. 안전장치 보강을 위해120초 종료 세 번을 불필요하게 반복하지 않았다.

현재 macOS release 실행파일의 별도 하위 프로세스 검증3개는 통과했다. 가상 미래 DB/schema999, 열이 잘못된 schema2의 변환 실패, 기존 outbox의 미래 JSON을 각각 격리 사본에 준비했다. 세 경우 모두 서버 주소를 공개하기 전 종료1로 거절했고 원 DB 전체 bytes와 보존 JSON 원문은 동일했다. 미래 DB에는 백업/변경 기록을 새로 만들지 않았다. 손상/미래 입력을 만드는 이 시험 준비는 정상 업무 자료의 적재 경로가 아니다. 기존 `tests/storage.rs`·`tests/catalog_storage.rs`의 원본 보호 시나리오를 실제 release 진입점에서 확인한 보강이며 기존 Rust 시험 수와 합산하지 않는다.

Windows↔macOS 왕복은 [T-027](../tickets/T-027-cross-os-data-transfer.md)에 유지한다. 같은 OS의 전체 폴더 이동 성공으로 해당 백로그나 [T-001](../tickets/T-001-portable-local-app.md)의 별도 장비·실제 절전·지원 환경·다운로드 실행/서명 조건을 완료 처리하지 않는다.

## 변경 후 RTD

- [Step 1] 목적 적합성 - PASS: T-019-01~06을 기존 스키마별 보존 근거·현재 release 원본 보호·전체 이동/재복원·README 절차·T-027 경계에 대응했다.
- [Step 2] 오류·중요 문제·보안 - PASS: 가상 공개 fixture·새 격리 경로·정상 세션/Origin/저장 API만 사용했다. 미래/손상 입력은 원본 보존을 실제 확인했다. 개인 자료·인증자료·개인 경로는 공개 파일에 넣지 않았다.
- [Step 3] 회귀 - PASS: 제품·스키마·기존 fixture·보존 계약은 그대로다. T-018의 실제 검증 근거를 재사용하고 부족한 현재 실행파일 결합 경계를 추가했다. 진행 중인 별도 전체 Rust 회귀를 완료로 세지 않는다.
- [Step 4] 파일·함수 크기 - PASS: 단일 목적 검증 스크립트와 결과 문서/티켓만 추가·변경했다. DTO·저장소의 무관한 리팩터링이 없다.
- [Step 5] 재사용·통합 - PASS: 기존 정상 API·공개 schema14 fixture/사진·Playwright·실제 종료 정책을 재사용했다. 새 앱 API나 프레임워크를 추가하지 않았다.
- [Step 6] 부작용 - PASS: 생성한 격리 자료와 실행별 로그를 제외 경로에 보존한다. 사용자 DB·전역 설정·서비스를 바꾸거나 기존 기록을 삭제하지 않았다.
- [Step 7] 전체 변경 재검토 - PASS: 독립 리뷰와 최종 새 파일/티켓 재읽기로 정상/DB-only·원본 보존 단언과 출처 구분을 확인했다. 예상 거절의 대기에 상한을 추가했다.
- [Step 8] 불필요한 잔여물 - PASS: 과거 finalize/stage 스크립트를 실행하지 않았고 이전 사전 메모를 완료 증거로 쓰지 않았다. 임시 로그·DB·API 원자료는 공개 후보 밖에 있다.
- [Step 9] 품질 검증 - PASS: 실제 release 결합 스크립트1회 성공, 실제 release 실패 보호3개, readonly 무결성/FK, 스크립트 구문·UTF-8/LF·상대 링크·diff·후보 ignore 검사를 확인했다. 최초 환경 실패와 후속 성공을 구별했다.
- [Step 10] 사용자·개발자 흐름 - PASS: 정상 화면 접속→API 자료/사진 등록→복원→자연 종료→수동 교체/이동→조회/재복원을 실제 실행했다. README의 정상 종료·전체 보존·실패 복구 절차와 연결한다.
- [Step 11] 발견 문제의 원인 - PASS: 최초 실패는 macOS 실행 샌드박스의 Chromium Mach 포트 제한이었다. 권한이 허용된 같은 검증으로 확인했고 제품 오류로 분류하지 않았다. 추가 제품 결함은 발견하지 않았다.
- [Step 12] READY 판단 - PASS: T-019 범위는 READY다. 별도 장비·절전·서명·OS 왕복·T-021 및 전체 goal 완료를 의미하지 않는다.
- [Step 13] 요청된 커밋·PR 준비 - PASS: 사용자가 승인한 이번 범위의 공개3개 경로만 후보로 준비한다. 현재 미스테이징이며 root가 실제 status·unstaged/staged diff·ignore를 다시 확인한 뒤 커밋한다. 제외 자료·PR·push·배포는 포함하지 않는다.
