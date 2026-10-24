# T-013 검증 기록

2026-10-25. 상태: **READY — 설계·구현·여섯 수용 기준과 RTD 완료**. [티켓](T-013-purchase-returns.md)과 [계획·Before4](T-013-plan.md)를 따른다. 자료 검증이나 이전 버전 fixture 준비를 지정 실제 시연의 성공으로 간주하지 않는다.

## 환경과 실행 결과

Windows 개발 PC, Node24.19·npm12.2.1·Python3.12.14·Rust/Cargo1.99·MSVC19.41·Windows SDK10.0.22621, Playwright1.64.1·Chromium156.0.8078.4에서 가상 자료와 격리 DB를 사용했다. 제품 파일을 동결하고 frontend 시험 변경 뒤에도 표준 빌드를 다시 수행했다.

| 실제 명령·범위 | 최종 결과 |
| --- | --- |
| `node scripts/build.mjs` | 최종 종료0, UI 생성·release 빌드와 개인 경로 잔존 검사1건 통과 |
| `cargo test --locked --no-fail-fast` | 전체 488건, 49개 결과 집합, 실패0·종료0 |
| `cargo fmt --check` | 종료0 |
| `cargo clippy --locked --all-targets -- -D warnings` | 종료0 |
| `npm --prefix frontend test` | 90/90·종료0 |
| `npm --prefix frontend run typecheck` | 종료0 |
| `node scripts/ui-manifest.mjs --check` | 종료0 |
| 기존12개 spec의 Chromium 회귀 | 80/80·종료0 |
| 신규 `purchase-returns.spec.ts` | 첫7개 중5통과/2실패, 수정한2개 재실행 통과. 7개 각각 검증 |
| 실제 키보드·픽셀 | Tab/Enter/Space·오류/미리보기 초점·메모 줄바꿈, 상세6장과390px 입력/결과2장 통과 |
| 파일·문서 | UTF-8/LF·상대 링크·diff-check·제외 규칙·공개 자료 검사 통과 |

기존 브라우저 실행은 `npm --prefix frontend run test:e2e -- e2e/business.spec.ts e2e/catalog.spec.ts e2e/local-app.spec.ts e2e/orders.spec.ts e2e/productions.spec.ts e2e/receipts.spec.ts e2e/recipes.spec.ts e2e/reconciliations.spec.ts e2e/sales.spec.ts e2e/stocktakes.spec.ts e2e/ui-foundation.spec.ts e2e/corrections.spec.ts --fully-parallel --workers=3`에 로컬 결과 디렉터리를 지정했다. 새7개도 같은 스크립트에 `e2e/purchase-returns.spec.ts --fully-parallel --workers=3`, 영향 재실행은 `--grep 'return replace void|late source responses' --workers=2`를 사용했다. 최초 로그·trace와 최종 결과는 로컬 전용 자료로 보존한다. 총 브라우저87개 각각의 통과이며 한 번의87개 무실패 실행으로 표현하지 않는다.

## 수용 기준별 결과

| 기준 | 실행 근거와 확인한 결과 |
| --- | --- |
|01|Store·HTTP·브라우저에서 원입고 수량/금액/단위와 실제 유효 source를 구별했다. 원입고 R−유효 반품 합과 현재 Q를 별도 검사하고 R3/I3/U0인 포함 입고도 실제 R로 한도를 판단했다. 원입고보다 많은 반환을 차단했다.|
|02|순수 계산·Store·실제 화면에서 현재 혼합 평균을 사용했다. 알려진0원은 허용하고 원구입 금액·마지막 단가로 현재0원을 대체하지 않았다.|
|03|가상 원구입 참고30000과 현재2kg/20000에서1kg 반품→차감10000, 잔량1kg/10000을 화면에서 구별했다. 세 너비와 두 테마에서도 각각 표시된다.|
|04|기존 주문 취소 회귀로 미입고 취소가 주문 상태만 바꿈을 유지했다. 실제 반품 이후 다음 실사에서 반품을 소모로 다시 배분하지 않았다. 지연 금액 보정→새 반품→다음 실사 연계도 통과하여 T-010-02를 완료했다.|
|05|혼합3kg/40000에서 부분13333.333333·전량잔여26666.666667를 차감해 최종Q/V0, 마지막 유효 단가를 보존했다. 가치 미확인·현재 수량 부족·타차원·정밀도 초과는 입력을 보존하며 확인을 안내했다.|
|06|반품 대체/취소/복원은 원반품과 연결되고 원기록·원응답을 유지한다. create/confirm 응답 유실·JSON 발행 실패·재시작 뒤 같은 UUID는 한 번만 적용된다. source 금액/단위 정정, source 취소, 유효 한도 침범과 최신 복원 템플릿을 검증했다.|

