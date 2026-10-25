# T-018 검증 기록

2026-10-25. [설계·Before4](T-018-plan.md)의 여섯 수용 기준과 RTD After13을 완료했다. 아래 최초 실행·실패 보완·후속 실행을 구분하며 마지막 요약을 최종 결과로 삼는다. 사전 구조 실험과 자료 검증을 앱 복원 성공으로 대신하지 않는다.

## 사전 근거

- 현재 Windows x64 환경에서 정상 종료한 가상 스키마14 DB를 읽기 전용으로 읽어 메모리 DB에 복제했다. Python3.12.14·SQLite3.53.1에서 전체58테이블 중26테이블의30개 outbox FK를 순번 관계로 옮겼다. 원행·40개 원문과 published 값·FK 무결성·다음 순번·rollback 전후 구조/행·원본 DB 해시 보존을 확인했다. 종료0이다.
- 위 실험은 Rust 번들 SQLite의 실제 마이그레이션이나 전체 importer·파일 발행·브라우저 검증이 아니다. 해당 앱 검증은 구현 뒤 별도로 수행했다.
- 기존 공개 변경을 보존한 채 T-017 실행 파일과 사용자 기존8경로를 별도로 보존했다. 공개 시험에는 가상 자료만 사용한다.

## 구현과 첫 검증

- 가상 schema14 기준 자료는 기존 release의 정상 API로 생성했다. 변경 기록50개·원응답 재시도100개·사진2개를 보존했고 정상 종료2회, FK 오류0개와 무결성 검사를 확인했다. 공개 fixture는 해당 가상 자료이며 개인 시연 자료를 사용하지 않았다.
- 기존 release에 대한 HTTP RED는 5개 통과·5개 실패(종료1)로 신규 복원 경로가 없음을 확인했다. 마이그레이션 RED도 기대 schema15와 실제14의 차이로 종료1이었다.
- UI의 늦은 이전 세대 응답과 최초 상태 조회를 각각 실패 시험으로 확인한 뒤 수정했다. 첫 통합 UI 시험의 VM import 누락도 수정했다. 복원 중 조회의 `stale_read`를 재시도로 분류하고, 처리된 원요청 보관 뒤 사용자가 명시적으로 빈 새 재료 입력을 시작하도록 보완했다. 저장공간 실패 시 입력 유지·새 입력 준비 중 POST0·다음 명시 저장의 새 요청을 확인했다. 최종 UI 단위144개와 타입 검사는 종료0이다.
- `node scripts/build.mjs` 첫 실행은 UI 생성 뒤 Rust 컴파일 오류5개로 종료101이었다. 모듈 공개 범위와 선택적 이전 기록 접근을 수정한 두 번째 실행은19경로 생성·release 빌드·실행파일 경로 노출 검사1개를 통과해 종료0이었다. 미사용 메서드와 후속 UI/검증 보완 뒤 세 번째 표준 빌드도19경로·release·산출물 검사1개를 통과했다.
- 독립 읽기에서 구형 등록 UUID와 다른 업무 요청의 충돌 분류, 복원 JSON의 알 수 없는 중첩 필드, 새 형식 헤더의 실제 원요청 연결을 보완하고 회귀를 추가했다. 정적 발견 후 수정한 항목이며 실행 전 RED로 표현하지 않는다.
- `npm --prefix frontend run typecheck`, `node --test scripts/ui-manifest.test.mjs`(1개), `node scripts/ui-manifest.mjs --check`는 세 번째 빌드 뒤 각각 종료0이었다. 이어서 새 Rust decoder/저장/API/파일 시험을 실행했다.
- importer의57개 활성 테이블에 대해 실제 schema14의 열과 대응을 대조했다. 이는 정적 대응 검사이며 실제 전체 상태 복원 검증을 대신하지 않는다.
- ADR-009의 기술 결정을 연결 Obsidian 노트에 반영하고 재읽었다. 처음에는 구현·검증 진행 상태로 기록했고, 검증을 마친 뒤 T-018 완료와 남은 T-019/T-021를 반영했다. 해당 작업 문구 외의 기존 내용은 바이트 기준으로 보존했다.

## 첫 Rust 실행과 보완

