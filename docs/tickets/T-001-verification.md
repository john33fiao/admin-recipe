# T-001 검증 기록

- 일자: 2026-10-24
- 판정: **구현 변경 READY / 티켓 완료 NOT READY**
- 환경: macOS 27.0.1 ARM64, rustc/cargo 1.98.0, 셸 Node 24.20.0, npm 11.19.0. 이 환경의 npm 스크립트 내부 Node는 26.7.0으로 관측했다.
- 브라우저: Playwright 1.64.1 / Chromium 156.0.8078.4, headless. macOS 기본 브라우저 자동 열기 OS 호출도 성공했으나 해당 외부 브라우저 화면은 자동 관찰 도구에 노출되지 않았다.
- 기준: [T-001](T-001-portable-local-app.md), [계획](T-001-plan.md), [ADR-006](../decisions/ADR-006-local-runtime-and-journal.md)

## 실행한 명령과 결과

| 명령 | 결과 |
| --- | --- |
| `npm --prefix frontend ci` | 잠금 파일 기반 의존성 설치 |
| `node scripts/build.mjs` | Nuxt 정적 생성·입력/산출물 해시·Rust release 성공 |
| `cargo fmt --check` | 통과 |
| `cargo test --locked` | 31개 통과: 수명/저장 중 종료 6, 경로 1, hot journal 4(실제 회귀 3+하위 프로세스 보조 1), HTTP 3, 실제 WebSocket 4, 저장 13 |
| `cargo clippy --locked --all-targets -- -D warnings` | 통과 |
| `npm --prefix frontend test` | 3개 통과: Unicode 입력 검증·재시도 데이터 직렬화/복원 |
| `npm --prefix frontend run typecheck` | 통과 |
| `node --test scripts/ui-manifest.test.mjs` | 누락 산출물·소스 변경·자산 변조·청크 삭제 검증 통과 |
| `npm --prefix frontend run test:e2e` | Chromium 6개 사용자 흐름 통과 |
| `file`, `otool -L`로 release 검사 | ARM64 Mach-O, 약 4.2 MiB. AppKit/Foundation/CoreFoundation·libobjc·libiconv·libSystem만 동적 의존 |
| `git diff --check`, 공개 문서 상대 링크·UTF-8/LF·제외 규칙 확인 | 통과 |
| `npm --prefix frontend audit --json` | high 7개 경고 잔존. 아래 의존성 검토 참조 |

처음 Playwright 실행은 해당 버전의 브라우저 미설치로 실패했고 `npm exec -- playwright install chromium` 후 재실행했다. 기본 샌드박스의 TCP bind 제한은 승인된 로컬 테스트 실행으로 해소했다. 이전 실패를 통과 결과로 숨기지 않고 원인 해결 후 전체를 다시 검증했다.

## 의존성 보안 경고

