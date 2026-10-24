# T-015 구현 계획과 변경 전 검토

2026-10-25. 상태: **설계·구현·수용 검증과 RTD 완료**. 실제 근거는 [검증 기록](T-015-verification.md)을 따른다. T-014 커밋 `f4a3b09` 이후 [T-015](T-015-photos-and-suppliers.md)의01~06과 [요구사항](../REQUIREMENTS.md), [계산·데이터 계약](../DOMAIN-CONTRACT.md) §10, [ADR-007](../decisions/ADR-007-consumption-and-ticket-scope.md) P-12를 적용한다. RTD Before와 PM test-scenarios, design/frontend-design, security 시큐어코딩, Obsidian repository 경로로 검토했다. 새 운영 정책을 선택하는 작업이 아니다.

## 범위와 화면

재료와 레시피 상세에서 같은 품목 ID의 사진·구입처 패널을 사용한다. 여러 사진의 대표1장·설명·연결 해제, 링크 또는 설명만 있는 구입처를 개별 저장한다. 품목의 간단한 설명은 기존 MaterialVersion.description, 조리 설명은 기존 RecipeVersion에 둔다. 이 저장과 사진·구입처 저장의 단위를 명확히 구분한다. 사진 없는 기존 품목/메뉴 등록은 그대로 지원한다.

현재 자료와 불변 profile 이력을 별도 시각/버전으로 조회한다. 현재 사진을 과거 레시피 저장 당시 사진인 것처럼 표시하지 않는다. 대표 사진을 연결 해제하면 남은 첫 사진을 화면에서 새 대표로 표시하며 저장 전에 바꿀 수 있다. 사진0장은 대표 없음이다. 큰 원본을 동시에 모두 디코딩하지 않도록 선택한 사진만 크게 표시하고 나머지는 설명/선택 목록으로 제공한다. 원본을 임의 재압축·메타데이터 제거하지 않는다.

신규 파일은 한 번에1개를 등록해 여러 장을 누적한다. 다중 업로드 큐·외부 사진 호스팅·이미지 생성·서버 URL 미리보기·발주 전송·파일 삭제/GC·전체 원자료 이관은 추가하지 않는다. 전체 복원 writer는 T-018, 공식 이전 버전별 통합 호환은 T-019에서 검증한다.

## 저장 계약

schema14 / JSON format2에 다음5관계를 추가한다. 직전 schema13 reader는 당시 DTO와 수용 규칙을 고정해 현재 업무 타입 참조 없이 동결하고 정상 API로 만든 이전 자료의 ID·열·원요청 응답·JSON 원문을 보존한다. 원래 catalog DTO의 알 수 없는 필드 허용 동작도 유지하되 새 profile 관계와 미래 스키마는 이전 reader가 받아들이지 않는다. 이전 자료에는 가짜 profile/seq를 생성하지 않는다.

| 관계 | 보존할 근거 |
| --- | --- |
| photo_assets | 실제 SHA256·서버 생성 상대 key·검증 형식·길이·치수·최초 seq/시각 |
| material_profile_versions | 품목/당시 품목 버전·이전 또는 복제 출처 profile·seq/시각 |
| material_profile_heads | 품목별 현재 profile 버전 |
| material_profile_photos | profile 내 사진 ID·순서·자산 참조·caption·대표 여부 |
| material_profile_suppliers | profile 내 순서·선택 URL·설명 |

개정은 사진/구입처의 전체 목록을 새 불변 profile로 저장한다. 사진 교체·연결 해제·품목 보관 후에도 과거 profile·자산 bytes를 삭제하지 않는다. 기존 사진 ID는 기대 profile 소속을 검증하며 다른 품목 ID로 임의 연결하지 않는다. 레시피 복제는 기존 recipe.copy 트랜잭션 안에서 새 소유 품목의 독립 profile/사진 ID를 만들고 자산 bytes만 공유한다. 이후 원본 수정은 복제본을 바꾸지 않으며 원 copy 요청 재생은 원결과를 유지한다.

새 파일은 검증→같은 디렉터리 임시 기록·sync→no-clobber 게시→DB profile/자산/원응답/snapshot 트랜잭션→기존 JSON 발행 순서다. 파일시스템과 SQLite의 단일 ACID를 주장하지 않는다. 게시 후 DB 실패의 미등록 파일은 남을 수 있지만 API에서 조회되지 않고 자동 삭제하지 않는다. JSON 실패는 저장 여부 미확인으로 처리하며 원 UUID를 재시도한다.