## 저장·통합 근거

T-012 최종 커밋 `eb50b1d`의 release를 별도 복사하여 정상 API34개 변경으로 schema12 자료를 만들었다. 원응답34개를 재시작 전후 각1회, 총68회 바이트 대조했고 JSON34개 바이트, 읽기 전용 FK 검사0건·integrity `ok`를 확인했다. 이 자료를 schema13으로 이관한 시험은 원컬럼·원요청·원응답·JSON을 보존한다. 성공 근거를 만들기 위한 SQL 수정은 하지 않았다.

| 공개 가상 fixture | SHA256 |
| --- | --- |
| `tests/fixtures/schema-v12.sql` | `346c96dee8e27ee4761f0c5eb5a9d3bfedf6b98b8a72a3208c9030b807f0b4d7` |
| `tests/fixtures/snapshot-v2-schema12.json` | `1cee137ca91ac2dc716abb95f1699635f57a30278e8a8f50b92005792db1aab1` |

frozen12의149개 DTO를 이전 커밋과 정적 대조했고 닫힌 reader5개 시험으로 새 반품 태그/관계가 과거 형식에 스며들지 않음을 확인했다. 변경 전 schema8~11 reader도 원문을 유지하며 검증 view에서만 빈 반품 컬렉션을 사용한다. 물리 컬럼·효과 JSON·원확정/원입고·품목·seq의 관계 검증과 전체 트랜잭션 롤백을 확인했다.

통합 Store4건은 반품 취소→원입고 금액 정정→최신 source로 복원, 취소 source의 null 복원 템플릿, 원입고의 품목/단위 정정 뒤 원단위 보존, 지연 금액 보정 전후 반품·다음 실사를 다룬다. 마지막 가상 사례에서 과거 반품4의 차감액을 유지하며 지연 D3을 현재1.8/별도1.2로 나누고 다음 반품1은 현재 평균1.3을 사용한다. 다음 실사 소모1/원가1.3, 잔량4/가치5.2로 보존한다.

## 최초 실패와 해결

- 구현 전 정상 입고를 만든 뒤 반품 source/list는 기대200에 실제404, 새 업무 kind는 기대201에400이었다. UI 단위5개 중3개도 새 태그·미완성/null·불확실 원요청 복원의 실제 assertion에서 실패했다. 컴파일 오류를 행동 실패로 사용하지 않았다.
- 캡처 스크립트의 첫 실패는 재료 GET 응답 형태 오판, 다음 실패는 파생 event head 수 기대3/실제5였다. 스크립트 기대를 바로잡았으며 두 번째 DB는 읽기 전용으로 최종 내보냈다. 제품·업무 DB를 수작업 변경하지 않았다.
- 첫 선택 Rust 명령은139통과/4실패·종료101이었다. 기존 lib 시험이 현재 schema13 fixture를 만들면서 검증 상한을12로 고정한 원인이었다. 시험이 현재 `SCHEMA_VERSION`을 쓰도록 수정한 뒤 전체 회귀가 통과했다.
- 첫 신규 브라우저는5통과/2실패·종료1이었다. source 개정 뒤 보존된 오래된 초안의 정상409에서 명시 재검토 단계를 생략한 시험, 지연 route handler 완료 전에 unroute한 시험이었다. 두 시험만 수정하고 표준 빌드를 다시 수행한 뒤 영향2건이 통과했다. 제품 수정·시간 한도 완화로 숨기지 않았다.
- 정적 검토에서 source 한도 침범 메시지에 관련 반품/source 식별자와 요청량·한도를 명시하도록 보완했다. 해당 수치를 요구하는 순수 회귀와 전체 검증이 통과했다.

## 변경 파일과 후속 경계

