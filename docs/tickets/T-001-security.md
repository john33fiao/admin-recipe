# T-001 보안 검토 · 2026-10-24

대상은 공개 예정 소스·잠금 파일·release 산출물과 로컬 실행/저장 경계다. 실제 자료 대신 합성 DB·가상 재료·별도 브라우저 프로필을 사용했다. 실행 환경·명령·한계는 [검증 기록](T-001-verification.md)에 있다. 이 검토는 배포 인증이나 모든 취약점의 부재를 보장하지 않는다.

## 발견·조치

| 구분 | 근거·재현 조건 | 영향과 조치 | 재검증 |
| --- | --- | --- | --- |
| 확정: 실행파일의 개인 경로 노출 | 기존 Windows release 바이트에 사용자 홈 경로 문자열 64건. 심볼 strip만으로 Rust 소스 위치 문자열이 제거되지 않음 | 실행파일 공유 시 빌드 PC 경로 노출. `scripts/build.mjs`에서 홈·프로젝트 경로를 rustc `--remap-path-prefix`로 치환. Cargo에는 셸 없이 인수를 전달하고 MSVC static CRT 설정 유지 | `node --test scripts/release-artifact.test.mjs` 수정 전 실패, 정상 release 재빌드 후 통과. UTF-8/UTF-16LE와 양쪽 경로 구분자 검사. 실행파일은 Git 제외이며 원본 바이너리를 공개하지 않음 |
| 확정: Windows E2E 정상 종료 판정 오류 | `child.kill('SIGINT')` 뒤 종료 코드는 null이며 정상 종료 메시지가 없음. 이전 헬퍼는 이를 구별하지 않았고 정리 단계도 지연됨. 이미 종료한 프로세스를 건너뛰는 누락도 exit 7 통제로 재현 | 배포/이동 검증의 신뢰성 결함. 모든 테스트 페이지를 빈 페이지로 이동하고 기본 유예 뒤 코드 0·정상 종료 메시지를 확인. 이미 종료한 앱도 검사하며 `close`로 stdout 수집 완료를 기다림. 강제 정리는 실패 상태를 유지 | 비정상·강제 종료 통제는 거절하고 정상 종료 통제는 통과. 정상 종료 후에만 실행파일 교체·db/ 이동. 강제 브라우저 종료와 hot journal 복구는 별도 시나리오 |
| 잔여: node-forge 권고 | 잠금 버전 1.4.0, npm audit high 7개 | 동일 권고의 상위 의존성 전파. 수정 버전 없음. 아래 도달 가능성·재검토 조건에 따라 유지 | 감사 경고 해결 또는 npm 감사 통과로 보고하지 않음 |
| 탐지 오탐 | Gitleaks 8.30.1이 `useMaterialRegistration.ts`의 `storageKey` 한 곳을 generic-api-key로 분류 | 값은 공개 상수 `sunchang.pending-material.v1`; sessionStorage에 실패 요청을 보존하는 키 이름이며 인증정보가 아님. 이력과 현재 트리의 같은 탐지 | 광범위 allowlist나 탐지 비활성화 없이 원문·사용처 대조 |

