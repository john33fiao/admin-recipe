# T-020 검증 기록

- 작성일: 2026-10-24
- 관련: [티켓](T-020-ui-foundation.md), [구현 계획과 변경 전 검토](T-020-plan.md)
- 상태: READY — T-020 구현·신규 UI 검증·기존 T-001 브라우저 회귀 완료

## 구현 범위

- 기존 `/` 재료 등록·목록과 상세 주소를 보존하고 `/overview` 시작 화면과 공통 탐색을 추가했다. 시작 화면의 재료 수는 목록과 같은 실제 API 결과다.
- 목록 조회·재시도 상태를 `useMaterials`로 공유하고, 탭의 미전송 입력 보관·복구·보관 실패 안내를 추가했다. 서버의 영속 업무 임시 기록(T-005)과 구별한다.
- 페이지 이동 후 본문으로, 필수 입력 오류와 저장 완료 후 입력으로 포커스를 옮긴다. 현재 메뉴는 밑줄·굵기·`aria-current`로 표시한다.
- `/overview`만 서버의 명시적 SPA 경로에 추가했다. 세션·Host/Origin·API/자산의 오류 응답 정책은 유지한다.

## 실행 결과

Windows 11 Home x64, Node 24.19.0, npm 12.2.1, Rust 1.99.0, Nuxt 4.6.2, Vue 3.5.44, Playwright 1.64.1, Chromium 156.0.8078.4(revision 1248)에서 실행했다. 브라우저·Rust 저장 검증은 격리 임시 폴더의 가상 자료로 수행했다. Windows 샌드박스의 실행/파일 제약을 피하기 위해 해당 테스트 명령은 승인된 확장 권한으로 실행했으며 제품의 보안·저장 처리는 완화하지 않았다.

| 명령 | 최종 결과 |
| --- | --- |
| `node scripts/build.mjs` | PASS — Nuxt 정적 생성, 입력/산출물 기록, Rust release, 실행파일 경로 잔존 검사 1개 |
| `npm --prefix frontend test` | PASS — 5개. 신규 draft 테스트는 구현 전 모듈 없음으로 실패 후 통과 |
| `npm --prefix frontend run typecheck` | PASS |
| `npm --prefix frontend run test:e2e -- ui-foundation.spec.ts --output=../tmp/t020-playwright` | PASS — 6개, 9.5초, 화면 18개 생성 |
| `cargo fmt --check` | PASS |
| `cargo test --locked` | PASS — 37개(라이브러리 7, 실행파일 1, hot journal 4, HTTP 8, lifecycle HTTP 4, storage 13) |
| `cargo clippy --locked --all-targets -- -D warnings` | PASS |
| `node scripts/ui-manifest.mjs --check` | PASS |
| `node --test scripts/ui-manifest.test.mjs` | PASS — 조정 담당자가 1개 실행·확인 |
| 파일·문서 검사 | PASS — 최종 변경 후보 18개의 UTF-8/LF·상대 링크, `git diff --check`, `git status --short`, `git check-ignore --no-index -v` 확인. 후보의 ignore 일치 없음 |
| `npm --prefix frontend run test:e2e -- local-app.spec.ts --fully-parallel --workers=4 --output=../tmp/t020-local-app` | PASS — 조정 담당자가 기존 6개 회귀 실행·확인, exit 0, 4.1분. 정상 종료 120초 유예·응답 유실·JSON 실패·경로 이동·복수 탭·세션 재시도 통과 |

빌드에는 기존 Nitro cache-driver external 처리 경고와 Node의 shell 옵션 사용 중단 예고가 출력됐지만 명령은 성공했다. 이를 새 기능이나 검증 실패로 감추지 않는다. 현재 변경은 해당 빌드 도구·의존성을 바꾸지 않는다.

## 수용 기준별 증거

| 기준 | 구현·검증 근거 |
| --- | --- |
| T-020-01 | 시작→재료 입력/목록→상세 탐색, 실제 재료 수, 목록 빈/로딩/오류·새로고침, 저장 중 입력 잠금과 완료를 E2E로 확인 |
| T-020-02 | 1440×900, 768×1024, 390×844의 라이트/다크에서 시작·입력/목록·상세 18개 screenshot과 자동 검증. 긴 이름·단위·16개 가상 목록의 읽기와 주요 동작 확인 |
| T-020-03 | 모든 화면에서 document 가로 너비가 viewport를 넘지 않음. 긴 목록의 세로 높이가 viewport보다 커지고 전체 목록·footer까지 스크롤됨 |
| T-020-04 | Tab→본문 건너뛰기→Enter, 첫 오류 입력 포커스·연결된 오류 문구, 저장 후 입력 포커스, 경로 변경 후 본문 포커스, 레이블과 텍스트 상태를 확인 |
| T-020-05 | 초안의 이동/새로고침·연결 끊김/복구 보존, 보관 실패 문구, 저장 중 readonly, 화면 이탈 뒤 완료된 요청의 초안 정리·중복 없음 확인. 기존 불확실 요청 UUID 재시도 코드를 보존하고 단위 검사 |
| T-020-06 | UI·계획·이 기록은 단일 PC의 화면 너비 대응으로 설명하며 네트워크 공동 접속이나 모바일 서버 지원을 주장하지 않음 |

화면 파일은 로컬 `tmp/t020-playwright/ui-foundation-three-widths-f9dd8-form-list-and-detail-usable/` 아래 `materials`, `overview`, `detail`과 너비·테마 조합의 PNG 18개다. 모두 직접 열어 확인했다. 별도 리뷰어도 대표 PC/태블릿/모바일 4개에서 긴 이름·단위 줄바꿈과 주요 동작을 확인했다. 실제 운영 자료는 사용하지 않았다.

