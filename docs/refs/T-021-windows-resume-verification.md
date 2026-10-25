# T-021 Windows 재개 검증

2026-10-25. 기준은 `master`의 `4046c3620d33d760d8e53d4ecd865b4113b6ce8c`이며 공개 작업 트리와 staged가 깨끗한 상태에서 시작했다. 기존 Mac 결과와 이번 Windows 결과를 구분한다. T-019·T-031~T-033의 완료 상태를 유지한다.

## RTD Before

- [Step 1] 변경 계획 - PASS: 현재 Windows 개발 환경을 복구하고 새 격리 가상 DB에서 정상 UI/API·저장·조회·일괄 입력·오류 수정·부분 성공·원요청 재시도·복원 영향을 확인한다. 실제 실행 결과와 필수 외부 미검증을 기록한다.
- [Step 2] 계획 검토 - PASS: 기존 사용자 자료와 이전 증거를 보존한다. ZIP과 현재 파일을 대조하고 새 기준점을 사용한다. 직접 DB 적재와 기존 finalize/stage/sync 실행기를 사용하지 않는다.
- [Step 3] 검토 재검토 - PASS: 현재 개발 PC를 개발 도구 없는 별도 장비로 간주하지 않는다. 과거 Mac/Windows 통과 수를 복사·합산하지 않는다. 종료 단계가 섞인 실행기는 먼저 검사하고 필요한 기능만 분리한다.
- [Step 4] 계획 단순화 - PASS: 기존 잠금 파일·표준 빌드·정상 writer와 기존 시험을 사용한다. 새 기능·의존성 갱신·자동 배포·T-022~T-030·전체 운영 자료 이관을 추가하지 않는다.

## 범위와 판정 원칙

이번 앱 종료·재시작 검증은 사용자 요청으로 생략한다. 자동 종료·기본120초 유예·탭/브라우저 강제 종료 후 앱 종료·종료/재시작 반복은 실행하지 않는다. 시험용 프로세스 정리는 기능 시험의 자원 정리이며 종료 검증 통과로 세지 않는다.

T-001/T-021의 별도 깨끗한 Windows/Mac·실제 절전·지원 환경·다운로드 실행/서명은 필수 미검증으로 유지한다. Windows↔macOS 왕복은 T-027 백로그다. Mac Chrome의24개 표본·마지막1,440,121ms는 부분 근거이며 해당 실행을 재개하지 않는다.

## 환경과 이전본 확인

Windows x64, OS 빌드 `10.0.26300.9711`·표시 버전26H2, Rust/Cargo1.99.0(`x86_64-pc-windows-msvc`), Node24.19.0, npm11.19.0, Git2.47.0.windows.1이다. 레지스트리 ProductName은 Windows10 Home으로 보고되므로 지원 OS 이름·최소 버전을 새로 확정하지 않는다. Visual Studio2022 Community의 MSVC14.41.34120 설치 경로에서 `cl.exe`19.41.34123.0과 Windows SDK10.0.22621.0을 확인했다.

설치 Chrome156.0.8078.47·Edge156.0.4314.42와 Playwright1.64.1을 사용했다. 전역 Codex 설정·로그인·플러그인·브라우저를 복원하거나 변경하지 않았다. 실행 검증에는 로컬 Playwright와 raw CDP를 사용하며 네이티브 컴퓨터 제어 도구에 의존하지 않는다.

ZIP SHA-256은 전달된 `55c65446bce4bfb6701c5dfc7717984808de86637ffa5c0c074fa3d14e96bbcd`와 같았다. 내부 manifest의3,465개 파일 크기·SHA와 현재 공개 추적495개를 대조해 불일치0개를 확인했다. 현재 폴더에 빠진 최신 비공개 문서는 별도 참조 사본으로 읽고, 기존 원본을 덮어쓰지 않는 방식으로 보존했다. 빌드·의존성·trace의 의도된 제외를 제품 실패로 세지 않았다.

## 개발 환경 복구와 새 실행파일

