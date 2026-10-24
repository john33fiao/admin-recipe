use anyhow::{Context, bail};
use chrono::{SecondsFormat, Utc};
use fs2::FileExt;
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
    time::Duration,
};
use tempfile::NamedTempFile;
use uuid::Uuid;

const SCHEMA_VERSION: i64 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Material {
    pub id: String,
    pub name: String,
    pub unit: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateMaterial {
    pub request_id: String,
    pub name: String,
    pub unit: String,
}

#[derive(Debug)]
pub enum StoreError {
    Validation(String),
    Conflict(String),
    Unavailable(String),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation(message) | Self::Conflict(message) | Self::Unavailable(message) => {
                formatter.write_str(message)
            }
        }
    }
}
impl std::error::Error for StoreError {}

/// SQLite is authoritative. Every mutation stores an immutable, complete JSON
/// payload in the same transaction; publication can therefore resume after exit.
pub struct Store {
    connection: Connection,
    root: PathBuf,
    // Keep the lock file open for the entire connection lifetime. Never unlink
    // it: another process could otherwise lock a different inode at this path.
    _writer_lock: File,
}

#[derive(Serialize, Deserialize)]
struct Snapshot {
    format_version: u32,
    schema_version: i64,
    sequence: i64,
    request_id: String,
    recorded_at: String,
    materials: Vec<Material>,
}

impl Store {
    pub fn open(root: &Path) -> anyhow::Result<Self> {
        fs::create_dir_all(root).context("데이터 폴더를 만들 수 없습니다")?;
        let writer_lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root.join(".writer.lock"))
            .context("데이터 폴더의 잠금 파일을 열 수 없습니다")?;
        writer_lock.try_lock_exclusive().context(
            "이 데이터 폴더를 사용하는 앱이 이미 실행 중이거나 잠금을 사용할 수 없습니다",
        )?;

        let database = root.join("app.sqlite3");
        // Inspect existing databases read-only before any writable connection,
        // journal-mode change, migration, or recovery can modify their contents.
        if database.exists() {
            check_existing_version(&database)?;
        }
        let mut connection =
            Connection::open(&database).context("SQLite 데이터 파일을 열 수 없습니다")?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "foreign_keys", true)?;
        let version = check_version(&connection)?;
        check_integrity(&connection)?;
        if version < SCHEMA_VERSION {
            if version > 0 {
                backup_before_migration(&connection, root, version)?;
            } else {
                let tables: i64 = connection.query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE name NOT LIKE 'sqlite_%'",
                    [],
                    |row| row.get(0),
                )?;
                if tables != 0 {
                    bail!("버전 정보가 없는 기존 데이터베이스는 변경하지 않습니다");
                }
            }
            migrate(&mut connection, version)?;
        }
        validate_schema(&connection)?;
        // No WAL sidecar is required when a normally closed database is moved.
        connection.pragma_update(None, "journal_mode", "DELETE")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        fs::create_dir_all(root.join("snapshots")).context("JSON 기록 폴더를 만들 수 없습니다")?;
        let mut store = Self {
            connection,
            root: root.to_owned(),
            _writer_lock: writer_lock,
        };
        store
            .recover()
            .context("JSON 변경 기록을 복구할 수 없습니다")?;
        Ok(store)
    }

    pub fn list(&self) -> Result<Vec<Material>, StoreError> {
        read_materials(&self.connection).map_err(unavailable)
    }

    pub fn get(&self, id: &str) -> Result<Option<Material>, StoreError> {
        self.connection
            .query_row(
                "SELECT id, name, unit, created_at FROM materials WHERE id = ?1",
                [id],
                material_from_row,
            )
            .optional()
            .map_err(unavailable)
    }

    pub fn create(&mut self, input: CreateMaterial) -> Result<Material, StoreError> {
        let input = validate_input(input)?;
        let previous: Option<(String, String, String)> = self
            .connection
            .query_row(
                "SELECT name, unit, material_id FROM creation_requests WHERE request_id = ?1",
                [&input.request_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(unavailable)?;
        if let Some((name, unit, id)) = previous {
            if name != input.name || unit != input.unit {
                return Err(StoreError::Conflict(
                    "같은 요청 ID에 다른 입력을 사용할 수 없습니다".into(),
                ));
            }
            self.recover()?;
            return self.get(&id)?.ok_or_else(|| {
                StoreError::Unavailable("등록 요청에 연결된 재료를 찾을 수 없습니다".into())
            });
        }

        // A failed publication must be resolved before any later mutation.
        self.recover()?;
        let material = Material {
            id: Uuid::new_v4().to_string(),
            name: input.name,
            unit: input.unit,
            created_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        };
        let transaction = self.connection.transaction().map_err(unavailable)?;
        transaction
            .execute(
                "INSERT INTO materials (id, name, unit, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![
                    material.id,
                    material.name,
                    material.unit,
                    material.created_at
                ],
            )
            .map_err(unavailable)?;
        transaction.execute(
            "INSERT INTO creation_requests (request_id, name, unit, material_id) VALUES (?1, ?2, ?3, ?4)",
            params![input.request_id, material.name, material.unit, material.id],
        ).map_err(unavailable)?;
        transaction
            .execute(
                "INSERT INTO snapshot_outbox (payload, published) VALUES ('', 0)",
                [],
            )
            .map_err(unavailable)?;
        let sequence = transaction.last_insert_rowid();
        let snapshot = Snapshot {
            format_version: 1,
            schema_version: SCHEMA_VERSION,
            sequence,
            request_id: input.request_id,
            recorded_at: material.created_at.clone(),
            materials: read_materials(&transaction).map_err(unavailable)?,
        };
        let payload = serde_json::to_string_pretty(&snapshot).map_err(unavailable)? + "\n";
        transaction
            .execute(
                "UPDATE snapshot_outbox SET payload = ?1 WHERE sequence = ?2",
                params![payload, sequence],
            )
            .map_err(unavailable)?;
        transaction.commit().map_err(unavailable)?;

        self.recover().map_err(|error| StoreError::Unavailable(format!(
            "재료는 DB에 저장되었지만 JSON 기록을 완료하지 못했습니다. 같은 요청으로 다시 시도하세요: {error}"
        )))?;
        Ok(material)
    }

    /// Verify retained immutable files and regenerate missing ones, including
    /// published records whose files were lost during a database-only move.
    pub fn recover(&mut self) -> Result<(), StoreError> {
        // Hold only record keys in memory; retained full-state payloads may be
        // much larger than the current material list.
        let entries: Vec<(i64, bool)> = self
            .connection
            .prepare("SELECT sequence, published FROM snapshot_outbox ORDER BY sequence")
            .map_err(unavailable)?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(unavailable)?
            .collect::<rusqlite::Result<_>>()
            .map_err(unavailable)?;
        for (sequence, published) in entries {
            let payload: String = self
                .connection
                .query_row(
                    "SELECT payload FROM snapshot_outbox WHERE sequence = ?1",
                    [sequence],
                    |row| row.get(0),
                )
                .map_err(unavailable)?;
            let snapshot: Snapshot = serde_json::from_str(&payload).map_err(unavailable)?;
            if snapshot.format_version != 1
                || !(1..=SCHEMA_VERSION).contains(&snapshot.schema_version)
                || snapshot.sequence != sequence
            {
                return Err(StoreError::Unavailable(
                    "지원하지 않거나 손상된 JSON 변경 기록입니다".into(),
                ));
            }
            publish_snapshot(&self.root.join("snapshots"), sequence, payload.as_bytes())
                .map_err(unavailable)?;
            if !published {
                self.connection
                    .execute(
                        "UPDATE snapshot_outbox SET published = 1 WHERE sequence = ?1",
                        [sequence],
                    )
                    .map_err(unavailable)?;
            }
        }
        Ok(())
    }
}

