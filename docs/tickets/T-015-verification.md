# T-015 검증 기록

2026-10-25. 상태: **완료, T-015 범위 READY**. [티켓](T-015-photos-and-suppliers.md)과 [계획·Before4](T-015-plan.md)를 따른다. T-014 커밋 `f4a3b09`에서 이어가며 기존 사용자 백로그 변경과 개인 자료를 보존한다.

## 구현과 검증 경계

schema14 / JSON2에 불변 사진 자산과 품목별 사진·구입처 버전 다섯 관계를 추가했다. 재료와 레시피 상세는 같은 품목의 패널을 사용하며 현재 연결을 바꾸어도 과거 자산은 남긴다. 사진이 없는 경우, 여러 사진·대표 한 장, URL 또는 설명만 있는 구입처를 지원한다. 메뉴 복제는 독립 연결과 사진 ID를 만들고 자산 bytes를 공유한다.

허용 형식·파일/본문/치수 상한, 원요청 보존과 파일 재선택, 게시 후 DB 실패의 미등록 파일, Windows 내구성 한계는 계획에 기록했다. 이 티켓의 가상 사진 검증은 T-018 전체 상태 복원이나 T-021 지정 실제 자료 시연을 대신하지 않는다.

## 실제 실패와 수정

- 구현 전 JSON 전송 함수의 FormData 처리는 기존 JSON 시험만 통과하고 신규 파일 시험은 실패했다(1통과/1실패, 종료1). multipart를 그대로 전송하도록 확장했다.
- 변경 전 실행 파일에서 현재/이력 GET 두 경로는404, 정상 사진 POST는405였다. 기존 보호 경계를 포함한 결과는5통과/3실패, 종료1이다.
- 최초 Store 시험은 오래된 frontend 산출물 때문에 빌드 단계에서 차단됐다. 이를 행동 실패로 간주하지 않았다. 표준 빌드를 실제 종료0으로 완료한 뒤 신규 스키마 시험이13과14의 차이로 실패했다(종료101).
- 독립 화면 검토에서 TAB 입력을 허용하는 UI와 거절하는 서버의 차이를 재현했다. UI 입력 검증을 맞춘 뒤 전체 단위112건과 타입 검사가 각각 종료0이었다.
- 첫 통합 표준 빌드는 SQLite 변환이 없는 Rust u64 파일 길이 때문에 두 컴파일 오류로 종료101이었다. SQL i64와 u64 사이의 범위를 검사하는 변환으로 수정했다. 후속 빌드·통합 시험 결과는 아래에 별도로 기록한다.
- 정적 검토에서 source13 DB의 미래 schema14 outbox를 마이그레이션 전에 거절하는 검사, profile 원요청의 기대 revision과 실제 순번 관계, 등록 사진 metadata 손상의503 응답을 보완하고 지속 시험을 추가했다. 이 발견을 실행 전 행동 실패로 표현하지 않는다.
- 첫 전체 Rust 실행은 모든 시험을 컴파일한 뒤277통과/254실패·종료101이었다. 다수 기존 시험에서도 임시 JSON 파일 게시가 샌드박스의 접근 거부(OS5)로 실패해 권한을 분리한 재검증이 필요했다. 별개로 원본 catalog DTO가 알 수 없는 필드를 허용하는데 신규 frozen reader 시험이 거절을 기대한 오류를 확인했다. 당시 정의를 유지하고 시험을 기존 허용 동작·직렬화 시 미채택 및 엄격한 반품 DTO의 거절 확인으로 바로잡았다.
- 첫 신규 브라우저 실행은6통과/1실패·종료1이었다. 메뉴 복제 뒤 원본 사진 교체에서409가 났다. 깨끗한 탭 입력을 복원할 때 최신 profile GET을 받아도 이전 revision을 유지하는 경우를 별도 VM 시험으로 재현했다. 실제 dirty/pending 입력 보존과 구분하여 수정·재검증한다. 화면 보조의 테마 버튼 선택도 실제 접근성 이름과 맞지 않아 첫 화면6장을 두 테마의 근거로 사용하지 않는다.

## 이전 자료 근거

변경 전 커밋의 보존 실행 파일을 새 격리 폴더에서 실행하고 정상 API로 가상 재료·가격·레시피·입고·판매·실사·정정·반품 등의46개 변경을 만들었다. 원요청92회 재생의 응답 bytes, 모든 JSON 원문, SQLite 외래키와 무결성을 확인했으며 수집 명령은 종료0이다. 공개 fixture는 이 자료의 SQL과 마지막 JSON이다. 정상 자료를 기대값 적재나 SQLite 직접 변경으로 만들지 않았다.