복원 decoder10개는9 PASS/1 FAIL·종료101이었다. 구형 schema6 reader가 공유하는 보유 상태의 선택적 필드 두 개가 현재 형식에서는 null로 직렬화되어 정상 과거 원문까지 거절했다. 구형 복원 경로만 원문의 모든 키·값·배열 순서를 보존하면서 형식 변환에서 추가된 null을 허용하도록 수정했다. 알 수 없는 원문 필드는 값이 null이어도 거절하며 새 schema15는 완전한 형식 동일성을 유지한다. 같은10개 재실행은 모두 PASS·종료0이다.

사진 자산7개·복원 파일6개·HTTP7개는 첫 실행에서 모두 PASS였다. 저장소14개는10 PASS/4 FAIL·종료101이었다. 전체 schema14 수입의 판매 원가율 사유는 SQL NULL과 JSON 문자열 null을 구분해야 했고, 원 writer처럼 JSON 문자열로 보존하도록 고쳤다. 사전 검사에서는 최상위 savepoint의 RELEASE가 빈 트랜잭션을 commit하여 DB 파일의 SQLite 변경 카운터 두 바이트가 바뀌었다. 사전 검사가 연 최상위 트랜잭션은 전체 ROLLBACK으로 종료하고 외부 트랜잭션 안에서만 ROLLBACK TO/RELEASE를 사용하도록 수정했다. 전체 DB 행·파일 바이트 불변의 기존 단언을 유지한 재실행은 사진7·파일6·HTTP7·저장14 모두 PASS·종료0이다. decoder를 포함한44개가 통과했다.

첫 `cargo clippy --locked --all-targets -- -D warnings`는 불필요한 단위 표현식 하나로 종료101이었다. 값이 같은 빈 match 분기로 정리한 뒤 Clippy는 종료0이다. 그 직후 fmt에서 한 줄 배치를 지적해 정리했고 최종 `cargo fmt --check`는 종료0이다. 증분 캐시의 하드링크 실패 시 복사로 대체한다는 환경 경고는 기록에 남겼으며 제품 경고를 억제하는 설정을 추가하지 않았다.

## 전체 회귀에서 발견한 문제와 후속 확인

전체 Rust 회귀와 기존 브라우저115개는 네 번째 표준 빌드로 실행했다. 새 스키마 기대값이14로 남아 있는 기존 시험은 실제15/14 실패를 확인한 곳만 갱신하고 구형 자료의 미래 형식 거절 값은 유지했다. 회귀에서 내부 기록 검증 오류가 입력 오류로 분류되는 경계와, 현재 사진 참조가 손상됐는데 보존 등록만 보고200을 반환하는 경계를 발견했다. 각각 내부/수입 오류 분류와 현재·보존 descriptor 동일성 검사를 보완한 뒤 후속 빌드·영향 재검증을 수행했다.

새 복원 브라우저 첫7개는6 PASS/1 FAIL·종료1이었다. 정상 복원·여러 탭·응답 유실 후 기본120초 정상 종료/재시작과 새 입력이 통과했다. 실패는 Blob 원문을 Chromium CDP의 `postDataBuffer()`로 읽을 수 없던 시험 관측이었다. 별도 정상 UI 진단에서 가져오기201·원SHA/길이·서버 보관 원문 bytes·미리보기 성공을 확인했고 해당 단언으로 보완했다. 실제 Tab/Enter/Space·확정·완료 초점 이동은 통과했다. 최초6개 화면 검토 뒤390px 비교표의 복원 후 열이 내부 가로스크롤에 가려지는 점을 추가 발견해 두 값이 함께 보이도록 보완하고 기하 단언·화면·키보드를 재검증했다.