## 발견한 문제와 재검증

1. 저장 중 입력 화면을 떠나면 watcher가 해제되어, 응답 성공 후에도 탭에 옛 초안이 남았다. 지연 POST→시작 화면 렌더 및 입력 화면 제거→성공→페이지 다시 열기로 재현했으며, 이미 등록된 이름이 새 입력에 복구되는 실패를 확인했다. 성공 경로에서 초안을 명시적으로 삭제하고 불확실 요청 키보다 먼저 정리하며 입력 초기화 동안 pending을 유지하도록 수정했다. 동일 시나리오에서 빈 입력과 등록 1건을 확인했다.
2. 클라이언트 탐색은 가능했지만 서버의 SPA 허용 목록에 `/overview`가 없어 직접 GET·새로고침이 404였다. HTTP와 브라우저에서 실패를 재현한 뒤 경로 하나만 추가했다. GET/HEAD HTML·HEAD 빈 본문과 알 수 없는 경로의 404를 확인했다.
3. 새 HTTP fixture의 `HEAD /api/missing`은 기존 정책상 정확한 Origin이 필요하다. Origin 생략으로 발생한 403은 테스트 준비 오류였으며 fixture에 같은 Origin을 넣었다. 보안 처리를 바꾸지 않은 최종 37개 테스트가 통과했다.

## 변경 후 RTD

- [Step 1] 목적 적합성 - PASS: 위 6개 수용 기준에 탐색·상태·너비·테마·입력 보존 증거를 연결했다. 후속 업무 기능은 만들었다고 표시하지 않는다.
- [Step 2] 오류·중요 문제·보안 - PASS: 저장 완료 뒤 초안 재생성 위험과 직접 경로 404를 수정했다. 미확정 UUID·세션·Host/Origin·알 수 없는 API/자산 오류를 보존하고 가상 자료만 사용했다.
- [Step 3] 회귀 - PASS: 기존 루트와 상세 주소, 등록 API, 불확실 요청 재시도·연결·테마 의미를 유지했다. 기존 T-001 브라우저 6개도 최신 실행파일로 통과했다.
- [Step 4] 파일·함수 크기 - PASS: 목록 상태 공유·draft 복구를 각각 작은 모듈로 분리하고 백엔드는 경로 1개만 추가했다. 업무 계산·저장 스키마를 섞지 않았다.
- [Step 5] 재사용·통합 - PASS: 기존 Nuxt·CSS 토큰·등록 컴포저블과 실제 재료 API를 재사용한다. 새 프레임워크나 외부 자산은 없다.
- [Step 6] 부작용 - PASS: 작성 중 입력은 탭의 sessionStorage에만 보관한다. 새 DB/JSON 업무 기록·외부 서비스·백그라운드 작업·전역 설정은 만들지 않았다.
- [Step 7] 전체 변경 재검토 - PASS: 최종 제품 diff·신규 파일·테스트와 화면을 읽었고 독립 리뷰의 두 문제를 재현·수정했다.
- [Step 8] 불필요한 잔여물 - PASS: 미구현 업무의 가짜 수치·메뉴를 제외했다. 테스트 출력은 로컬 tmp에 두며 공개 후보와 분리한다.
- [Step 9] 품질 검증 - PASS: 표준 빌드, 단위·타입·브라우저·Rust·포맷·정적 검사와 자산 정합성을 실행했다. 최종 변경 후보 18개의 문서 링크·UTF-8/LF·diff·제외 규칙을 확인했고 ignore 일치는 없다.
- [Step 10] 사용자·개발자 흐름 - PASS: 시작에서 실제 목록·입력과 상세로 이동하고 직접 경로 새로고침을 확인했다. 계획·티켓·검증 기록을 연결했으며 개발자의 후속 업무 화면 기준을 명시했다.
- [Step 11] 발견 문제의 원인 - PASS: 화면 수명에 종속된 초안 정리와 서버 경로 누락을 각각 재현 테스트로 고쳤다. 최종 신규 UI 6개·Rust 37개가 통과했다.
- [Step 12] READY 판단 - PASS: T-020 수용 기준과 신규 UI 6개·기존 T-001 브라우저 6개·Rust 37개 등 관련 검증을 통과했다. 독립 리뷰에서 찾은 두 문제를 해결했으며 T-020 범위는 READY다. 전체 앱 출시 준비 완료와는 구분한다.
- [Step 13] 요청된 커밋·PR 준비 - PASS: 현재 사용자의 명시적 승인에 따라 최종 18개 공개 파일의 status·unstaged/staged diff와 제외 규칙을 확인한다. 기존 staged 변경은 없고 T-003 계획·개인 자료·검증 출력은 포함하지 않는다. 메시지는 실제 staged diff로 작성하며 PR·push·배포는 수행하지 않는다.

## 검증 범위의 한계

세 화면 너비는 단일 PC 브라우저의 반응형 화면 검증이다. 네트워크 공동 접속·모바일 서버·별도 깨끗한 Windows/Mac·실제 절전·OS 간 데이터 이동 검증 완료를 뜻하지 않는다. 신규 UI 테스트는 fixture를 강제 종료하여 정리하며, 정상 종료 근거는 기존 T-001 E2E에서 따로 확인했다. 구현 담당자는 스테이징·커밋·PR·배포를 수행하지 않았다.