첫 수집 시 취소 후 복원 수량에 대한 수집기의 기대가 원요청과 달라 종료1이었다. 원입력과 실제 정정 경로를 확인해 수집기 기대를 고쳤고 제품은 바꾸지 않았다. 실패 자료는 별도로 보존했다.

동결한 schema13 reader의153개 wire DTO는 `f4a3b09861704e46b2a3cc38eef00c5005090635`의 원본 정의와153개 모두 일치한다. 현재 profile/domain 타입 참조는 없다. 새 앱으로의 실제 마이그레이션·원문/원응답 보존 시험은 통합 검증 결과에 기록한다.

## 통합 검증

두 번째 `node scripts/build.mjs`는16개 경로와 release 실행 파일을 생성하고 경로 잔존 검사1건까지 종료0으로 통과했다. 해당 상태의 `cargo fmt --check`와 자산 매니페스트 검사도 종료0이다. 기존 재료·레시피·공통 UI 브라우저18건은2.3분·종료0, 실제 키보드의 파일 선택·대표·구입처·오류 초점·저장·과거/현재 이동도 종료0이었다. 위에서 발견한 화면 문제를 고친 뒤에는 산출물을 다시 생성하고 관련 검증을 이어간다. 수용 기준과 RTD After13은 최종 실행 근거를 확인한 후 판정한다.

## 남은 작업

아래 최종 수용 검증과 RTD 검토 결과를 따른다. Obsidian의 D-09·D-11·D-13 기술 계약은 같은 작업에서 추가하고 재읽기·변경 전 원문 보존을 확인했다. 실제 검증 완료와 구분해 기록했으며 노트를 커밋하지 않는다. 이후 T-016~T-019·T-021을 순서대로 이어간다. T-001 별도 장비·실제 절전 검증과 T-021 지정 실제 자료 시연·배포 준비는 미완료이며 goal은 ACTIVE다.

## 최종 통합 결과

앞의 기록은 각 구현·검증 시점의 상태다. 깨끗한 profile 복원 문제와 실제 테마 선택을 수정한 뒤 세 번째 `node scripts/build.mjs`가16경로·release2분33초·실행 파일 경로 잔존 검사1건을 종료0으로 통과했다. 권한을 분리한 전체 Rust 검증은531통과/0실패/0무시, 종료0이다. 샌드박스에서 실패했던 파일 게시·junction·handle 해제 후 이동·JSON 기록도 안전 검사를 완화하지 않고 통과했다.

| 실제 명령·환경 | 최종 결과 |
| --- | --- |
| `node scripts/build.mjs` | 최신 frontend 생성·release·실행 파일 검사 PASS, 종료0 |
| `cargo test --locked --no-fail-fast` | 전체531 PASS, 종료0. Windows에서 실행 |
| `cargo fmt --check` | PASS, 종료0 |
| `cargo clippy --locked --all-targets -- -D warnings` | PASS, 종료0. 컴파일 캐시 hard link 대신 복사 경고는 별도로 남음 |
| `node scripts/ui-manifest.mjs --check` | 최신 입력·생성 자산 일치, 종료0 |
| frontend `node --experimental-strip-types --test tests/*.test.ts` | `npm test`의 실제 스크립트,114 PASS·종료0 |
| frontend `node node_modules/@nuxt/cli/bin/nuxi.mjs typecheck` | `npm run typecheck`의 CLI, 종료0 |
| frontend Playwright CLI: `catalog.spec.ts`, `recipes.spec.ts`, `ui-foundation.spec.ts` | 기존18 PASS·2.3분·종료0 |
| frontend Playwright CLI: `profiles.spec.ts` | 최초6통과/1실패 뒤 수정한 영향2건 모두 PASS·2.2분·종료0. 나머지5건의 선행 통과와 함께 신규7흐름을 검증 |
| 실제 키보드 보조 | Tab·Enter·Space, 파일/설명/대표/구입처·오류 초점·저장/과거/현재 이동 PASS·종료0 |
| Chromium156.0.8078.4·Node24.19.0·Windows 화면 | 1440/768/390px × 실제 라이트/다크6장 직접 확인. 선택 사진1장·긴 설명/URL·줄바꿈·대비·가로 넘침 검사 PASS |

