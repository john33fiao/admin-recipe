# ADR-006: 로컬 실행·재료 저장·JSON 기록 구현

- 상태: 채택 — T-001의 구현 세부, OS별 배포 검증은 일부 미완료
- 결정일: 2026-10-24
- 관련: C-01, D-05, D-10~D-13, [T-001](../tickets/T-001-portable-local-app.md)
- [ADR-005](ADR-005-rust-nuxt-local-server.md)의 사용자 결정은 유지한다. O-01~O-04·수량·원가 정책을 변경하지 않는다.

## 선택과 근거

Nuxt 4.6.2 정적 SPA를 rust-embed 8.13.0으로 내장하고 Axum 0.8.9/Tokio 1.54.1 서버가 제공한다. rusqlite 0.40.2 `bundled`로 SQLite를 함께 빌드한다. Rust 최소 버전은 1.98이며, Rust 라이브러리와 Nuxt/Vue/TypeScript 및 전이 의존성은 Cargo.lock·frontend/package-lock.json에 고정한다. 별도 Node 서버 방식은 단일 바이너리 조건에 불필요한 실행 의존성을 더하므로 채택하지 않았다.

TypeScript 7.0.2는 vue-tsc 3.3.13의 내부 tsc 참조와 호환되지 않아 실제 타입 검사에 성공한 6.0.3으로 고정했다. Vue 3.5.44·vue-router 5.4.0을 사용한다. Node는 Nuxt가 요구하는 `^22.22.3 || ^24.15.0 || >=26.0.0` 범위를 따른다.

## 실행과 연결

- 실행파일 위치의 `db/`를 사용한다. `--no-browser`는 검증·수동 주소 열기용이며 데이터 위치는 바꾸지 않는다.
- `.writer.lock`의 OS 잠금을 프로세스 수명 동안 유지한다. 중복 실행은 명확한 오류로 종료한다. 기존 인스턴스를 자동으로 여는 기능은 없다.
- `127.0.0.1`의 임의 포트에 먼저 바인딩한 뒤 기본 브라우저를 연다. 실패하면 터미널의 주소로 직접 접속할 수 있고 최초 대기는 120초다.
- 실행마다 새 임의 토큰을 URL fragment로 전달한다. 동일 Origin POST로 HttpOnly·SameSite=Strict 세션 쿠키를 발급한 뒤 fragment를 제거한다. 비밀번호 입력은 없다. Host·Origin·세션을 API와 WebSocket에서 검사한다. 외부 웹사이트 요청은 거절하며 데이터 폴더는 HTTP 정적 경로에 노출하지 않는다.
- 서버 Ping은 15초, Pong 없는 연결은 45초 후 만료한다. 초기 접속 대기와 연결 0개 이후 유예는 각각 120초다. 유효한 연결 하나가 남아 있으면 유지한다.
- monotonic clock과 wall clock의 관측 공백이 30초를 넘으면 새 유예를 준다. OS 절전 여부를 확정하는 검출기는 아니며 긴 스케줄링 중단도 복귀 기회로 취급한다. UI는 focus·online·visible 복귀 때 소켓을 다시 연결한다.
- 종료 시 새 작업을 거절하고 수락한 저장이 마칠 때까지 기다린다. WebSocket upgrade 작업도 별도로 추적해 닫는다. DB 작업은 blocking 작업 스레드에서 mutex로 직렬화한다.

## 데이터 계약

`db/app.sqlite3`의 스키마 버전은 2다. 재료는 UUID·trim한 이름(1~100 Unicode 문자)·관리 단위(1~20 문자)·UTC 생성 시각을 보관한다. 제어 문자는 거절하며 이름 중복은 허용한다. 기본 단위는 UI의 g이고 자유 단위를 지원한다. 단위 환산·수량·원가·수정·삭제는 이번 작은 흐름의 범위 밖이다.

변경 요청 UUID는 영구 보관한다. 같은 UUID와 같은 입력은 원래 재료를 반환하고, 다른 입력은 409로 거절한다. UI는 전송 실패 시 같은 요청을 다시 보내며 응답이 불확실한 동안 내용 편집을 막는다. sessionStorage 복구는 같은 출처·탭의 새로고침 범위다. 앱을 다시 실행해 포트가 바뀌면 이전 탭의 임시 요청을 자동 이관하지 않는다. 이 경우 저장 목록을 확인한 뒤 새 입력 여부를 판단한다.

## JSON 완료 의미와 복구

1. 재료·요청 UUID·전체 상태 JSON 원문을 한 SQLite 트랜잭션으로 저장한다.
2. `db/snapshots/00000000000000000001.json`처럼 증가하는 순번의 파일을 임시 파일→flush→덮어쓰기 없는 발행 순서로 저장한다.
3. JSON 발행까지 성공해야 HTTP 성공을 반환한다. JSON 실패 후에는 DB에 변경이 이미 있을 수 있으므로 완료라고 표시하지 않고 동일 요청 재시도를 안내한다.
4. 다음 변경 전·재시작 시 DB에 보존한 원문과 모든 기존 JSON을 대조하고 누락 파일을 재생성한다. 불일치는 덮어쓰지 않고 오류로 멈춘다. 미해결 기록이 있으면 후속 변경을 막는다.