현재 PATH에는 npm이 없어 공식 registry의 npm11.19.0을 작업 전용 Git 제외 경로에 받고 SHA-512 integrity를 검증했다. 전역 설치 대신 전용 명령 래퍼와 캐시를 사용했다. 패키지 배포본의 초기 래퍼는 설치 위치를 잘못 추론해 실패했고 전용 래퍼로 바로잡았다. 첫 의존성 설치는 sandbox의 디렉터리 생성 EPERM으로 실패했다. 허용된 실행 환경에서 같은 `npm --prefix frontend ci`를 다시 실행해 종료0을 확인했다. 제품·잠금 파일은 바꾸지 않았다.

지정 순서대로 의존성 설치 후 `node scripts/build.mjs`를 실행했다. UI20경로 생성·Windows release·실행파일 개인 경로 잔존 검사1개가 통과했고 종료0이었다. Rust release 단계는3분7초였다. Nuxt의 cache-driver 외부 의존 처리 안내와 Node의 shell 인자 경고는 로그에 보존했다. 빌드 성공을 무경고로 표현하지 않는다.

새 `target/release/sunchang-hansang.exe`의 SHA-256은 `af8c9dcc69d4a3eb2c91d756a2cf99a13d8831df8593bf47230e1dc2341514fc`다. PE x64이며 직접 DLL 의존은 shlwapi·bcryptprimitives·kernel32·ntdll·api-ms-win-core-synch·ws2_32 계열이다. Authenticode는 NotSigned다. 이 속성 확인은 개발 도구 없는 별도 장비·실제 다운로드/SmartScreen 실행 검증을 대신하지 않는다.

## 기능 검증

| 명령·대상 | 실제 결과와 범위 |
| --- | --- |
| `npm --prefix frontend test` | 165 PASS·실패0·생략0·종료0. 이번 Windows 실행의 UI 단위 결과 |
| `npm --prefix frontend run typecheck` | 종료0 |
| `node --test scripts/ui-manifest.test.mjs` / `node scripts/ui-manifest.mjs --check` | 각각1 PASS/종료0, 현재 입력·생성 자산 정합성 종료0 |
| 종료 없는 Rust API·복원·사진 선택 시험 | 고유65개를 각 실행에서 확인: business6·catalog5·correction_restore1·http8·receipt12·restore_files6·restore_http7·restore_storage15·profile_http5. 최초 실패와 후속을 아래에 구분 |
| `cargo fmt --check` / `cargo clippy --locked --all-targets -- -D warnings` | 모두 종료0. Clippy는 정적 검사이며 종료 시험을 실행하지 않는다. incremental cache hardlink의 복사 fallback 경고는 보존 |
| 정상 API 통합의 종료 이전 구간 | 설치 Chrome에서47 PASS·종료0. 원래48개 중 프로세스 재실행 상태 단언1개만 제외. confirm의 같은 요청 응답 비교도 유지 |
| `bulk.spec.ts`·`ui-foundation.spec.ts` | 설치 Chrome19개와 Edge19개가 각각 모두 PASS, 전체 실행 종료0·약1.7분. 가상 자료·새 시험 DB만 사용 |

기존 통합 실행기의 업무 본문과 요청 helper는 별도 Windows 사본에 그대로 유지하고 `App.stop`·재실행·정상 종료 단언을 제거했다. 사전 가격→발주/직접 입고→생산→판매→폐기→실사/배분→정정→현재 평균 반품→전체 복원, 원요청 재생·복원 후 중복 재적용0, 세션/Origin 거절을 확인했다. 실제 API 원문·DB·요청은 Git 제외 경로에 보존한다. 종료하지 않은 앱을 시험용 child 정리로 회수했으며 정상 종료 성공으로 세지 않는다.

브라우저 시험은 양식3종·CSV/직접표/붙여넣기·수정 후 확인·부분 성공·외부 초안 재검토·원 UUID/본문 재시도·복원 후 원요청 보존과 재적용 방지를 포함한다. 390/768/1440px·라이트/다크와 Tab/Shift+Tab을 확인했다. 대표 좁은 다크·넓은 라이트의 실제 캡처를 직접 읽었다. 이19개 범위는 전체 브라우저 회귀나 실제 절전·다운로드 실행을 의미하지 않는다.

### Rust 선택 시험의 최초 실패