브라우저 명령은 `node node_modules/@playwright/test/cli.js test`에 위 spec과 `--fully-parallel`을 지정했다. 최초 신규/기존은 `--workers=3`, 마지막 영향2건은 `--grep 'photo replacement and unlink|photo history survives normal restart' --workers=2`였다. 각 실행은 독립 가상 DB와 보존 로그·종료코드를 사용했다. 신규7건을 한 번에 모두 통과한 것으로 합쳐 표현하지 않는다.

첫 Clippy 실행은 동결 reader를 생성하면서 빠진 큰 enum 예외 네 곳 때문에 종료101이었다. 원본에도 있던 inline 표현을 유지하는 이유를 해당 네 enum에 명시했다. wire 정의·직렬화·실행 로직은 바꾸지 않았으며153개 원본 대조와 최종 Clippy가 통과했다.

## 수용 기준별 결과

| 기준 | 실제 확인 | 결과 |
| --- | --- | --- |
| T-015-01 | 사진0/PNG·JPEG 여러 장·대표 변경, 링크만/설명만/여러 구입처, 기존 품목 설명 별도 보존. 재료·레시피 공통 UI의 실제 저장·조회 | PASS |
| T-015-02 | 서버가 생성한 SHA 상대 key·원본 bytes 보존, 원파일명 변경 후 같은 요청, 정상 종료·전체 폴더 이동·재시작 후 과거 파일과 해시 일치 | PASS |
| T-015-03 | 교체·연결 해제·품목 보관 뒤 과거 profile과 자산 유지. 독립 복제는 새 사진 ID와 공유 bytes이며 원본 변경이 복제본을 바꾸지 않음 | PASS |
| T-015-04 | 이름/확장자/MIME/signature·깨진 사진·APNG·바이트/픽셀/part 상한, 잘못된 descriptor·파일 변조·비정규 대상·Windows junction·열린 handle 보호. 실패 시 기존 연결·파일 보존 | PASS |
| T-015-05 | 절대 http/https·host·credentials/control 검사, 위험 URL과 텍스트 검증, 무세션/외부 Host·Origin 거절 및 body 미소비. 미등록 자산404, 등록 손상503, 실제 MIME·길이·HEAD·보안 헤더 | PASS |
| T-015-06 | 파일 게시 뒤 DB/의미검증 실패의 미등록 orphan, JSON 실패 후 같은 UUID 재생·단일 효과, 복제 전체 rollback, 동시 admission·수신 중 종료·수락 후 disconnect 저장 완료. 응답 유실·reload 후 동일 파일 재선택과 원본문 보존 | PASS |

새 파일/HTTP14건과 profile Store14건, 이전 자료 호환3건, frozen reader5건을 전체531건에 포함한다. 이전 source13의46개 변경과 원응답92회 재생을 새 schema14 기록 이후·재시작 이후에도 확인했다. 기존 테이블의 모든 열·행과46개 JSON 원문을 보존하고 새 사진 관계에는 가짜 기록을 만들지 않았다. 미래 schema14 outbox가 source13 DB에 있으면 DDL 전에 거절하며 원DB/파일을 유지했다.

## 변경 파일과 영향

`src/profile.rs`, `src/store/profile.rs`와 validation은 DTO·개정·이력·복제·JSON 관계 검증을 담당한다. `src/photo_assets.rs`와 `src/server.rs`, Cargo manifest/lock은 파일 검증·안전한 게시·제한된 multipart·등록 자산 조회를 담당한다. `src/store.rs`, snapshot과 frozen8~13은 schema14 변환과 이전 reader를 연결한다. 기존 recipe copy에는 같은 트랜잭션의 profile 복제와 원재시도 자산 검사만 추가했다.

frontend 공통 profile 컴포넌트·composable·types/utils와 기존 두 상세·api transport·CSS를 연결했다. 신규 단위/브라우저와 파일/HTTP/Store/호환 시험, 정상 API source13 SQL/JSON fixture를 추가했다. 기존 저장 시험11파일은 현재 스키마 기대14곳만13에서14로 바꾸고 과거 fixture와 미래 거절 기준은 유지했다. README·구조·티켓/계획/검증·색인/TODO는 실제 완료 범위를 반영한다.

## RTD After13

[Step 1] 목적 적합성 - PASS: 여섯 수용 기준을 정상·오류·재시도·이력·복제·이동의 실제 Store/API/UI 결과에 대응했다.

[Step 2] 오류·보안 - PASS: 제한된 decode·본문·admission, no-follow/열린 handle·덮어쓰기 금지·자산 hash, URL/세션/Origin과 원응답을 검증했다. 공개 후보에는 가상 자료만 있으며 개인 파일·링크·계정 경로를 포함하지 않는다.