기존 브라우저 첫115개는111 PASS/4 FAIL·종료1·20.5분이었다.6000·6665·6668 포트의 시작 차단3개와 개요 목록 이동 직후 이전 메인 DOM을 읽는 단언 경합1개다. 포트 문제는 가상 HTTP200과 같은 Chromium의 `ERR_UNSAFE_PORT`로 별도 재현했고 [Fetch 규칙](https://fetch.spec.whatwg.org/#port-blocking)을 기준으로 앱의 loopback 재할당을 추가했다. 브라우저 보안 설정은 바꾸지 않는다. 개요는 실제 목록 요청200·최종 목록 제목을 trace로 확인한 뒤 제목 대기를 보완했다. 각 실행의 고정 실행파일 해시를 전후 비교했다.

다섯 번째 표준 빌드는19경로·release(5분50초)·산출물 검사1개를 통과했다. 후속 core 검증은 lib64개·bin3개 모두 PASS·종료0이며 포트의 재할당/해제와 오류 전파, 내부/수입 오류 분류를 포함한다. 사진 HTTP6개도 PASS·종료0으로 현재 자산 손상을 보존 등록이 숨기던 회귀를 확인했다. 기존115와 겹치는 시작/재시작·수정 영역14개는13 PASS/1 FAIL이었다. 이 재검증에서 처음 도달한 catalog의 보관 후 개요 단언은 T-017의 ‘전체 품목(보관 포함)’ 계약과 달리1을 기대하던 옛 시험으로 확인했다. 바로 앞 active1/archived1/all2 단언을 유지하며 개요 라벨과2를 확인하도록 보완했고 최종 재시작 흐름까지 통과했다.

새 복원 브라우저의 업로드와 여섯 화면 크기 시험2개 재실행은 모두 PASS·종료0이었다. 키보드 후속도 종료0이다. 추가540px 진단에서는 표 영역452px보다 표 최소 폭500px가 커서 오른쪽 열이 가려졌다.390px에만 적용했던 비교표 규칙을 중간 폭에도 적용한 뒤 표 영역과 내용 폭이 모두452px이고 모든 셀이 함께 보이는 것을 단언과 실제 화면으로 확인했다.

이후 정정 오류1개와 사진/복원 파일·HTTP·저장34개도 PASS·종료0으로 확인했다. 다섯 번째 빌드의 후속 Rust는 총108개이며, 최초 전체 실행과 겹치는 시험이므로 서로 다른 전체 시험 수에 더하지 않는다.

마지막 독립 읽기에서는 외부 원문의 기존 요청 UUID를 새 복원 요청에 사용하면 활성 요청 PK 충돌이 서버 오류로 분류되는 경계를 발견했다. 정상 Store에서 만든 원문을 정상 import한 실제 회귀는0 PASS/1 FAIL·종료101이었다. 이미 로컬에서 처리된 요청의 원응답 재생 순서를 유지하면서 선택 원문의 활성/보존/legacy UUID는 직전 상태 파일 발행 전에 충돌로 거절하도록 보완했다. 기존 DB bytes·전관계·state·원문 불변과 정상 새 UUID의 복원/재생/재시작 단언을 유지했다. imports 전체 파일명/bytes 불변은 수정 뒤 추가한 보강 단언이다. 최종 빌드 뒤 저장15개·HTTP7개가 모두 통과했으며 신규 충돌 시험도 포함한다.

전체 회귀·영향 재검증·최종 화면·RTD After13을 마쳤다. T-001의 별도 장비·실제 절전, T-021 지정 실자료 시연과 최종 완료는 별도 미검증으로 유지한다.

## 변경 파일과 수용 기준 연결

| 범위 | 주요 파일 | 수용 기준 |
| --- | --- | --- |
| 활성 상태와 보존 원장·스키마15 | `src/store.rs`, `src/store/restore.rs`, `src/store/restore/{schema,lifecycle}.rs`, 기존 catalog/mutation/profile writer | 01·02·05·06 |
| 구형 JSON 변환과 전체 역변환 | `src/store/snapshot.rs`, `snapshot/restore_decode.rs`, `snapshot/restore_import.rs`와 하위 도메인 모듈 | 01·02·04 |
| 원문·사진·HTTP 경계 | `src/restore.rs`, `src/restore_files.rs`, `src/photo_assets.rs`, `src/server.rs` | 03·04·05·06 |
| 복원 화면과 입력 보존 | `frontend/app/pages/restore.vue`, RestoreComparison/RestoredStateBanner/RetainedInputs, useRestore/current-state/restore-state와 기존 편집 composable | 01~06의 실제 사용자 흐름 |
| 회귀와 가상 이전 자료 | `tests/restore_{storage,http,files}.rs`, snapshot/restore_tests, photo 및 현재 schema 기대값 시험, schema14 가상 fixture·사진, UI restore 단위·E2E | 전체 기준·이전 writer 회귀 |
| 브라우저 포트 선택 | `src/browser_listener.rs`, `src/main.rs` | 통합 검증 중 발견한 시작/재시작 결함 |
| 공개 설명 | README, ARCHITECTURE, DOMAIN-CONTRACT, ADR-009, 티켓·색인·TODO | 구현/검증 상태·이동 및 복구 한계 |

사용자 기존 변경8경로는 해시와 작업 문구 역변환으로 보존을 확인한다. 혼재된 색인에서는 이번 티켓 문구만 별도 패치로 다루며 비공개 원자료·도구 로그·개인 절대경로는 공개 후보에 포함하지 않는다.

## 최종 검증 결과

Windows x64, Rust1.99/MSVC19.41, Node24.19.0, Playwright1.64.1·Chromium156.0.8078.4에서 가상 자료로 수행했다. 표의 영향 재검증은 앞선 시험과 겹친다.

| 실행 | 실제 결과 |
| --- | --- |
| `node scripts/build.mjs` 최종 |19경로 생성·release4분10초·산출물 경로 검사1개, 종료0 |
| `cargo test --locked --no-fail-fast` 최초 전체 |605 PASS/20 FAIL, 종료101. 현재 schema 기대18곳과 제품 오류 분류/사진 참조2곳 |
| 현재 schema 기대18곳 개별 `cargo test --locked --test <suite> <name> -- --exact` |18 PASS, 각 종료0. 구형 source의 미래 형식 거절값은 유지 |
| 수정 후 `cargo test --locked --no-fail-fast --lib --bins`, profile HTTP·정정 오류·사진/파일/복원 영향 |108 PASS, 각 종료0. 추가 core3개 포함 |
| 최종 `cargo test --locked --no-fail-fast --test restore_storage --test restore_http` |22 PASS, 종료0. 신규 source UUID 충돌1개 포함 |
| Rust 서로 다른 현재 시험 |629개 각각 PASS. 최초 전체와 위 후속을 시험 식별자로 대조했으며 한 번의 전체 종료0으로 표현하지 않음 |
| `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings` |각 종료0. 캐시 hardlink 대신 복사 환경 경고는 보존 |
| UI 단위·타입 |144 PASS·타입 종료0. 표준 npm typecheck도 실행했고 후기 CSS/시험 변경의 타입·구문을 재확인 |
| 기존 Chromium115 |최초111 PASS/4 FAIL → 겹치는 영향14의13 PASS/1 FAIL → 최종 catalog1 PASS. 마지막 시험은 기본120초 정상 종료·재시작까지 확인 |
| 신규 복원 Chromium7 |최초6 PASS/1 FAIL → 업로드/화면2 PASS →540px를 추가한 최종 화면1 PASS. 각7개를 검증했고 중복 합산하지 않음 |
| 실제 키보드·화면 |Tab/Enter/Space·확정·완료 초점·현재 자료 이동 종료0.1440/768/390px 라이트/다크6장과390px 키보드2장·540px 직접 검토 |
| `node --test scripts/ui-manifest.test.mjs`, `node scripts/ui-manifest.mjs --check` |1 PASS·검사 종료0 |

첫 전체의 장시간101건 누적 실사 시험도 끝까지 실행했다. 해당 stocktake_storage 묶음의 debug 실행은2446.14초였다. 같은 개발 PC의 T-016 기록598.06초보다 길었으며 이번 실행 중 다른 빌드/검증도 진행했으므로 엄밀한 성능 배율로 단정하지 않는다. 전체 원문·보존 근거를 반복 검사하는 누적 비용은 후속 release 측정에서 확인할 위험이다. 같은101 사례를 불필요하게 재실행하지 않았다. debug 누적 시간은 release 개별 저장 지연이나 배포 준비 판정을 대신하지 않으며 누적 자료 측정은 T-021에 남긴다.

## 수용 기준별 결과

| 기준 | 결과와 근거 |
| --- | --- |
|01|PASS: 정상 이전 구현의 가상 schema14 전체57활성 관계·버전·정정·배분·초안·사진 연결을 typed 전건 비교했다. v1 및 각 이전 JSON의 원 ID/원요청을 유지하고 선택 이후 활성 행은 제거했다.|
|02|PASS: 초기 revision0·마이그레이션 직후 capture와 A→B→A 재복원, 새 순번/세대·원문/사진 bytes·보존 원응답을 확인했다.|
|03|PASS: 선택/전역 사진의 누락·손상·경로·descriptor 검사, 연결 해제 후 과거 자산 조회와 JSON-only 누락 안내·차단을 확인했다.|
|04|PASS: 미래/중복키/미지필드/깨진참조·실제 junction·열린 핸들·상한/timeout·source UUID 충돌을 거절하고 DB/원문 불변과 rollback을 확인했다.|
|05|PASS: 이미 처리된/미처리 옛 요청·본문 충돌·여러 탭·늦은200·File pending·사라진 상세와 명시 새 입력을 확인했다. 자동 재적용은 없다.|
|06|PASS: commit 전/후 발행 오류·대기 취소·종료 후 완료·동일UUID 복구·기본 유예 정상 종료/재시작·재복원·새 기록을 확인했다.|

## RTD After

[Step 1] 목적 적합성 - PASS: 여섯 기준을 전체 상태/보존/사진/입력 거절/동시 요청/재시작 근거에 대응했다.

[Step 2] 오류·보안 - PASS: 닫힌 target와 DTO, 세션·Origin·원문 상한·no-follow·실제 hash·원요청 충돌과 정보 노출 경계를 읽고 실제 오류 주입으로 확인했다.

[Step 3] 회귀 - PASS: 이전 writer·마이그레이션·원문/원응답과 기존115 브라우저 흐름을 확인했다. 실제 실패와 수정 후 결과를 분리했다.

[Step 4] 크기 - PASS: DTO/파일/API/저장 lifecycle·스키마/도메인별 역변환/화면 책임을 나눴다. 큰 SQL fixture는 가상 과거 구현의 전체 관계 보존 근거이다.

[Step 5] 재사용·통합 - PASS: 기존 SQLite writer/recover·cap 파일 접근·변경 metadata·공통 입력 보존을 사용했다. 새 서비스·의존성을 추가하지 않았다.

[Step 6] 부작용 - PASS: 원문·사진·직전 상태를 무기한 보관하고 미등록 파일을 노출하지 않는다. 저장량 증가와 hard-link/Windows sync 한계는 명시하며 OS 설정을 바꾸지 않았다.

[Step 7] 전체 변경 - PASS: 저장/API·파일/UI의 독립 검토와 root 통합 재읽기, 실제 화면·문서와 코드를 대조했다.

[Step 8] 잔여물 - PASS: 임시 진단·실자료·원로그는 제외 경로에 보존한다. 공개 fixture는 가상 자료이며 자동 삭제나 디버그 우회가 없다.

[Step 9] 품질 검증 - PASS: 위 실제 빌드·시험·정적 검사와 파일/링크·UTF-8/LF·diff·제외 규칙 확인을 수행했다. 다른 OS 시험을 통과로 표시하지 않는다.

[Step 10] 사용자·개발자 흐름 - PASS: 선택/가져오기→검증→전체 영향→명시 확정→직전 지점/완료와 입력 보존을 확인했다. README와 티켓·ADR 연결을 맞췄다.

[Step 11] 발견 원인 - PASS: 구형 선택null·SQL NULL/JSON null·savepoint 빈commit·내부 오류 분류·사진 descriptor·source UUID·차단포트·SPA 대기·모바일 표의 원인을 확인하고 해당 경계를 재검증했다.

[Step 12] READY 판단 - READY: T-018 범위의 여섯 기준을 충족한다. T-019 종합 호환/이동, T-021 실제 시연·누적/의존성 검토, T-001 별도 장비·절전·배포 검증은 남아 있다.

[Step 13] 요청된 커밋 준비 - PASS: 명시 승인된 이번 변경만 대상으로 기존 사용자8경로·무시 파일을 분리한다. status/unstaged/staged·ignore와 실제 staged diff를 검토하여 한국어 커밋을 작성한다. push·PR·배포는 수행하지 않는다.

## 남은 작업과 한계

이번 결과는 개발 PC의 Windows/Chromium 근거이다. 깨끗한 별도 Windows/Mac·실제 절전·전원 차단·서명/공증·정식 배포와 OS 간 왕복 성공을 주장하지 않는다. 전체 DB 이동·수동 앱 교체 및 이전 자료 증거의 종합은 T-019에 남긴다. 지정 실제 자료의 격리 DB 시연과 전체 업무 통합·누적 측정·의존성 검토는 T-021에서 이어간다. 이들 미완료 상태 때문에 goal·배포 준비 완료로 처리하지 않는다.
