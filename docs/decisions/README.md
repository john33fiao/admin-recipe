# 결정 기록

첫 버전 범위, 데이터 정책, 기술 스택, 호스팅, 라이선스처럼 이후 작업의 기준이 되는 결정을 기록한다.

[요구사항과 남은 기술 설계](../REQUIREMENTS.md)와 [진행 순서](../TODO.md)를 함께 확인한다.

| 기록 | 상태 | 결정일 |
| --- | --- | --- |
| [ADR-001: MIT 라이선스 채택](ADR-001-mit-license.md) | 채택 | 2026-10-24 |
| [ADR-002: 개발 단계와 로컬 저장·배포 방향](ADR-002-first-release-and-local-storage.md) | 일부 ADR-003으로 대체 | 2026-10-24 |
| [ADR-003: 입력 시점별 재고·원가와 이동 가능한 데이터](ADR-003-inventory-events-and-portable-data.md) | 채택 | 2026-10-24 |
| [ADR-004: 재고 운영 예외 네 가지 확정](ADR-004-inventory-exception-policies.md) | 채택 | 2026-10-24 |
| [ADR-005: Rust·Nuxt 로컬 서버와 DB 하위호환성](ADR-005-rust-nuxt-local-server.md) | 채택, 세부 구현은 ADR-006·T-001 참조 | 2026-10-24 |
| [ADR-006: 로컬 실행·재료 저장·JSON 기록 구현](ADR-006-local-runtime-and-journal.md) | 채택, OS별 검증 일부 미완료 | 2026-10-24 |
| [ADR-007: 실사로 소모 확정·판매 개수 배분과 원가 정책](ADR-007-consumption-and-ticket-scope.md) | 채택, ADR-003·ADR-004의 일부 운영 규칙 대체 | 2026-10-24 |
| [ADR-009: 전체 상태 복원과 보존 요청 원장](ADR-009-full-state-restore-and-replay.md) | 채택 — T-018 구현·검증 완료 | 2026-10-25 |

새 기록은 다음 번호의 `ADR-NNN-짧은-제목.md` 형식을 사용하고 다음을 담는다.

- 상태와 결정일
- 문제와 제약
- 검토한 선택지
- 선택과 근거
- 영향과 되돌리는 조건
- 관련 요구사항 ID·티켓

제안과 채택을 구분한다. 기존 결정을 변경하면 이전 기록을 삭제하지 않고 대체 관계를 남긴다.