`npm ci`와 `npm audit`에서 high 7개 경고를 확인했다. 이는 서로 다른 7개 결함이 아니라 Nuxt→Nitro→listhen→node-forge 1.4.0 경로의 같은 RSA 서명 검증 권고가 상위 패키지로 전파된 결과다. 2026-10-24 확인한 [GHSA-86w9-cpqp-85rv](https://github.com/advisories/GHSA-86w9-cpqp-85rv)는 수정 버전을 제시하지 않는다.

설치된 listhen 소스의 node-forge 사용은 개발 서버의 인증서 처리 경로다. 현재 앱은 정적 UI 생성 후 Rust HTTP 서버로 실행하며 Node/Nitro 서버·HTTPS 인증서 검증 기능을 배포하지 않는다. 현재 구현에서 해당 RSA 검증을 호출하는 경로는 찾지 못했다. 의존성 경고 자체는 해결되지 않았으며 개발 서버 HTTPS나 Node 서버 배포를 추가하기 전에 다시 검토해야 한다. npm이 제안한 Nuxt 4.1.3 강제 다운그레이드는 검증한 구성과 다르므로 적용하지 않았다. 감사 통과로 보고하지 않는다.

## 수용 기준별 결과

| ID | 결과·실행 증거 | 남은 검증 |
| --- | --- | --- |
| T1-01 | macOS release에 Nuxt 자산 내장. 자산 삭제/변조·기존 소스 변경 검증. 새 `public/` 입력 추가를 놓치는 감시 결함을 재현하고 전체 입력 루트 감시로 수정한 뒤 Cargo 실패 확인 | Windows release·CPU별 산출물 |
| T1-02 | 소스 없이 임시 폴더로 복사한 바이너리 실행, PATH를 비운 상태에서 UI/API 사용. macOS 시스템 동적 의존성 검사와 기본 브라우저 열기 OS 호출 성공 | 개발 도구가 없는 별도 Windows/Mac, 외부 기본 브라우저 화면 관찰, 다운로드 실행 권한·서명·공증 |
| T1-03 | 한글·공백 실행파일 경로, 다른 CWD, 실제 두 번째 프로세스 잠금 거절. db 경로가 파일일 때 명확한 오류 | 다른 OS 파일 권한·읽기 전용 디렉터리·교체 절차 |
| T1-04 | 상세 링크 이동·주소 직접 열기·새로고침, API/정적/사진 미존재 404, 세션 없는 요청·외부 Host/Origin 거절 | 별도 브라우저별 동작 |
| T1-05 | 실제 TCP/WebSocket의 두 연결·하나 종료·마지막 종료·유예 중 재연결, 부적절한 연결이 수명을 연장하지 않음. Chromium 복수 탭·새로고침 | 실제 브라우저 프로세스 강제 종료, 긴 백그라운드 유지, 기본 시간값의 장시간 수동 검증 |
| T1-06 | 초기 무접속 종료, 두 시계의 관측 공백 상태 테스트·UI reconnect 구현. 첫 세션 요청 실패 중 새로고침 회귀 통과 | 실제 절전 복귀, 자동 브라우저 열기 실패·브라우저별 장시간 시나리오 |
| T1-07 | 수락한 저장은 종료 신호 후 완료, 이후 요청 거절. JSON 발행 실패/재시도/재시작 복구·중복 방지. 실제 프로세스 중단의 hot journal 복구 | 실제 디스크 full·전원 차단·Windows 내구성 |
| T1-08 | 가상 재료 등록·목록/상세·동일 ID/단위·JSON 확인·재시작. 응답 유실 후 새로고침·같은 요청 재시도 시 1개만 유지 | 구현 범위의 macOS 흐름 충족 |
| T1-09 | 합성 v1→v2 변환·변환 전 복구본·변환 실패 원본 바이트 보존·미래 버전 거절. hot journal에 미완료 미래 값이 있어도 복구된 실제 버전 검사 | 정식 과거 출시본 fixture(아직 존재하지 않음), 다른 OS |
| T1-10 | 정상 종료한 db/를 같은 Mac의 다른 한글 폴더로 옮겨 동일 ID/단위 조회. DB만 이동한 누락 JSON 재발행 단위 테스트 | Windows↔macOS 이동, 사진 및 과거 JSON 전체 복원은 후속 범위 |

필수 OS별 수용 기준을 충족하지 못했으므로 T-001을 완료 처리하지 않는다. 등록된 다음 티켓은 없다. TODO의 후속 아이디어는 이번 작업 범위로 추가하지 않았다.

## 해결한 문제

- 새 입력 디렉터리를 Cargo가 감시하지 않아 오래된 UI를 그대로 빌드할 수 있었다. 실제 새 `public/` 파일 추가로 재현하고 루트 감시와 해시 검사를 결합했다.
- 세션 발급 전에 fragment를 제거하면 연결 중 새로고침 시 실행 토큰을 잃었다. 발급 성공 후 제거하고 요청 실패·새로고침 회귀를 검증했다.
- hot rollback journal이 남은 DB는 읽기 전용 버전 조회만으로 열 수 없었다. 실제 프로세스를 트랜잭션 도중 종료해 재현했다. 복사본 복구 후 버전을 확인해 지원 DB만 원본 복구하며 미래 DB/저널은 바이트 그대로 보존한다.
- 브라우저 테스트에서 SPA 전환 직후 주소를 너무 일찍 읽었다. URL 전환을 기다린 뒤 데이터 ID와 비교하도록 검증 코드를 고쳤다.

## 변경 후 검토

[Step 1] 목적 적합성 - PASS: 재료 등록·저장·조회와 휴대 실행 기반을 구현했다. 미충족 OS 기준은 완료로 표시하지 않았다.

[Step 2] 오류·중요 문제·보안 - PASS: SQL 매개변수·입력 제한·Host/Origin/세션·정적 경로 분리·잠금·hot journal을 검증했다. 공개 후보에는 가상 데이터만 있다.

[Step 3] 회귀 - PASS: 사용자 운영 결정·O-01~O-04·후속 범위를 유지했다. 시작 시 기존 Git 변경은 없었다.

[Step 4] 파일·함수 크기 - PASS: 수명/서버/저장/UI/빌드 책임을 분리했다. 저장 테스트와 실제 네트워크 테스트를 구분했다.

[Step 5] 재사용·통합 - PASS: 동일 Store를 실제 API와 테스트에서 사용한다. 기존 문서 색인과 ADR을 연결했다.

[Step 6] 부작용 - PASS: 개발 의존성·생성물·db/는 제외했다. 로컬 서버만 열며 JSON 기록은 무기한 증가한다. 전역 서비스·자동 동기화는 설치하지 않았다.

[Step 7] 전체 변경 재검토 - PASS: 독립 리뷰가 찾은 3개 문제를 수정하고 전체 테스트를 다시 실행했다. 최종 소스와 공개 diff를 재검토했다.

[Step 8] 불필요한 잔여물 - PASS: 테스트 자료는 임시 폴더 사용, 생성물·로그·운영 데이터·지침·스킬은 커밋에서 제외했다.

[Step 9] 품질 검증 - PASS: 위 빌드/테스트/형식/링크 검사 결과를 확인했다. 로컬 RTD 스킬 2개 검증기와 호환 심볼릭 링크도 확인했다.

[Step 10] 사용자·개발자 흐름 - PASS: 등록→상세→새로고침→재시작/이동, 응답 유실과 JSON 실패 재시도, 데스크톱 1280px·390px 다크 화면을 확인했다. README 명령으로 재현한다.

[Step 11] 발견 문제의 원인 - PASS: 실패를 실제로 재현한 뒤 원인을 수정했다. 저장·수명·UI 전체 회귀를 통과했다.

[Step 12] READY 판단 - PASS: 구현 변경은 검증 범위에서 READY. **T-001 전체는 NOT READY**이며 위 미검증 항목이 남는다. 연결 노트와 요구/구조 기록을 함께 갱신했다.

[Step 13] 요청된 커밋 준비 - PASS: 명시적 로컬 커밋 승인 범위에서 status·unstaged/staged diff와 후보별 ignore 검사를 수행한다. 비공개 자료·로컬 지침·운영 데이터는 제외한다. 푸시·PR·병합·배포는 하지 않는다.
