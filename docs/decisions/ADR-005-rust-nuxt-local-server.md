# ADR-005: Rust·Nuxt 로컬 서버와 DB 하위호환성

- 상태: 채택 — 사용자 결정 유지. 세부 제안은 [ADR-006](ADR-006-local-runtime-and-journal.md)으로 구현, 검증 결과는 T-001 참조
- 결정일: 2026-10-24
- 관련: D-10~D-13, [ADR-003](ADR-003-inventory-events-and-portable-data.md), [ADR-004](ADR-004-inventory-exception-policies.md)
- 첫 구현 준비: [T-001](../tickets/T-001-portable-local-app.md)

## 문제와 제약

단일 PC에서 레시피·재고를 관리하며 바이너리 하나와 옆의 `db/` 폴더만 옮겨 사용할 수 있어야 한다. SQLite·JSON 변경 기록·사진을 함께 보존한다. 기존 운영 정책과 O-01~O-04는 변경하지 않는다. 이 결정 당시 앱 코드와 매니페스트는 없었다. 현재 구현은 ADR-006을 따른다.

## 사용자 확정 사항

1. 백엔드는 Rust, 프론트엔드는 Nuxt다. 다른 언어·프레임워크를 다시 선택받지 않는다.
2. 바이너리가 로컬 서버를 시작하고 웹브라우저를 자동으로 열어 사용한다.
3. 바이너리와 브라우저는 WebSocket heartbeat로 연결을 확인한다. 일정 시간 이상 연결이 끊기면 바이너리도 종료한다. 구체적인 간격·유예 시간은 구현 설계다.
4. Windows와 macOS용 빌드를 대상으로 한다. 2026-10-24 후속 사용자 결정으로 이번 검증 대상 CPU를 **Windows x64·macOS ARM64**로 정했다(D-12). 최소 OS와 배포·브라우저 지원 보장은 실제 검증을 마친 뒤 정한다. Windows ARM64·Intel Mac 산출물은 이번 대상에 포함하지 않는다.
5. 앱 내부 업데이트는 현재 범위에서 제외한다.
6. DB 하위호환성은 **새 앱이 이전 버전 DB를 계속 열어 사용할 수 있음**을 뜻한다. 사용자가 이 방향을 명시적으로 확인했다. 이전 앱이 새 스키마 DB를 사용하는 역방향 호환은 보장 범위에 포함하지 않는다.
7. Electron 확장은 미래 가능성으로만 남기고 현재 설계·의존성·검증 범위에 포함하지 않는다.

## 세부 구성 제안과 대안

아래는 확정 요구를 구현하기 위한 개발자 제안이다. 라이브러리와 버전은 첫 구현 검증에서 확정하며, 사용자 지정 기술을 바꾸는 제안이 아니다.

| 구성 | 판단과 근거 |
| --- | --- |
| Nuxt 정적 SPA를 Rust에 내장 | 권장. `ssr: false`와 `nuxt generate`로 만든 `.output/public`을 Rust가 제공한다. 실행 시 별도 Node 서버를 요구하지 않는 구성이다. 로컬 업무 화면은 검색엔진용 SSR을 요구하지 않는다. |
| Nuxt SSR 서버와 Rust 서버를 함께 실행 | 현재 미채택. Node 서버 프리셋은 Node 실행 진입점을 만들므로 별도 런타임·프로세스 포장이 필요하다. 현재 단일 바이너리 조건에 불필요한 부담이다. |