종료 시험이 없는 API/복원 대상만 명시 선택했다. 최초 실행은 business6·catalog5·correction_restore1·http8 통과 뒤 receipt_http의8 PASS/4 FAIL로 종료101이었다. 네 실패는 미리보기의 파일 불변 검사 헬퍼가 Windows에서 의도적으로 배타 잠긴 `.writer.lock`의 바이트를 읽다가 OS 오류33으로 패닉한 것이다. 입고 API의 업무 기대값 불일치로 분류하지 않는다. 실제 상대 실패 경로를 추가한 동일 시험에서 이를 재현했다.

진단의 sandbox 실행 두 번은 그 전 단계의 JSON 원자 발행에서 OS 접근 거부로 멈췄고 원래 잠금 오류를 확인한 근거로 세지 않았다. 허용된 실행 환경에서 재현·수정·재검증 결과를 별도로 기록한다. DB/JSON·기타 업무 파일의 바이트 불변 검사는 유지한다.

수정은 `tests/receipt_http.rs`의 시험 헬퍼에 한정했다. 최상위 `.writer.lock`은 일반 파일 여부·존재·길이·수정 시각·읽기 전용 상태를 비교하고, 그 외 모든 업무 파일은 원래대로 전체 바이트를 비교한다. 디렉터리 항목도 비교해 빈 폴더의 추가·삭제를 탐지한다. 같은 이름의 하위 파일은 예외로 처리하지 않는다. 수정 뒤 `cargo test --locked --test receipt_http`는12 PASS·실패0·종료0(5.00초)이었다. 독립 diff 검토·포맷·Clippy 이후 `bff7618`로 커밋했다. 제품 잠금·API·권한·업무 기대값을 바꾸지 않았다.

최초 실행에서 도달하지 못한 `restore_files`6·`restore_http`7·`restore_storage`15는 별도 명시 선택 실행에서 모두 통과했다. `profile_http`는 종료 중 업로드 취소 시험1개를 명시적으로 제외하고5개 통과·1 filtered였다. 전체 Rust 실행이나631개 통과로 표현하지 않는다. 위 고유65개에 최초/후속 receipt의 겹치는8개나 진단 RED를 다시 더하지 않는다.

실제 선택 명령은 다음과 같다. 첫 명령의 최초 실패 뒤 복원 대상과 수정한 입고를 분리해 확인했으며, 최종 직접 입고 재검증도12 PASS·종료0(4.92초)이었다.

```sh
cargo test --locked --test http --test catalog_http --test business_http --test receipt_http --test restore_http --test restore_storage --test restore_files --test correction_restore
cargo test --locked --test restore_http --test restore_storage --test restore_files
cargo test --locked --test receipt_http
cargo test --locked --test profile_http -- --skip upload_admission_rejects_a_second_body_and_shutdown_cancels_only_unaccepted_upload
```

## 의존성과 자료 보존

현재 Windows의 `npm --prefix frontend audit --json`은 종료1, High7·Critical0이다. 기존 단일 node-forge 권고의 상위 전파 경로가 남는다. `nuxt4.6.2 → @nuxt/nitro-server4.6.2 → nitropack2.13.5 → listhen1.10.1 → node-forge1.4.0`을 설치 코드·잠금 파일과 대조했다. 자동 audit fix·다운그레이드를 적용하지 않았다. 새 Windows RustSec 감사를 수행했다고 주장하지 않는다. 자세한 사용 경로와 잔여 위험은 [보안 기록](T-021-security-review.md)을 따른다.

지정 시연90단언과 최종 보존 복사본 읽기10단언은 ZIP의 원결과와 대조했다. 이번 변경은 시험 헬퍼·검증 기록에 한정하며 지정 자료의 재고를 다시 입력하지 않았다. 기존 Mac의 Rust631·브라우저119+재검증3·새13·통합48·release 규모 측정은 그대로 보존하고 이번 Windows 통과 수에 합산하지 않는다.

실제 Windows Obsidian 볼트와 연결 노트를 읽어 현행 C-/D- 및 최신 완료/미검증 상태를 대조했다. 정책·요구·데이터 구조 변경이 없어 노트 수정이 필요한 새 결정은 없다. 볼트를 앱 저장소로 간주하거나 과거 Mac 경로를 사용하지 않았다. 미동기화 요구 결정은 없다.