경로 치환은 [rustc 공식 옵션](https://doc.rust-lang.org/rustc/command-line-arguments.html#--remap-path-prefix-remap-source-names-in-output), target flags 결합은 [Cargo 설정](https://doc.rust-lang.org/cargo/reference/config.html#targettriplerustflags)을 따른다. 링커나 C 도구의 별도 경로까지 자동으로 제거한다고 가정하지 않고 최종 산출물을 검사한다. Windows 신호 동작은 [Node child_process 문서](https://nodejs.org/api/child_process.html#subprocesskillsignal)와 실제 실패로 확인했다.

## 잠금 파일·공급망

`npm ci`는 현재 잠금 파일을 그대로 사용했다. 설치된 경로는 Nuxt 4.6.2 → @nuxt/nitro-server 4.6.2 → nitropack 2.13.5 → listhen 1.10.1 → node-forge 1.4.0이다. 공식 npm registry의 최신 버전과 대조했으며 이 경로의 버전을 바꾸지 않았다.

[GHSA-86w9-cpqp-85rv / CVE-2026-85393](https://github.com/advisories/GHSA-86w9-cpqp-85rv)는 2026-09-03 공개·10-01 갱신, 10-24 재확인 시 영향 범위 `<=1.4.0`, 수정 버전 없음이다. 특정 인코딩의 RSA PKCS#1 v1.5 위조 서명을 받아들일 수 있는 결함이다. 7개 high를 7개의 독립 결함으로 계산하지 않는다.

| 단계 | 실제 사용과 노출 판단 |
| --- | --- |
| 개발 | listhen의 설치 소스에서 인증서 생성·개인키 서명, PEM/PFX 읽기 경로를 확인했다. 해당 호출에서 취약 RSA 서명 검증 함수 호출은 찾지 못했다. 현재 `dev`는 loopback HTTP다. 외부 인증서·서명을 받아 검증하는 기능은 구현하지 않았다. |
| 설치·빌드 | Nuxt 준비/정적 생성 과정에서 Node 의존성을 실행한다. 취약 패키지가 개발 도구에 존재하므로 공급망 위험 자체는 남는다. 프로젝트 코드·설정·의존성은 신뢰하는 입력이어야 한다. `npm audit fix --force`나 Nuxt 4.1.3 다운그레이드는 적용하지 않았다. |
| 배포 실행 | 내장 `.output/public`과 Rust HTTP 서버만 사용한다. Node/Nitro 서버·listhen·forge를 배포하지 않으며 생성 UI에서 forge/RSA 검증 경로를 찾지 못했다. 이는 현재 구성의 정적 도달 가능성 판단이며 패키지 자체의 안전 판정은 아니다. |

HTTPS 개발 서버, 외부 키/인증서, Node 서버 배포 또는 권고/수정 버전이 바뀌면 다시 감사한다. `cargo audit 0.22.2 --file Cargo.lock --json`은 취약점 0·경고 0이었다. RustSec DB는 1,352건, 커밋 `1645308cb4766561b0b274d8c93f7033a766fc96`이다. 잠긴 Tokio 1.54.1은 [RUSTSEC-2026-0384](https://rustsec.org/advisories/RUSTSEC-2026-0384.html)의 수정 버전이다. 감사는 해당 시점에 알려진 권고만 확인한다.

설치 스크립트가 있는 잠금 항목은 프로젝트의 `nuxt prepare`, esbuild 0.28.2, macOS 전용 선택 의존성 fsevents 2.3.3이다. esbuild는 플랫폼 바이너리를 준비하며 프로젝트 postinstall은 Nuxt 개발 타입/생성물을 준비한다. Windows에서 fsevents는 실행 대상이 아니다. Rust 빌드 스크립트와 bundled SQLite C 컴파일도 개발 호스트에서 실행된다. npm 11.19.0은 공식 registry tarball의 SHA-512, Gitleaks는 공식 release SHA-256을 대조했다. cargo-audit는 crates.io에서 `--locked`로 빌드했다. 빌드 도구·node_modules·캐시·검증 로그는 배포물과 공개 커밋에서 제외한다. 정적 생성의 Nitro cache-driver 경고와 Windows npm 실행의 DEP0190 경고는 남지만 빌드는 성공했다. npm 셸 인수는 고정된 `run generate`이고 사용자 입력을 전달하지 않는다.

## 실행 경계 검토

| 경계 | 근거·검증 | 잔여 위험·한계 |
| --- | --- | --- |
| loopback·DNS rebinding·CSRF | `127.0.0.1:0`만 bind. Host는 실제 IP:port와 정확히 비교. 다른 Origin 거절, 쓰기·세션 발급·WebSocket은 정확한 Origin 필수. 외부/다른 포트/null/없는 Origin과 유효 쿠키 조합도 403 확인 | 네이티브 로컬 프로세스는 HTTP 헤더를 위조할 수 있다. 외부 웹페이지와 로컬 OS 권한의 경계는 다름 |
| 실행 토큰·세션 | 매 실행 UUID v4 두 개를 연결한 임의 토큰. URL fragment로 전달하고 세션 발급 성공 뒤 제거. HttpOnly·SameSite=Strict·세션 쿠키. 잘못된 토큰 401, 응답 본문에 토큰 없음 | 주소 출력은 수동 접속을 위한 인증자료다. 터미널·브라우저 프로필·실패 trace를 공유하면 유출될 수 있다. 쿠키는 포트 간 보안 격리를 제공하지 않으며 같은 호스트의 악성 로컬 서비스/같은 OS 사용자를 방어하는 인증 체계가 아님 |
| WebSocket | Host/Origin/쿠키 통과 후 upgrade. 잘못된 Origin/쿠키의 반복 연결은 앱 수명을 늘리지 않음. 프레임·메시지 1 KiB 제한, Ping/Pong 만료·종료 시 소켓 정리 | 연결 수 총량·요청 빈도 상한은 없음. 인증된 세션 또는 로컬 프로세스의 자원 고갈 가능성을 완전히 제한하지 않음. 인터넷/다중 사용자 서비스로 확장하기 전 제한 설계 필요 |
| 입력·SQL·XSS | JSON body 16 KiB, 이름 100자·단위 20자 및 제어문자 검증, SQL 매개변수 사용. 17 KiB 요청 413. SQL 구문 이름을 문자로 저장. Vue 보간, 악성 HTML 이름의 실제 브라우저 표시 검사 | 저장된 데이터·변경 JSON은 무기한 증가. 목록 페이지 분할·전체 스냅샷 저장량 최적화는 후속 운영 규모 검토 대상 |
| 정적 자산·경로 | 내장 자산만 제공. db/.git/.env 및 일반/인코딩된 상위 경로 요청 404. API/누락 자산을 SPA 성공으로 바꾸지 않음. 입력/산출물 트리의 심볼릭 링크를 빌드에서 거절 | db/의 junction·symlink를 다른 곳으로 바꿀 수 있는 로컬 파일 권한까지 차단하지 않음. 다른 사용자가 쓸 수 있는 공유 폴더를 신뢰 경계로 사용하지 않음 |
| SQLite·호환 | 파일 잠금, 트랜잭션, 합성 v1 변환 전 복구본, 실패 원본 보존, 미래 DB·hot journal 원본 보호 테스트 | 정식 과거 출시본은 아직 없음. 악성 사용자가 직접 수정한 DB는 비신뢰 입력 방어 범위의 한계가 있음 |
| 저장·종료 | 수락한 쓰기 완료·후속 쓰기 거절, SQLite FULL 롤백, JSON 발행 실패·동일 요청 재시도·재시작 복구, 실제 하위 프로세스 종료의 hot journal 회귀 | 실제 디스크 full·전원 차단과 Windows 디렉터리 flush 내구성은 미검증. SQLite FULL 오류 주입을 물리 디스크 시험으로 표현하지 않음 |

현재 단일 PC·사용자 정책에서 비밀번호 잠금을 추가하지 않았다. 로컬 OS 사용자/파일 권한과 브라우저 프로필을 신뢰한다는 기존 실행 경계를 문서화했다. 이 검토에서 외부 웹페이지가 인증 경계를 넘어 저장을 수행하는 확정 경로는 찾지 못했다. 제한 없는 로컬 자원 소비와 파일 변조는 잠재 위험으로 남으며, 공동 접속/서버 전환 시 재설계해야 한다.

## 공개 경계와 재현

전송 대상 `ccfee58..HEAD`의 두 기존 커밋과 새 변경, 추적/공개 후보 파일, 생성 정적 자산과 실행파일을 검사했다. Gitleaks JSON은 `--redact`로 기록하고 고정 저장소 키 오탐을 분류했다. 개인 경로·비공개 문서 링크·개인키 패턴과 파일 목록을 추가 점검했다. 실제 운영 자료·인증정보·로컬 지침·스킬은 공개 후보에 없다. 로그·trace·브라우저 프로필·가상 DB와 산출물은 ignore 규칙 아래에 보관한다. Git 제외는 이전 커밋을 숨기는 수단이 아니므로 push 직전에 모든 전송 커밋을 다시 검사한다.

재현 명령은 `npm --prefix frontend audit --json`, `cargo audit --file Cargo.lock --json`, `cargo test --locked`, `npm --prefix frontend run test:e2e`, `node scripts/build.mjs`, `node --test scripts/release-artifact.test.mjs`다. Gitleaks는 확정한 원격 기준과 HEAD 사이의 `git --log-opts` 검사 및 공개 파일만 복사한 디렉터리 검사를 수행한다. 원시 결과는 토큰·개인 경로를 포함할 수 있어 커밋하지 않는다.