Nuxt 공식 배포 문서의 [정적 SPA와 Node 서버 설명](https://raw.githubusercontent.com/nuxt/nuxt/main/docs/1.getting-started/16.deployment.md)을 근거로 한 판단이다. 정적 SPA에서도 Nuxt 라우팅·Vue 컴포넌트를 사용하며 업무 API와 파일 저장은 Rust가 담당한다. Nuxt의 서버 API·SSR 전용 기능에 업무 처리를 넣지 않는다.

| 부분 | 권장 구현안 | 확인할 한계 |
| --- | --- | --- |
| HTTP·WebSocket | Tokio + Axum | [Axum WebSocket](https://docs.rs/axum/latest/axum/extract/ws/index.html) 지원을 확인했다. 프로세스 수명과 종료 처리는 앱이 구현한다. |
| UI 자산 내장 | rust-embed | [release 빌드 내장](https://docs.rs/rust-embed/latest/rust_embed/trait.RustEmbed.html)을 지원한다. 기본 debug 빌드는 파일시스템을 읽으므로 휴대성 검증에는 release 산출물을 사용한다. |
| SQLite | rusqlite의 `bundled` 기능 | [공식 빌드 설명](https://github.com/rusqlite/rusqlite#notes-on-building-rusqlite-and-libsqlite3-sys)에 따라 SQLite C 소스를 함께 컴파일한다. 빌드 도구는 필요하며 별도 SQLite DLL 불필요 여부는 산출물로 검증한다. |

동기 DB 처리는 전용 작업 스레드 등으로 직렬화해 WebSocket 처리를 막지 않도록 제안한다. 의존성·도구 버전과 잠금 파일은 구현 시 고정한다. MIT와 제3자 고지는 보존한다.

## 빌드와 OS별 실행

- 빌드 순서는 **Nuxt 정적 자산 생성 → Rust에 내장 → OS별 release 빌드**다. 현재 `cargo build`가 Nuxt까지 빌드하는 설정은 없다. 한 명령으로 묶을지는 빌드 자동화에서 결정하고, 누락·오래된 UI 자산으로 배포하지 않도록 검증한다.
- Cargo는 기본적으로 호스트 대상을 빌드한다. OS·아키텍처별 산출물이 필요하며, 한 번의 기본 빌드가 Windows·Mac 실행파일을 모두 만드는 것은 아니다. [Cargo build](https://doc.rust-lang.org/cargo/commands/cargo-build.html)
- Windows·macOS 각 환경에서 빌드·실행하는 CI 구성을 우선 제안한다. Rust 지원과 별개로 C 컴파일러·링커·SDK가 필요하다. macOS 교차 빌드는 SDK가 필요할 수 있고, 비 Windows 호스트에서 MSVC로의 교차 빌드는 공식 지원 대상이 아니다. [macOS 대상](https://doc.rust-lang.org/rustc/platform-support/apple-darwin.html), [Windows MSVC 대상](https://doc.rust-lang.org/rustc/platform-support/windows-msvc.html)
- Windows MSVC의 C 런타임 정적 링크를 검토하고 실제 의존 DLL을 검사한다. macOS의 시스템 라이브러리 의존성은 OS 제공분과 추가 설치분을 구분한다. `bundled`나 Rust 사용만으로 실행 의존성이 모두 사라진다고 단정하지 않는다. [Rust C 런타임 링크](https://doc.rust-lang.org/reference/linkage.html#static-and-dynamic-c-runtimes)
- macOS는 다운로드한 배포물의 실행 권한·Gatekeeper·서명·공증과 Finder에서의 실행 동작을 확인한다. 로컬에서 컴파일해 실행한 결과만으로 다운로드 배포를 검증하지 않는다. 서명·공증의 제공 범위는 배포 전에 정한다. [Apple 배포 안내](https://developer.apple.com/developer-id/)
- 사용자가 쓰기 가능한 위치에서 실행한다. 현재 작업 디렉터리가 아닌 실행파일 옆 `db/`를 사용한다. 한글·공백 경로, 파일 잠금·교체, 서로 다른 OS의 경로 처리도 검증한다.
- 개발 도구가 없는 PC에서 바이너리와 `db/`만으로 실행하고, 소스·외부 UI 폴더·별도 Node·SQLite 설치 없이 동작하는 것을 확인해야 단일 바이너리 조건을 충족했다고 보고한다. 현재는 미검증이다.

## 시작·heartbeat·종료 제안

1. `db/` 쓰기 가능 여부와 같은 데이터 폴더의 중복 실행을 확인한다. 기존 인스턴스가 있다면 새 DB 작성자를 만들지 않고 기존 화면을 여는 안을 우선한다.
2. loopback 주소에 서버를 바인딩한 뒤 실제 포트로 기본 브라우저를 연다. 시작 실패 시 브라우저를 먼저 열지 않는다. 브라우저 자동 열기 실패 시 주소·오류를 알린다.
3. 초기 연결 대기, 정상 연결, 재연결 유예, 안전 종료 상태를 구분한다. **모든 유효한 화면 연결이 끊긴 상태**가 유예 시간을 넘었을 때 종료한다. 한 탭을 닫아도 다른 탭이 살아 있으면 유지하고 새로고침은 재연결로 처리한다.
4. 서버가 WebSocket Ping을 보내고 Pong을 확인하는 방식을 우선한다. 프로토콜 Ping/Pong은 브라우저 JavaScript API에 노출되지 않으므로 JS 타이머가 Ping 프레임을 보낸다고 설계하지 않는다. 연결 확인과 화면 스크립트의 정상 동작 보장은 별개다. [WebSocket 표준](https://websockets.spec.whatwg.org/#ping-and-pong-frames)
5. 최초 제안값은 Ping 15초, 응답 없음 45초에 해당 연결 만료, 연결 0개가 된 뒤 종료 유예 120초, 초기 연결 대기 120초다. 확정 상수가 아니며 백그라운드 탭·브라우저별 동작·절전 복귀로 조정한다. 절전 복귀 직후 오래된 시각 차이만으로 종료하지 않고 새 탐색·재연결 유예를 준다.
6. 종료 시 새 변경을 받지 않고 진행 중인 저장·복원을 안전하게 완료하거나 롤백한다. JSON 미기록 변경은 재시작 시 복구할 수 있는 내구성 있는 상태로 남긴 뒤 DB를 닫는다. heartbeat 시간 초과가 저장 강제 중단 사유가 되어서는 안 된다.

네트워크 공동 접속과 비밀번호 UI는 추가하지 않는다. 로컬 서버는 loopback에만 노출하고 Host·Origin 확인과 실행 세션 검증으로 외부 웹페이지의 변경 요청·WebSocket 접속을 제한하는 안을 적용 검토한다. 연결 수에는 해당 앱 세션의 연결만 포함한다. API·WebSocket·사진·정적 자산 경로와 SPA fallback을 구분한다.

## DB 호환성·교체·복원 제안

- 새 앱에서 모든 정식 이전 버전 DB를 지원한다. DB 스키마 버전과 순차 마이그레이션을 관리하고 중간 버전을 건너뛴 업그레이드도 검증한다. 아직 첫 버전 DB는 없다.
- 앱 종료 후 실행파일을 수동 교체하고 `db/`를 유지하는 배포 절차를 제안한다. 실행파일 교체만으로 스키마 호환성이 생기는 것은 아니므로 마이그레이션을 별도로 구현한다.
- 마이그레이션 전에 일관된 복구본을 만들고, 검증 실패 시 기존 데이터를 유지한다. 모르는 미래 스키마는 수정하지 않고 열기를 거부한다. 이전 실행파일로 교체하는 것만으로 DB가 되돌아간다고 안내하지 않는다.
- 과거 JSON 기록은 원본을 보존하고 버전별 읽기·변환을 지원하는 안을 제안한다. 과거 상태를 새 앱에 복원할 때도 필요한 스키마 변환을 거쳐 같은 업무 의미를 유지한다. 사진 참조와 기록 버전도 함께 검증한다.
- SQLite와 외부 JSON 파일이 자동으로 하나의 트랜잭션이 되지는 않는다. DB 트랜잭션 안에 업무 변경과 내구성 있는 기록 대기 정보를 함께 남기고 JSON을 순서대로 내보내는 안을 검토한다. 성공 응답 시점·파일 기록 실패·재시도·중복 방지·종료 후 복구 기준을 구현 전에 정한다.
- 이동은 정상 종료 후 `db/` 전체 복사 또는 일관된 내보내기로 검증한다. WAL을 사용할 경우 DB만 복사해 미반영 WAL을 잃지 않도록 한다. SQLite 백업 API만으로 사진·JSON까지 일관된 전체 복원이 해결되는 것은 아니다. [SQLite WAL](https://sqlite.org/wal.html), [Backup API](https://sqlite.org/backup.html)

## 향후 서버 이관과 남은 설계

Rust의 업무 계산·저장·HTTP API를 브라우저 자동 열기와 heartbeat 종료 같은 로컬 실행 정책에서 분리하는 안을 제안한다. 서버 모드에서는 브라우저 종료에 따라 프로세스를 종료하지 않고 상시 실행한다. 배포·접근 제어·인증·다중 사용자·운영 백업은 서버 전환 때 설계하며 현재 앱에 미리 구현하지 않는다. SQLite 유지 또는 DB 교체도 서버 요구에 따라 따로 판단한다.

품목·메뉴·전환 데이터 모델, 단위·정밀도, 원가·정정·지연 입고 보정, JSON 정합성과 사진을 포함한 전체 복원은 여전히 설계 대상이다. 이 ADR은 해당 알고리즘을 확정한 문서가 아니다.

## 검증 상태와 재검토 조건

공식 문서를 확인한 일자는 2026-10-24다. 위 링크는 지원 기능과 제약의 근거이며, 우리 앱의 실행 결과를 대신하지 않는다. 이 ADR 작성 당시 실행 전이었다. 이후 T-001에서 macOS 빌드·heartbeat·같은 OS 이동·합성 스키마 변환을 검증했다. 전체 복원·다른 OS 배포·실제 절전은 미검증이다. 최소 실행 검증과 첫 업무 흐름의 수용 기준은 [T-001](../tickets/T-001-portable-local-app.md)에 연결한다.

별도 런타임 없이는 배포가 안 되거나, heartbeat가 정상 사용 중 종료를 유발하거나, 이전 DB를 손실 없이 변환할 수 없는 사례가 생기면 해당 설계를 재검토한다. Electron 또는 서버 이관 착수는 별도 사용자 요청에 따른다.