fn unavailable(error: impl std::fmt::Display) -> StoreError {
    StoreError::Unavailable(error.to_string())
}

fn validate_input(mut input: CreateMaterial) -> Result<CreateMaterial, StoreError> {
    input.request_id = Uuid::parse_str(&input.request_id)
        .map_err(|_| StoreError::Validation("요청 ID는 올바른 UUID여야 합니다".into()))?
        .to_string();
    input.name = input.name.trim().to_owned();
    input.unit = input.unit.trim().to_owned();
    for (label, value, max) in [
        ("재료 이름", &input.name, 100),
        ("관리 단위", &input.unit, 20),
    ] {
        if value.is_empty() || value.chars().count() > max || value.chars().any(char::is_control) {
            return Err(StoreError::Validation(format!(
                "{label}은 제어문자 없이 1~{max}자로 입력하세요"
            )));
        }
    }
    Ok(input)
}

fn material_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Material> {
    Ok(Material {
        id: row.get(0)?,
        name: row.get(1)?,
        unit: row.get(2)?,
        created_at: row.get(3)?,
    })
}

fn read_materials(connection: &Connection) -> rusqlite::Result<Vec<Material>> {
    connection
        .prepare("SELECT id, name, unit, created_at FROM materials ORDER BY created_at, id")?
        .query_map([], material_from_row)?
        .collect()
}