원요청의 정규 identity는 품목 ID·정규 metadata·서버가 실제 bytes에서 검증한 자산 descriptor다. 파일명·multipart boundary·part 순서는 제외한다. 동일 UUID/본문/파일은 정확한 원응답을 재생하고 다른 사진·caption·대표·구입처는 충돌이다. 원요청 조회는 CAS보다 먼저이며 과거 기대 profile을 현재 head로 바꾸어 재시도하지 않는다. 성공/원응답 재생에 필요한 실제 자산도 검사한다.

## 파일·HTTP·자원 경계

JPEG/PNG만 지원한다. 실제 signature와 전체 decode를 확인하며 APNG·HTML/SVG·깨진/빈 파일은 거절한다. 파일≤10MiB, 각 변1~8192px, 총20,000,000px, decoder max_alloc128MiB를 적용한다. max_alloc은 best effort이며 프로세스 메모리의 엄격한 상한으로 설명하지 않는다. 후보 라이브러리는 image0.25.10의 JPEG/PNG 기능만 활성화한다. 기존 axum0.8.9에 multipart 기능, 명시 URL parser를 사용한다.

새 POST만 전체 body11MiB·metadata256KiB·photo 최대1개로 제한한다. metadata의 사진목록은20개·구입처20개, caption500자·URL2048자·구입처 설명2000자이며 전체 바이트 상한도 함께 적용한다. multipart 끝 뒤의 잔여 바이트도 포함하도록 제한된 전체 body를 수신한 뒤 각 part를 chunk 단위로 검사한다. 이 과정의 인코딩된 자료 복사본은 약21MiB까지이며 decoder 할당은 별도다. 알 수 없는/중복 part·잘못된 필드·파일 참조 불일치를 거절한다. URL은 절대 http/https와 host만 허용하고 credentials·제어문자·실행/파일 스킴을 거절한다. 외부 주소를 서버에서 가져오지 않으며 텍스트와 안전한 링크로 표시한다.

기존 Host/Origin/session guard와 종료 중 거절을 유지한다. body 수신 전에 단일 업로드 admission을 확보하고 동시 업로드는 즉시503을 반환한다. body 수신만60초 및 종료 신호로 제한하고, 수락된 decode/게시/DB·JSON 저장은 기존 with_store의 writer 잠금과 종료 재검사 뒤 끝낸다. HTTP 연결 종료를 미반영의 증거로 사용하지 않는다.

자산은 db/assets/photos 아래 SHA 기반 고유명칭이다. 원파일명/절대경로를 저장값·경로·응답 오류에 넣지 않는다. 경로 이탈·symlink/junction/reparse와 비정규 파일을 거절하고 열린 handle과 실제 bytes/hash를 검사한다. cap-std4.0.3·cap-fs-ext4.0.3의 디렉터리 handle과 no-follow 접근을 사용하며 검사한 각 디렉터리 handle을 작업 끝까지 유지한다. 임시 파일 sync 뒤 hard link로 덮어쓰기 없이 게시하고 실제 bytes를 다시 검사한다. hard link를 지원하지 않는 파일시스템에서는 저장을 거절하며 덮어쓰기 대안으로 전환하지 않는다. 문자열 prefix나 canonicalize 한 번으로 동시 로컬 경로 교체까지 안전하다고 주장하지 않는다. 기존 파일은 덮지 않으며 잘못된 동명 파일을 지우지 않는다. Windows directory sync의 내구성 한계와 다른 OS의 실제 미검증을 기록한다.

| 경로 | 계약 |
| --- | --- |
| GET /api/materials/{id}/profile | 현재 profile·현재 품목 버전·state, 미등록 profile은 정상 빈 상태 |
| GET /api/materials/{id}/profiles | 불변 profile 버전 요약 목록 |
| GET /api/materials/{id}/profiles/{version_id} | 해당 품목의 과거 profile |
| POST /api/materials/{id}/profiles | strict metadata1개+선택 photo1개의 multipart 개정 |
| GET /api/photo-assets/{sha256} | DB에 등록된 digest만 조회, 검증 MIME·길이·nosniff/no-store·CORP 적용 |