## 실제 Chrome·Edge의30분 백그라운드 관측

공개 `scripts/verify-windows.mjs` 전체를 실행하지 않았다. raw CDP 관측의 별도 사본에서 초기 무접속 종료·정상 종료·브라우저 강제 종료 후 앱 종료·재시작을 제거했다. 새 한글/공백 경로의 앱·DB·브라우저 프로필만 만들고 다른 CWD에서 현재 exe를 실행했다. CDP browser PID와 직접 생성한 PID가 같은지 확인했다. OS 기본 브라우저를 바꾸지 않았으며 `--no-browser`를 실제 자동 열기 실패로 해석하지 않는다.

실제 앱 창을 최소화하고 `document.visibilityState=hidden`·창 상태·앱/브라우저 생존·wall-clock 간격을 매분 검사했다. 첫0분 표본과30분까지의 표본으로 브라우저마다31개이며 시간·visibility·focus·백그라운드 제한을 가상으로 바꾸지 않았다. 초기 빈 창은 별도로 남아 있으므로 모든 브라우저 창의 최소화로 표현하지 않는다. 두 관측은 병렬로 수행하고 복귀·저장·정리는 브라우저별로 직렬 수행했다.

| 실제 브라우저 |30분 관측 | 복귀 결과 |
| --- | --- | --- |
| Chrome156.0.8078.47 |31표본 모두 hidden·minimized·alive, 1,800,014ms, 최대 wall 간격60,011ms | 기존 ID/단위 조회·새 정상 API 등록/조회·상세 화면 모두 PASS. 실제 두 화면도 직접 확인 |
| Edge156.0.4314.42 |31표본 모두 hidden·minimized·alive, 1,800,005ms, 최대 wall 간격60,012ms | 창 복귀 뒤 `document.visibilityState === 'visible'` 대기가30초 만료. 이후 조회/새 저장·화면은 미실행. 복귀 통과로 세지 않음 |

전체 관측 실행은 종료1이며 Chrome의 관측/복귀 통과와 Edge의 복귀 단언 실패를 분리한다. Edge의30분 생존 근거를 없애거나 복귀까지 통과한 것으로 확대하지 않는다. 원인 확인 전에는 제품 결함이나 사용자 행위로 단정하지 않는다. 새 Windows의 결과를 Mac 부분 관측과 합산하지 않는다.

생성한 앱을 먼저 즉시 강제 정리해 child 종료를 확인한 뒤 전용 브라우저를 정리했다. 양쪽 cleanupComplete는 true다. 이 SIGKILL 자원 정리를 정상 종료·120초 유예 통과로 세지 않는다. 실행 전후 exe·관측 사본 SHA는 동일하다. 원결과·각 매분 표본·복귀 화면은 Git 제외 경로에 보존한다.

새 격리 DB·Edge 프로필의 짧은 진단에서는 같은 창 복귀 명령 직후1개 표본만 visible이고 이후20개는 hidden이었다. 창은 normal·앱/브라우저 생존·focus=true로 관측됐으며 기존 API 조회·새 정상 API 저장/조회와 DOM·캡처는 확인했다. 캡처도 직접 읽었지만 이를 안정적인 visible 복귀 성공으로 해석하지 않는다. 첫 짧은 진단은 조건상 `Page.bringToFront`를 실행하지 않았으며, 원30분 실행의 실패 원인을 확정하거나 해결한 근거가 아니다. 원실행·사본·결과 해시는 보존했고 두 실행을 하나의30분/복귀 통과로 합치지 않는다.

마지막 짧은 비교에서는 원복귀 명령 뒤10초20개 표본의 visible0개, 실제 `Page.bringToFront` 뒤10초21개 표본도 visible0개였다. 최신 상태는 hidden·창 normal이었다. API 조회·새 저장과 DOM·캡처는 확인했고 정리도 완료했지만 전면 전환 명령이 문제를 해결한다는 가설은 지지되지 않았다. 전용 프로필에서도 브라우저 내부 대화상자와 확장 프로그램의 새 target을 관측했으나 원인으로 단정하지 않는다. visibility/focus를 가상 설정하거나 제품·OS·브라우저 설정을 바꾸지 않았고, 추가30분 실행은 하지 않았다. Edge의 안정적인 실제 창 복귀는 미검증으로 남긴다.