[Step 3] 회귀 - PASS: 새 시험을 포함한531건 전체와 관련 기존 브라우저18건, 당시 reader153정의·원문/원응답 보존을 확인했다. 운영 규칙과 외부 사용자8경로를 보존한다.

[Step 4] 파일·함수 크기 - PASS: 파일 처리·DTO·Store·의미 검증·공통 UI를 역할별로 분리했다. 큰 동결 reader는 호환성을 위한 당시 정의이며 현재 업무 타입을 참조하지 않는다.

[Step 5] 재사용·통합 - PASS: 기존 mutation·CAS·업무 잠금·JSON 복구·recipe copy·오류 표현·디자인 토큰을 재사용한다. 품목/조리 설명의 별도 정본을 추가하지 않았다.

[Step 6] 부작용 - PASS: 보존 사진과 실패 orphan의 공간 증가는 명시했다. 일반 업무 변경마다 전체 사진 bytes를 읽지 않는다. 외부 호스팅·서버 URL fetch·자동 삭제·감시 서비스를 추가하지 않았다.

[Step 7] 전체 재검토 - PASS: 저장·파일/서버·화면 독립 검토와 전역 연결·새 파일 검토를 대조했다. canonical revision, source13 사전 검사, 손상503과 깨끗한 복원 경계를 보완했다.

[Step 8] 불필요한 잔여물 - PASS: 제품 stub·디버그 로그·개인 fixture를 포함하지 않는다. 실행 자료와 이전 실패는 로컬 제외 경로에 보존한다.

[Step 9] 품질 검증 - PASS: 최신 표준 빌드, 전체531·UI114·타입/포맷/Clippy·자산, 관련 브라우저·키보드·실제6화면을 검증했다. UTF-8/LF·상대 링크·diff·ignore와 혼재 변경의 원문 보존을 확인한다. 각 실패와 종료값을 최종 결과와 구분했다.

[Step 10] 사용자·개발자 흐름 - PASS: 현재/과거·사진 없음·구입처·교체/연결 해제·독립 복제·오류 초점·파일 재선택을 실제 확인했다. 공개 README와 계획·티켓·결과·색인을 연결했다.

[Step 11] 원인과 수정 - PASS: FormData JSON화·제어문자 검증 차이·깨끗한 복원 CAS를 실제 실패로 확인하고 관련 회귀로 수정했다. SQL 타입 컴파일 오류, 샌드박스 권한, 수집기/reader/테마 시험의 기대 오류도 제품 결함과 분리했다.

[Step 12] READY 판단 - READY: T-015 수용 범위는 완료했다. 전체 복원·다른 OS/파일시스템·실제 절전·지정 운영 자료 시연·배포 준비의 완료를 뜻하지 않는다.

[Step 13] 커밋 준비 - PASS: 명시 승인 범위의 공개 변경만 stage 대상이다. status·unstaged/staged diff·각 경로 ignore를 확인하며 TODO/색인은 본 작업 hunk만 적용한다. 외부 사용자 변경·노트·로컬 설정/자료는 제외하고 push·PR·배포하지 않는다.

## 한계와 다음 작업

검증은 현재 Windows와 가상 사진/업무 자료에서 수행했다. Unix 전용 symlink 시험, 다른 OS·파일시스템의 파일 게시와 directory sync, 전원 손실 내구성은 검증하지 않았다. hard link 미지원 환경은 기존 파일을 덮지 않고 저장을 거절한다. 이 한계는 T-001·T-019·T-021의 실제 환경 검증과 분리한다.

source13 원요청92회 재생의 debug 호환 시험은154.88초였다. 이를 release 사용자 작업 응답 시간으로 해석하지 않는다. JSON 원문과 의미 검증의 누적 비용은 T-021에서 release로 별도 측정한다. 전체 복원은 T-018, 정식 이전 버전별 호환은 T-019에서 이어진다.

Obsidian 연결 노트의 D-09·D-11·D-13 대응 절과 변경 이력을 추가하고 양쪽 내용을 재읽었다. 기존 노트의 메타데이터·링크·원자료·분석·사용자 백로그와 모든 기존 bytes를 보존했으며 이 로컬 노트는 커밋하지 않는다. 현재 추가 사용자 운영 결정은 필요하지 않다.

다음은 T-016 판매 통계·실사 소모·원가 이력이다. T-017~T-019·T-021도 순서대로 이어간다. T-001 필수 외부 환경 검증과 T-021 지정 실제 자료 시연·배포 준비는 미완료이며 goal은 ACTIVE다.
