use super::*;

pub(in crate::store) fn migrate_restore(c: &Connection) -> Result<(), StoreError> {
    let journals = c
        .prepare("SELECT sequence,payload FROM snapshot_outbox ORDER BY sequence")
        .map_err(unavailable)?
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
        .map_err(unavailable)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(unavailable)?;
    for (seq, payload) in journals {
        snapshot::validate_source(&payload, seq, 14)?;
    }
    c.execute_batch("ALTER TABLE snapshot_outbox RENAME TO state_sequences;
        CREATE TABLE snapshot_outbox(sequence INTEGER PRIMARY KEY NOT NULL REFERENCES state_sequences(sequence),payload TEXT NOT NULL,published INTEGER NOT NULL CHECK(published IN(0,1)));
        INSERT INTO snapshot_outbox(sequence,payload,published) SELECT sequence,payload,published FROM state_sequences;
        ALTER TABLE state_sequences DROP COLUMN payload;
        ALTER TABLE state_sequences DROP COLUMN published;
        CREATE TABLE preserved_mutation_requests(request_id TEXT PRIMARY KEY NOT NULL,operation TEXT NOT NULL,payload TEXT NOT NULL,response TEXT NOT NULL,epoch TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES state_sequences(sequence));
        CREATE TABLE preserved_creation_requests(request_id TEXT PRIMARY KEY NOT NULL,name TEXT NOT NULL,unit TEXT NOT NULL,material_json TEXT NOT NULL,epoch TEXT NOT NULL,sequence INTEGER REFERENCES state_sequences(sequence));
        CREATE TABLE restore_records(request_id TEXT PRIMARY KEY NOT NULL REFERENCES preserved_mutation_requests(request_id),previous_epoch TEXT NOT NULL,result_json TEXT NOT NULL);
        CREATE TABLE preserved_photo_assets(sha256 TEXT PRIMARY KEY NOT NULL,descriptor_json TEXT NOT NULL);
        CREATE TABLE restore_imports(sha256 TEXT PRIMARY KEY NOT NULL,byte_length INTEGER NOT NULL CHECK(byte_length>0),target_json TEXT NOT NULL);") .map_err(unavailable)?;
    preserve_active_requests(c)?;
    for row in super::super::profile::snapshot_rows(c)?.assets {
        preserve_asset(c, &row.descriptor)?;
    }
    Ok(())
}

pub(in crate::store) fn validate_restore_schema(c: &Connection) -> Result<(), StoreError> {
    c.prepare("SELECT sequence FROM state_sequences LIMIT 0")
        .map_err(unavailable)?;
    c.prepare("SELECT sha256,byte_length,target_json FROM restore_imports LIMIT 0")
        .map_err(unavailable)?;
    let state = read_preserved_state(c)?;
    for row in &state.mutation_requests {
        if row
            .sequence
            .parse::<i64>()
            .ok()
            .is_none_or(|v| v <= 0 || v.to_string() != row.sequence)
        {
            return Err(unavailable("보존 요청 순번이 올바르지 않습니다"));
        }
    }
    snapshot::validate_current_preserved(c)?;
    Ok(())
}