`src/purchase_return.rs`·`src/store/purchase_return.rs`가 원입고 조회·정규화·평균 차감·원기록 저장과 검증을 담당한다. business/server/holding, correction·reconciliation의 기존 재생·검증 경로를 연결했다. schema/snapshot과 frozen12는 이전 형식을 보존한다. Nuxt의 반품 입력·결과·목록/상세·source 조회와 공통 업무/정정/보유 표시, 관련 Rust·UI 시험과 가상 fixture를 추가했다. README·ARCHITECTURE·티켓·진행 색인을 실제 결과로 맞췄다.

연결 노트에는 C-02·D-04·D-08 및 ADR-007 P-10의 현재 평균 반품·원입고 한도·실사/지연 금액·정정과 schema13 기술 계약을 대응했고 양쪽을 재확인했다. 기존 메타데이터·자료·링크와 사용자 백로그 변경은 보존한다. 원자료와 로컬 설정·증거는 Git 제외 경로에 남기며 공개 코드·시험은 가상 자료만 사용한다.

## RTD After13

[Step 1] 목적 적합성 - PASS: 여섯 기준과 T-010 반품 연결을 실제 순수/저장/HTTP/UI 결과로 대조했다.

[Step 2] 오류·중요 문제·보안 - PASS: 엄격한 DTO·수치 범위·관계 검증과 SQL 바인딩을 유지한다. 기존16KiB·세션/Host/Origin·라이프사이클 보호를 재사용한다. 공개 후보에서 개인 경로·원자료·비공개 링크를 검사했다.

[Step 3] 회귀 - PASS: 전체 Rust·기존80개 브라우저·UI90과 과거 schema12 원문 이관이 통과했다. 외부 사용자8개 경로의 변경을 보존하며 혼합 색인은 본 작업 조각만 분리한다.

[Step 4] 파일·함수 크기 - PASS: 새 도메인/저장/source 조회/입력/결과 역할을 나눴다. frozen12와 fixture의 크기는 기존 자료 보존용이며 무관한 리팩터링을 추가하지 않았다.

[Step 5] 재사용·통합 - PASS: 공통 업무/정정/outbox·보유·가치/출처 재생을 사용한다. 별도 환급 계산이나 새로운 반품 정정 엔진을 만들지 않았다.

[Step 6] 부작용 - PASS: schema13은 반품 관계와 nullable 원천 참조를 추가한다. 새 의존성·전역 설정·외부 서비스·백그라운드 작업은 없다. 격리 시험 자료만 생성했다.

[Step 7] 전체 변경 재검토 - PASS: 저장·계산/API·UI의 독립 검토와 root 통합 검토, 새 파일/최종 diff·이전 DTO 대조를 수행했다.

[Step 8] 불필요한 잔여물 - PASS: 임시 helper·실패 로그·시각 근거는 로컬 전용이다. 제품 stub/디버그 자료와 오래된 완료 설명을 확인했다.

[Step 9] 품질 검증 - PASS: 위 실제 명령·정적/문서/파일 검사를 수행했다. 최초 실패와 수정 후 통과를 구별한다.

[Step 10] 사용자·개발자 흐름 - PASS: 원입고→초안/복구→미리보기/확정→상세/정정과 연속 작성, 오류 초점·키보드·세 너비/두 테마를 실제 확인했다.

[Step 11] 발견 문제의 원인 - PASS: 시험의 schema 상한·stale 검토 누락·비동기 해제 순서와 제품 오류 안내 보완을 분리하고 각 영향 재검증을 완료했다.

[Step 12] READY 판단 - READY: T-013과 T-010 후속 반품 기준을 충족했다. T-014 이후·T-001 외부 환경·T-021 실제 시연을 완료했다는 뜻은 아니다.

[Step 13] 요청된 커밋·PR 준비 - PASS: 사용자의 명시 승인 범위에서 실제 diff·기존 staged 여부·ignore 규칙을 확인하고 검증한 공개 경로만 분리한다. 기존 사용자 변경·개인 자료·로컬 설정을 제외한다. push·PR·배포는 수행하지 않는다.

다음 작업은 T-014 통합 작업 화면이다. 개발 도구 없는 별도 Windows/Mac·실제 절전 복귀, T-021 지정 새 DB 시연과 배포 준비는 미검증이며 goal은 계속 ACTIVE다.