JSON 형식 1은 `format_version`, `schema_version`, `sequence`, `request_id`, `recorded_at`, `materials`를 담는다. 기록은 무기한 보관한다. 전체 스냅샷 방식이므로 변경 수와 재료 수에 따라 저장 공간·시작 검사 시간이 증가한다. 현재 작은 저장 흐름에서 선택한 단순성의 비용이며 향후 형식 전환 시 기존 기록 호환을 보존해야 한다.

SQLite는 DELETE journal과 synchronous FULL을 사용한다. 정상 종료 뒤 DB만 이동해도 JSON 원문은 DB에서 다시 내보낼 수 있다. 사진은 아직 구현하지 않았다. 사진을 포함한 온전한 이동은 여전히 db/ 전체 복사다. JSON으로 선택 상태를 전체 복원하는 기능은 이 기록 발행 기능과 구분하며 미구현이다.

파일 flush는 모든 OS에 적용하고 디렉터리 sync는 Unix에 적용한다. Windows 디렉터리 flush 및 실제 전원 차단 내구성은 미검증이다. 파일시스템·장치가 flush를 준수한다는 가정은 SQLite 자체와 동일하게 남는다.

## 이전 DB와 실패 처리

v0 빈 DB→v1 재료/기록 테이블→v2 요청 UUID 테이블을 순차 생성한다. v1은 실제 정식 출시 이력이 아닌 합성 fixture다. 변환 전 SQLite 백업 API로 `db/backups/pre-v1-<uuid>.sqlite3`를 만들고 모든 변환·검증을 한 트랜잭션에 수행한다. 실패하면 원본 데이터·스키마를 유지한다. 미래 버전은 원본 DB를 변경하지 않고 거절한다.

강제 종료 후 hot rollback journal이 남은 경우 읽기 전용 조회만으로 복구할 수 없다. 원본과 저널의 임시 복사본에서 SQLite 복구·버전 검사를 먼저 수행하고, 지원 버전일 때 원본 복구를 허용한다. 미완료 트랜잭션의 헤더 버전을 그대로 신뢰하지 않는다.

## 빌드·검증과 되돌리기

`node scripts/build.mjs`가 UI 생성→입력/산출물 SHA-256 manifest→Cargo release를 실행한다. build.rs는 새 입력 경로를 포함한 소스 변경, 자산 삭제·변조를 거절한다. 생성물은 Git에서 제외하고 공개 복제본에서 재생성한다.

2026-10-24 Windows 검사에서 release의 소스 위치 문자열에 개인 경로가 남는 것을 재현했다. 빌드 스크립트에서 홈·소스 경로를 치환하고 `node --test scripts/release-artifact.test.mjs`로 UTF-8/UTF-16LE 산출물 검사를 추가했다. 기존 실행파일에서 실패, 정상 재빌드 후 통과를 확인했다. Windows static CRT 설정은 유지한다. C/링커 등 별도 도구의 경로를 모두 치환한다고 가정하지 않고 최종 산출물을 검사한다. Windows 후속 검증 뒤 같은 날짜에 macOS ARM64에서도 정상 빌드·산출물 검사를 통과했다. 환경·해시·회귀 범위는 T-001 검증 기록에 구분해 남긴다.

검증 결과와 한계는 [T-001 검증 기록](../tickets/T-001-verification.md)에 둔다. macOS ARM64의 로컬 성공이 Windows·다른 Mac·다운로드 서명/공증·실제 절전의 성공을 뜻하지 않는다. 앱 교체 전 종료하고 db/ 전체를 보존한다. 예전 바이너리로 되돌리는 것만으로 DB 버전이 되돌아가지는 않는다.

## 공식 근거

- [Nuxt 정적 배포](https://nuxt.com/docs/4.x/getting-started/deployment)
- [Axum HTTP·WebSocket](https://docs.rs/axum/0.8.9/axum/)
- [rusqlite 빌드와 bundled SQLite](https://github.com/rusqlite/rusqlite#notes-on-building-rusqlite-and-libsqlite3-sys)
- [SQLite 원자적 커밋과 hot journal](https://www.sqlite.org/atomiccommit.html)
- [SQLite 백업 API](https://sqlite.org/backup.html)
- [WebSocket Ping/Pong](https://websockets.spec.whatwg.org/#ping-and-pong-frames)
- [Rust Instant의 OS별 동작](https://doc.rust-lang.org/stable/std/time/struct.Instant.html)
- [Tokio 지연 tick 정책](https://docs.rs/tokio/latest/tokio/time/enum.MissedTickBehavior.html)