### 추가 가시성 기준점 확인

새 전용 프로필의 짧은 추가 실행에서는 확장 환영 탭 후보가0개였고 닫은 환영 탭도0개였다. 원복귀 명령은21표본 중 visible1개, 전면 요청과 앱 재선택 뒤에는 각각21표본 중 visible0개였다. 따라서 환영 탭이 없어도 hidden이 관측되며, 환영 탭 닫기의 개선 효과를 검증한 것으로 표현하지 않는다. 앱 target ID/세션 오류도 기존 코드·시계열에서 확인되지 않았다. 정상 API·DOM·캡처와 이전 증거 해시 보존·자원 정리는 확인했다.

이어 최소화 자체를 실행하지 않는 새 프로필에서 초기 앱·정확한 앱 활성화·앱 재선택·별도 초기 빈 창1개 닫기 이후를 비교했다. 각 단계의21표본 모두 hidden으로, 연속2초 visible 기준점을 확보하지 못했다. 환영 탭은 없었고 로그인·동기화 대화상자는 조작하지 않았다. 기존 조회·새 정상 API 저장/조회와 DOM은 확인했지만 마지막 `Page.captureScreenshot`은15초 시간 초과로 실패해 진단은 종료1이었다. 정리는 완료했고 원30분/이전 진단의 소스·결과·화면 해시는 같았다.

이는 가시성 기준점 부재와 복귀 검증을 분리해야 한다는 추가 근거다. 현재 자동화 환경에서 실제 Edge 창의 가시성을 입증하지 못했으며 제품 결함·사용자 행동·특정 확장이나 내부 대화상자의 인과를 확정하지 않는다. 같은 CDP 반복이나 전역 설정 변경으로 통과시키지 않고 실제 창 상태를 관찰할 수 있는 세션에서 재개한다. 이전 성공 수·원30분 실패·종료 생략과 T-001/T-021의 미완료 판정은 유지한다.

## 현재 판정

Rust 선택 검증과 수정 뒤 재검증은 완료했다. Chrome은30분 유지·복귀 업무 확인을 마쳤고 Edge는30분 유지 근거와 복귀 단언 실패를 보존했다. T-001·T-021·배포 준비는 NOT READY다.

## 남은 환경과 재개 절차

| 미완료 기준 | 필요한 환경·다음 확인 |
| --- | --- |
| T1-02 / T-021-05 | 개발 도구 없는 별도 깨끗한 Windows x64·Mac ARM64에서 해당 OS의 실행파일과 격리 `db/`로 UI/API·실행 의존성을 확인 |
| T1-06 / T-021-05 | 실제 절전·복귀와 해당 브라우저의 재연결·입력/저장, 실제 OS 브라우저 자동 열기 실패 조건과 안내 확인 |
| Edge 실제 창 복귀 | 원30분 실행의 visible 전환 실패는 미해결이다. 실제 창·브라우저 상태를 관찰할 수 있는 세션에서 visibility 전환과 앱 업무를 다시 구분해 확인하고 새 결과를 원실행과 분리 |
| 지원 환경 | 실제 검증한 OS/브라우저 버전과 미검증 범위를 구분하고 지원 최소 버전·다른 환경을 근거 없이 확정하지 않음. Safari UI 미실행 상태 유지 |
| 다운로드 실행·서명 | 실제 배포 경로의 다운로드·실행 권한/경고·Windows 서명 및 Mac 서명/공증을 확인. 로컬 NotSigned/ad-hoc 속성 검사로 대체하지 않음 |
| 이번 앱 종료·재시작 | 사용자 요청으로 생략. 별도의 명시적 요청 전에는 자동 종료·120초 유예·브라우저 강제 종료 후 앱 종료·종료/재시작 반복을 재개하지 않음 |

