use super::{Store, StoreError, catalog, restore, snapshot, unavailable};
use crate::catalog::{CatalogState, MutationMeta};
use chrono::{SecondsFormat, Utc};
use rusqlite::{Transaction, params};
use serde::{Serialize, de::DeserializeOwned};

impl Store {
    pub(super) fn mutate<R: Serialize + DeserializeOwned>(
        &mut self,
        meta: Option<&MutationMeta>,
        request_id: &str,
        operation: &str,
        payload: &str,
        apply: impl FnOnce(&Transaction<'_>, i64, &str, &CatalogState) -> Result<R, StoreError>,
    ) -> Result<R, StoreError> {
        if let Some(prior) = restore::preserved_request(&self.connection, request_id)? {
            if prior.operation != operation || prior.payload != payload {
                return Err(StoreError::Conflict(
                    "같은 요청 ID에 다른 입력을 사용할 수 없습니다".into(),
                ));
            }
            self.recover()?;
            let state = catalog::state(&self.connection)?;
            if prior.epoch != state.epoch {
                return Err(restore::generation(state, true));
            }
            return serde_json::from_str(&prior.response).map_err(unavailable);
        }
        let legacy: bool = self
            .connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM preserved_creation_requests WHERE request_id=?1)",
                [request_id],
                |row| row.get(0),
            )
            .map_err(unavailable)?;
        if legacy {
            return Err(StoreError::Conflict(
                "기존 등록 요청 ID를 새 명령에 사용할 수 없습니다".into(),
            ));
        }
        self.recover()?;
        let tx = self.connection.transaction().map_err(unavailable)?;
        let current = catalog::state(&tx)?;
        if meta.is_some_and(|m| m.epoch != current.epoch)
            || (meta.is_none() && restore::has_restored(&tx)?)
        {
            return Err(restore::generation(current, false));
        }
        if meta.is_some_and(|m| m.expected_revision != current.revision) {
            return Err(StoreError::StaleState(
                "자료가 변경되었습니다. 최신 상태를 확인한 뒤 다시 저장하세요".into(),
            ));
        }
        tx.execute("INSERT INTO state_sequences DEFAULT VALUES", [])
            .map_err(unavailable)?;
        let sequence = tx.last_insert_rowid();
        tx.execute(
            "INSERT INTO snapshot_outbox(sequence,payload,published) VALUES(?1,'',0)",
            [sequence],
        )
        .map_err(unavailable)?;
        let recorded_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let state = CatalogState {
            epoch: current.epoch,
            revision: sequence.to_string(),
        };
        let response = apply(&tx, sequence, &recorded_at, &state)?;
        tx.execute(
            "UPDATE catalog_state SET revision=?1 WHERE singleton=1",
            [sequence],
        )
        .map_err(unavailable)?;
        tx.execute("INSERT INTO mutation_requests(request_id,operation,payload,response,epoch,sequence) VALUES(?1,?2,?3,?4,?5,?6)",
            params![request_id,operation,payload,serde_json::to_string(&response).map_err(unavailable)?,state.epoch,sequence]).map_err(unavailable)?;
        restore::preserve_active_requests(&tx)?;
        let snapshot_json = snapshot::encode(&tx, sequence, request_id, &recorded_at)?;
        // Semantic evidence belongs to the same transaction as its rows. A
        // bad projection must roll back; recovery only retries publication of
        // an already validated snapshot after commit.
        snapshot::validate(&snapshot_json, sequence)?;
        tx.execute(
            "UPDATE snapshot_outbox SET payload=?1 WHERE sequence=?2",
            params![snapshot_json, sequence],
        )
        .map_err(unavailable)?;
        tx.commit().map_err(unavailable)?;
        self.recover().map_err(|error|StoreError::Unavailable(format!("자료는 DB에 저장되었지만 JSON 기록을 완료하지 못했습니다. 같은 요청으로 다시 시도하세요: {error}")))?;
        Ok(response)
    }
}