fn check_existing_version(database: &Path) -> anyhow::Result<()> {
    let existing = Connection::open_with_flags(database, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let result = check_version(&existing);
    drop(existing);
    match result {
        Ok(_) => Ok(()),
        Err(error)
            if matches!(error.downcast_ref::<rusqlite::Error>(),
                Some(rusqlite::Error::SqliteFailure(code, _))
                    if code.extended_code == rusqlite::ffi::SQLITE_READONLY_ROLLBACK) =>
        {
            // A crashed rollback-mode writer can require recovery even for a
            // version query. Recover a private copy first: the on-disk header
            // may contain an uncommitted version, and opening the original for
            // writes would alter an unsupported future database before refusal.
            let probe =
                tempfile::tempdir().context("DB 복구 검증용 임시 폴더를 만들 수 없습니다")?;
            let copy = probe.path().join("app.sqlite3");
            fs::copy(database, &copy).context("DB 복구 검증용 복사본을 만들 수 없습니다")?;
            let mut journal = database.as_os_str().to_os_string();
            journal.push("-journal");
            fs::copy(
                Path::new(&journal),
                probe.path().join("app.sqlite3-journal"),
            )
            .context("DB 복구 검증용 롤백 기록을 복사할 수 없습니다")?;
            let recovered = Connection::open_with_flags(&copy, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
            check_version(&recovered)?;
            check_integrity(&recovered)?;
            Ok(())
        }
        Err(error) => Err(error),
    }
}

fn check_version(connection: &Connection) -> anyhow::Result<i64> {
    let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if !(0..=SCHEMA_VERSION).contains(&version) {
        bail!("지원하지 않는 DB 스키마 버전 {version}입니다. 더 새로운 앱으로 열어주세요");
    }
    Ok(version)
}

fn check_integrity(connection: &Connection) -> anyhow::Result<()> {
    let result: String = connection.pragma_query_value(None, "quick_check", |row| row.get(0))?;
    if result != "ok" {
        bail!("SQLite 무결성 검사 실패: {result}");
    }
    Ok(())
}

fn validate_schema(connection: &Connection) -> anyhow::Result<()> {
    connection.prepare("SELECT id, name, unit, created_at FROM materials LIMIT 0")?;
    connection.prepare("SELECT sequence, payload, published FROM snapshot_outbox LIMIT 0")?;
    connection
        .prepare("SELECT request_id, name, unit, material_id FROM creation_requests LIMIT 0")?;
    let violation: Option<String> = connection
        .query_row("PRAGMA foreign_key_check", [], |row| row.get(0))
        .optional()?;
    if violation.is_some() {
        bail!("DB 참조 무결성 검사에 실패했습니다");
    }
    check_integrity(connection)
}

fn backup_before_migration(
    connection: &Connection,
    root: &Path,
    version: i64,
) -> anyhow::Result<()> {
    let directory = root.join("backups");
    fs::create_dir_all(&directory)?;
    let temporary = NamedTempFile::new_in(&directory)?;
    connection.backup(rusqlite::MAIN_DB, temporary.path(), None)?;
    temporary.as_file().sync_all()?;
    temporary
        .persist_noclobber(directory.join(format!("pre-v{version}-{}.sqlite3", Uuid::new_v4())))?;
    sync_directory(&directory)?;
    Ok(())
}

fn migrate(connection: &mut Connection, starting_version: i64) -> anyhow::Result<()> {
    // All sequential steps and validation share a transaction. An error leaves
    // the original schema/data intact; the consistent backup is also retained.
    let transaction = connection.transaction()?;
    let mut version = starting_version;
    if version == 0 {
        transaction.execute_batch(
            "CREATE TABLE materials (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL CHECK(length(name) BETWEEN 1 AND 100),
                unit TEXT NOT NULL CHECK(length(unit) BETWEEN 1 AND 20),
                created_at TEXT NOT NULL
            );
            CREATE TABLE snapshot_outbox (
                sequence INTEGER PRIMARY KEY AUTOINCREMENT,
                payload TEXT NOT NULL,
                published INTEGER NOT NULL DEFAULT 0 CHECK(published IN (0, 1))
            );
            PRAGMA user_version = 1;",
        )?;
        version = 1;
    }
    if version == 1 {
        transaction.execute_batch(
            "CREATE TABLE creation_requests (
                request_id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                unit TEXT NOT NULL,
                material_id TEXT NOT NULL UNIQUE REFERENCES materials(id)
            );
            PRAGMA user_version = 2;",
        )?;
    }
    validate_schema(&transaction)?;
    transaction.commit()?;
    Ok(())
}

fn publish_snapshot(directory: &Path, sequence: i64, payload: &[u8]) -> anyhow::Result<()> {
    fs::create_dir_all(directory)?;
    let destination = directory.join(format!("{sequence:020}.json"));
    match fs::read(&destination) {
        Ok(existing) => {
            if existing != payload {
                bail!("기존 JSON 기록이 DB와 다릅니다: {}", destination.display());
            }
            // A preceding attempt may have published the file but failed while
            // syncing the directory. A retry must finish that durability step.
            sync_directory(directory)?;
            return Ok(());
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let mut temporary = NamedTempFile::new_in(directory)?;
    temporary.write_all(payload)?;
    temporary.as_file().sync_all()?;
    // No-clobber publication also protects a destination created between the
    // read and publication. Partial JSON is never exposed at its final name.
    match temporary.persist_noclobber(&destination) {
        Ok(_) => {}
        Err(error) if error.error.kind() == ErrorKind::AlreadyExists => {
            if fs::read(&destination)? != payload {
                bail!("동시에 생성된 JSON 기록이 DB와 다릅니다");
            }
        }
        Err(error) => return Err(error.into()),
    }
    sync_directory(directory)?;
    Ok(())
}

fn sync_directory(directory: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    File::open(directory)?.sync_all()?;
    // Windows does not support opening directories through std::fs::File.
    // The snapshot itself is flushed before publication on every platform.
    #[cfg(not(unix))]
    let _ = directory;
    Ok(())
}