metadata는 meta(request_id/epoch/expected_revision), expected_material_version_id, expected_profile_version_id, photos, primary_position, suppliers다. 사진 입력은 source=existing(photo_id/caption) 또는 upload(caption)이며 upload는 최대1개다. 사진0장은 primary_position=null, 그 외 유효한 사진 위치 하나다. suppliers는 선택 url과 description이며 둘 다 비면 오류다. 세부 Rust/TS 이름은 이 동일 계약 안에서 맞춘다.

## 작성·재시도와 복원 선행

브라우저는 pending에 UUID·metadata·파일 SHA/길이/형식을 보존한다. File/Blob/object URL은 sessionStorage JSON에 넣지 않는다. 같은 탭 메모리에 파일이 없으면 동일 파일을 다시 선택하고 hash/길이를 대조한 뒤 같은 요청을 재시도한다. 늦은 hash·서버 응답은 품목/선택 generation을 검사한다. 미확인 전송을 다른 파일이나 새 UUID로 바꾸지 않는다. 저장 보관 실패는 전송 전에 안내하고, stale은 최신 정보와 원입력을 구분해 명시 검토한다.

시작 검사에서는 전체 이력의 고유 자산을 중복 제거해 존재·길이·hash를 확인한다. 일반 판매·실사 mutation마다 모든 사진을 반복 읽는 경로는 만들지 않는다. JSON은 자산 metadata와 상대 참조만 담으며 사진 bytes를 재생성하지 않는다. T-018은 같은 검증 경계를 재사용하되 현재 티켓에서 전체 복원 완료로 표시하지 않는다.

## 검증·파일·되돌림

가상 JPEG/PNG만 사용해 현재 미지원 경로의 행동 RED와 신규 저장·파일 검증 실패를 먼저 확인한다. 이후 사진0/여러장/대표·URL만/설명만·교체/연결 해제/보관·복제 독립성·원요청 재생·재시작·과거 참조·같은 OS db전체 이동, 악성 이름/URL/형식/픽셀/용량/중복part, 파일·DB·JSON 실패와 실제 기존 자료 보존을 Store/HTTP/UI로 검증한다. 직접 SQLite 변경으로 정상 업무 자료를 만들지 않는다.

예상 파일은 profile DTO·Store/스키마/snapshot·photo_assets·server/Cargo, 공통 profile UI와 재료/레시피 상세, 지속 시험/가상 fixture 및 문서다. frontend 변경 뒤 node scripts/build.mjs를 먼저 수행하고 cargo test --locked·fmt·clippy, UI 단위/typecheck·관련 기존/새 브라우저, 키보드/세 너비 두 테마와 RTD After13을 검증한다. 실제 명령/실패/미검증은 검증 기록에 남긴다.

되돌림은 변경 전 실행 파일·db 전체 복사본을 보존하고 가상 환경에서 검증한다. schema14를 schema13으로 임의 하향하지 않으며 미래 스키마를 기존 바이너리에 쓰게 하지 않는다. 기존 사용자 변경·개인 자료·로컬 설정은 제외한다. 데이터 구조/사진 보관 계약은 같은 작업에서 연결 기획 노트에 날짜·D-09·D-11·D-13으로 반영하고 접근 실패면 미동기화로 기록한다.

## RTD Before

[Step 1] 변경 계획 - PASS: 여섯 기준·저장 단위·파일/DB/JSON 실패·공유 DTO·검증과 되돌림을 정했다.

[Step 2] 계획 검토 - PASS: 저장·보안·화면 독립 사전 검토를 대조했다. 기존 설명/recipe.copy/mutate/guard를 재사용하고 외부 사용자8경로를 별도 보존한다. Obsidian 실제 볼트와 연결 노트를 확인했다.

[Step 3] 검토 재검토 - PASS: 파일/DB는 단일 원자 저장이 아님, 브라우저 JSON은 파일 bytes를 보존하지 못함, 원본 decoder 메모리/파일 게시의 OS 보장은 제한됨의 세 가정을 반영했다. 미확인 요청·과거 참조를 조용히 복구 완료로 바꾸지 않는다.

[Step 4] 계획 단순화 - PASS: 별도 설명 정본·업로드 세션·멀티파일 큐·외부 저장소·GC·전체 복원 writer 없이 기존 품목과 불변 자산/연결만 확장한다.