새 환경이 준비되면 실제 Git/goal 상태와 최신 기록을 먼저 읽고 해당 환경의 새 기준점을 만든다. 의존성 설치→표준 빌드→선택 검증 순서를 지키며, 사용자가 종료 검증을 다시 요청하기 전에는 종료 단계가 섞인 전체 실행기를 사용하지 않는다. 기존 자료는 보존하고 새 격리 DB와 해당 OS의 현재 실행파일·해시를 사용한다. 실제 결과·생략·미검증을 기준별로 갱신한 뒤 전체 완료 여부를 판단한다. T-027의 OS 왕복은 백로그로 유지한다.

## RTD After

- [Step 1] 목적 적합성 - PASS: Windows 환경 복구·새 빌드·종료 없는 정상 업무와 실제 브라우저 관측을 수행하고 Edge 복귀 실패·외부 필수 미검증을 구분했다.
- [Step 2] 오류·중요 문제·보안 - PASS(범위 검토): 실제 자료·세션·개인 경로는 공개 후보에서 제외했다. npm High7 잔여와 NotSigned·다운로드/지원 환경 미검증을 유지한다.
- [Step 3] 회귀 - PASS: 기존 사용자 자료·완료 티켓·운영 규칙·제품/API·잠금 파일을 보존했다. 시험 헬퍼 수정에서 DB/JSON·업무 파일의 바이트 불변 단언을 유지했다.
- [Step 4] 파일·함수 크기 - PASS: 시험 helper 한 곳과 결과·연결 문서만 변경했다. 제품 리팩터링이나 새 기능을 추가하지 않았다.
- [Step 5] 재사용·통합 - PASS: 기존 API 시나리오·UI 시험·표준 빌드를 사용했다. 로컬 스킬793개 파일의 양쪽 해시 대응을 확인했고 스킬은 수정하지 않았다.
- [Step 6] 부작용 - PASS: 새 가상 DB·전용 프로필·직접 생성 PID만 사용했다. 원30분 관측 정리 뒤 해당 경로의 활성 프로세스0개를 확인했다. 전역 설정·기본 브라우저·사용자 DB를 바꾸지 않았다.
- [Step 7] 전체 변경 재검토 - PASS: 독립 검토와 실제 diff·새 파일·원결과를 대조했다. Mac을 현재·Windows를 미래로 안내하던 오래된 요약을 현재 기록에 맞췄다.
- [Step 8] 불필요한 잔여물 - PASS: 과거 finalize/stage/sync·중단 Mac 실행기를 재실행하지 않았다. 원자료·실패·진단 사본은 로컬 Git 제외 경로에 보존한다.
- [Step 9] 품질 검증 - PASS(선택 범위): 빌드·Rust 고유65·UI165·API47·브라우저19개씩·타입/포맷/정적·자산 검사와 UTF-8/LF·상대 링크·diff·ignore 검사를 확인했다. 전체 Rust/E2E나 양 브라우저의30분/복귀 모두 통과로 표현하지 않는다.
- [Step 10] 사용자·개발자 흐름 - PASS(관측 범위): 오류 수정·부분 성공·원요청 재시도·복원 경계와 실제 화면을 확인했다. Chrome 복귀와 Edge의 제한된 근거를 구분하고 README/티켓에서 이 기록으로 연결한다.
- [Step 11] 발견 문제의 원인 - PASS(원인 구분): Windows 배타 잠금을 읽던 시험 helper의 OS 오류33을 재현·수정·재검증했다. sandbox의 쓰기 거부와 Edge visibility 전환의 원인 미확정을 제품 업무 결함으로 추정하지 않는다.
- [Step 12] READY 판단 - 기록·시험 수정 범위 READY / 전체 NOT READY: 확인한 결과와 실패를 보존했다. Edge 원프로토콜 복귀와 필수 외부 실환경 기준이 남아 T-001/T-021·배포 준비·전체 goal을 완료하지 않는다.
- [Step 13] 요청된 커밋 준비 - PASS: 사용자 권한에 따라 시험 helper를 독립 커밋했다. 문서 후보9개는 status·unstaged/staged diff·개별 ignore·공개 경계 검토를 거쳐 별도 논리 단위로 처리한다. ignored 자료·개인 변경은 포함하지 않으며 push·PR·배포는 수행하지 않는다.
