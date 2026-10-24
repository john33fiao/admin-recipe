use super::{
    CreateMaterial, Material, Store, StoreError, read_materials, unavailable, validate_input,
};
use crate::{
    catalog::*,
    domain::{Money, Quantity, Unit, UnitPrice},
};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Serialize, de::DeserializeOwned};
use uuid::Uuid;

fn validation(error: impl std::fmt::Display) -> StoreError {
    StoreError::Validation(error.to_string())
}

pub(super) fn uuid(value: &str, label: &str) -> Result<String, StoreError> {
    Uuid::parse_str(value)
        .map(|id| id.to_string())
        .map_err(|_| validation(format!("{label}가 올바른 UUID가 아닙니다")))
}

pub(super) fn normalize_meta(meta: &mut MutationMeta) -> Result<(), StoreError> {
    meta.request_id = uuid(&meta.request_id, "요청 ID")?;
    meta.epoch = uuid(&meta.epoch, "상태 세대")?;
    let revision = meta
        .expected_revision
        .parse::<i64>()
        .map_err(|_| validation("개정 번호가 올바르지 않습니다"))?;
    if revision < 0 || !meta.expected_revision.chars().all(|c| c.is_ascii_digit()) {
        return Err(validation("개정 번호가 올바르지 않습니다"));
    }
    meta.expected_revision = revision.to_string();
    Ok(())
}

pub(super) fn normalize_fields(
    name: &mut String,
    unit: &mut String,
    kind: &str,
    description: &mut String,
    allow_unknown: bool,
) -> Result<(), StoreError> {
    *name = name.trim().to_owned();
    *unit = unit.trim().to_owned();
    for (label, value, max) in [
        ("재료 이름", name.as_str(), 100),
        ("관리 단위", unit.as_str(), 20),
    ] {
        if value.is_empty() || value.chars().count() > max || value.chars().any(char::is_control) {
            return Err(validation(format!(
                "{label}은 제어문자 없이 1~{max}자로 입력하세요"
            )));
        }
    }
    Unit::parse(unit).map_err(validation)?;
    if !matches!(kind, "raw" | "prepared" | "stocked" | "menu")
        && !(allow_unknown && kind == "unclassified")
    {
        return Err(validation("재료 종류를 선택하세요"));
    }
    *description = description
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .trim()
        .to_owned();
    if description.chars().count() > 2000
        || description.chars().any(|c| c.is_control() && c != '\n')
    {
        return Err(validation("설명은 2000자 이내의 본문으로 입력하세요"));
    }
    Ok(())
}

pub(super) fn state(connection: &Connection) -> Result<CatalogState, StoreError> {
    connection
        .query_row(
            "SELECT epoch, revision FROM catalog_state WHERE singleton=1",
            [],
            |r| {
                Ok(CatalogState {
                    epoch: r.get(0)?,
                    revision: r.get::<_, i64>(1)?.to_string(),
                })
            },
        )
        .map_err(unavailable)
}

fn version_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<MaterialVersion> {
    Ok(MaterialVersion {
        id: row.get(0)?,
        material_id: row.get(1)?,
        name: row.get(2)?,
        unit: row.get(3)?,
        canonical_unit: row.get(4)?,
        dimension: row.get(5)?,
        kind: row.get(6)?,
        description: row.get(7)?,
        archived: row.get(8)?,
        recorded_at: row.get(9)?,
        seq: row.get::<_, Option<i64>>(10)?.map(|n| n.to_string()),
        source: row.get(11)?,
    })
}

const VERSION_COLUMNS: &str = "v.id,v.material_id,v.name,v.unit,v.canonical_unit,v.dimension,v.kind,v.description,v.archived,v.recorded_at,v.sequence,v.source";

pub(super) fn versions(
    connection: &Connection,
    id: Option<&str>,
) -> Result<Vec<MaterialVersion>, StoreError> {
    let sql = format!(
        "SELECT {VERSION_COLUMNS} FROM material_versions v WHERE (?1 IS NULL OR v.material_id=?1) ORDER BY v.sequence IS NOT NULL,v.sequence,v.recorded_at,v.id"
    );
    connection
        .prepare(&sql)
        .map_err(unavailable)?
        .query_map([id], version_row)
        .map_err(unavailable)?
        .collect::<rusqlite::Result<_>>()
        .map_err(unavailable)
}

pub(super) fn current_version(
    connection: &Connection,
    id: &str,
) -> Result<Option<MaterialVersion>, StoreError> {
    connection.query_row(&format!("SELECT {VERSION_COLUMNS} FROM material_versions v JOIN material_heads h ON h.current_version_id=v.id AND h.material_id=v.material_id WHERE h.material_id=?1"), [id], version_row).optional().map_err(unavailable)
}

pub(super) fn prices(
    connection: &Connection,
    id: Option<&str>,
) -> Result<Vec<MaterialPriceVersion>, StoreError> {
    let mut statement=connection.prepare("SELECT id,material_id,material_version_id,quantity_micros,amount_micros,unit,input_quantity,input_unit,recorded_at,sequence FROM material_prices WHERE (?1 IS NULL OR material_id=?1) ORDER BY sequence,id").map_err(unavailable)?;
    let rows = statement
        .query_map([id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, String>(6)?,
                r.get::<_, String>(7)?,
                r.get::<_, String>(8)?,
                r.get::<_, i64>(9)?,
            ))
        })
        .map_err(unavailable)?;
    rows.map(|row| {
        let (
            id,
            material_id,
            material_version_id,
            q,
            a,
            unit,
            input_quantity,
            input_unit,
            recorded_at,
            seq,
        ) = row.map_err(unavailable)?;
        Ok(MaterialPriceVersion {
            id,
            material_id,
            material_version_id,
            quantity: Quantity::from_micros(q).map_err(unavailable)?.decimal(),
            amount: Money::from_micros(a).map_err(unavailable)?.decimal(),
            unit,
            input_quantity,
            input_unit,
            recorded_at,
            seq: seq.to_string(),
        })
    })
    .collect()
}

pub(super) fn summary(
    connection: &Connection,
    version: MaterialVersion,
) -> Result<MaterialSummary, StoreError> {
    let created_at = connection
        .query_row(
            "SELECT created_at FROM materials WHERE id=?1",
            [&version.material_id],
            |r| r.get(0),
        )
        .map_err(unavailable)?;
    let price = super::holding::price(
        connection,
        &version.material_id,
        registered_price(connection, &version.material_id)?,
    )?;
    Ok(MaterialSummary {
        id: version.material_id,
        version_id: version.id,
        name: version.name,
        unit: version.unit,
        canonical_unit: version.canonical_unit,
        dimension: version.dimension,
        kind: version.kind,
        description: version.description,
        archived: version.archived,
        created_at,
        updated_at: version.recorded_at,
        price,
    })
}

pub(super) fn registered_price(
    connection: &Connection,
    material_id: &str,
) -> Result<Option<UnitPrice>, StoreError> {
    prices(connection, Some(material_id))?
        .pop()
        .map(|p| {
            UnitPrice::new(
                Money::parse(&p.amount).map_err(unavailable)?,
                Quantity::parse(&p.quantity).map_err(unavailable)?,
                p.id,
            )
            .map_err(unavailable)
        })
        .transpose()
}

fn required_version(
    connection: &Connection,
    id: &str,
    expected: &str,
) -> Result<MaterialVersion, StoreError> {
    let version =
        current_version(connection, id)?.ok_or_else(|| validation("재료를 찾을 수 없습니다"))?;
    if version.id != expected {
        return Err(StoreError::StaleState(
            "재료가 변경되었습니다. 최신 내용을 확인해 주세요".into(),
        ));
    }
    Ok(version)
}

fn insert_version(connection: &Connection, version: &MaterialVersion) -> Result<(), StoreError> {
    let seq = version
        .seq
        .as_ref()
        .map(|s| s.parse::<i64>())
        .transpose()
        .map_err(unavailable)?;
    connection.execute("INSERT INTO material_versions(id,material_id,name,unit,canonical_unit,dimension,kind,description,archived,recorded_at,sequence,source) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",params![version.id,version.material_id,version.name,version.unit,version.canonical_unit,version.dimension,version.kind,version.description,version.archived,version.recorded_at,seq,version.source]).map_err(unavailable)?;
    connection.execute("INSERT INTO material_heads(material_id,current_version_id) VALUES(?1,?2) ON CONFLICT(material_id) DO UPDATE SET current_version_id=excluded.current_version_id",params![version.material_id,version.id]).map_err(unavailable)?;
    Ok(())
}

pub(super) fn insert_material(
    connection: &Connection,
    name: String,
    unit: String,
    kind: String,
    description: String,
    seq: i64,
    time: &str,
) -> Result<MaterialSummary, StoreError> {
    let id = Uuid::new_v4().to_string();
    connection
        .execute(
            "INSERT INTO materials(id,name,unit,created_at) VALUES(?1,?2,?3,?4)",
            params![id, name, unit, time],
        )
        .map_err(unavailable)?;
    let parsed = Unit::parse(&unit).map_err(validation)?;
    let version = MaterialVersion {
        id: Uuid::new_v4().to_string(),
        material_id: id,
        name,
        unit,
        canonical_unit: parsed.canonical().into(),
        dimension: parsed.dimension().into(),
        kind,
        description,
        archived: false,
        recorded_at: time.into(),
        seq: Some(seq.to_string()),
        source: "catalog".into(),
    };
    insert_version(connection, &version)?;
    summary(connection, version)
}

pub(super) struct PriceInput<'a> {
    pub material_id: &'a str,
    pub expected_version_id: &'a str,
    pub quantity: &'a str,
    pub unit: &'a str,
    pub amount: &'a str,
}

pub(super) fn apply_catalog_price(
    connection: &Connection,
    input: PriceInput<'_>,
    seq: i64,
    time: &str,
) -> Result<MaterialPriceVersion, StoreError> {
    let version = required_version(connection, input.material_id, input.expected_version_id)?;
    let unit = Unit::parse(input.unit).map_err(validation)?;
    let quantity = Quantity::parse(input.quantity).map_err(validation)?;
    let amount = Money::parse(input.amount).map_err(validation)?;
    if quantity.micros() == 0 {
        return Err(validation("가격 기준 수량은 0보다 커야 합니다"));
    }
    if !unit.same_dimension(&Unit::parse(&version.unit).map_err(unavailable)?) {
        return Err(validation("가격 수량은 재료와 같은 차원 단위를 사용하세요"));
    }
    let canonical_quantity = unit.to_canonical(quantity).map_err(validation)?;
    let price = MaterialPriceVersion {
        id: Uuid::new_v4().to_string(),
        material_id: input.material_id.into(),
        material_version_id: version.id,
        quantity: canonical_quantity.decimal(),
        amount: amount.decimal(),
        unit: unit.canonical().into(),
        input_quantity: quantity.decimal(),
        input_unit: input.unit.into(),
        recorded_at: time.into(),
        seq: seq.to_string(),
    };
    connection
        .execute(
            "INSERT INTO material_prices(id,material_id,material_version_id,quantity_micros,
         amount_micros,unit,input_quantity,input_unit,recorded_at,sequence)
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                price.id,
                price.material_id,
                price.material_version_id,
                canonical_quantity.micros(),
                amount.micros(),
                price.unit,
                price.input_quantity,
                price.input_unit,
                price.recorded_at,
                seq
            ],
        )
        .map_err(unavailable)?;
    Ok(price)
}

impl Store {
    pub fn catalog_state(&self) -> Result<CatalogState, StoreError> {
        state(&self.connection)
    }

    pub fn catalog_list(&self, filter: CatalogFilter) -> Result<Vec<MaterialSummary>, StoreError> {
        let query = filter.query.trim();
        if query.chars().count() > 100 || query.chars().any(char::is_control) {
            return Err(validation("검색어는 제어문자 없이 100자 이내로 입력하세요"));
        }
        let archive = match filter.archive {
            ArchiveFilter::Active => Some(false),
            ArchiveFilter::Archived => Some(true),
            ArchiveFilter::All => None,
        };
        let sql = format!(
            "SELECT {VERSION_COLUMNS} FROM material_heads h JOIN material_versions v ON v.id=h.current_version_id AND v.material_id=h.material_id JOIN materials m ON m.id=h.material_id WHERE instr(v.name,?1)>0 AND (?2 IS NULL OR v.archived=?2) ORDER BY m.created_at,m.id"
        );
        let mut statement = self.connection.prepare(&sql).map_err(unavailable)?;
        let list = statement
            .query_map(params![query, archive], version_row)
            .map_err(unavailable)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(unavailable)?;
        list.into_iter()
            .map(|v| summary(&self.connection, v))
            .collect()
    }

    pub fn catalog_detail(&self, id: &str) -> Result<Option<MaterialDetail>, StoreError> {
        let id = uuid(id, "재료 ID")?;
        let Some(version) = current_version(&self.connection, &id)? else {
            return Ok(None);
        };
        Ok(Some(MaterialDetail {
            item: summary(&self.connection, version)?,
            versions: versions(&self.connection, Some(&id))?,
            prices: prices(&self.connection, Some(&id))?,
            state: state(&self.connection)?,
        }))
    }

    pub fn catalog_create(
        &mut self,
        mut input: CreateCatalogMaterial,
    ) -> Result<CatalogMutation<MaterialSummary>, StoreError> {
        normalize_meta(&mut input.meta)?;
        normalize_fields(
            &mut input.name,
            &mut input.unit,
            &input.kind,
            &mut input.description,
            false,
        )?;
        let meta = input.meta.clone();
        let payload = serde_json::to_string(&input).map_err(unavailable)?;
        self.catalog_mutate(
            Some(&meta),
            &meta.request_id,
            "material.create",
            &payload,
            move |tx, seq, time| {
                insert_material(
                    tx,
                    input.name,
                    input.unit,
                    input.kind,
                    input.description,
                    seq,
                    time,
                )
            },
        )
    }

    pub fn catalog_revise(
        &mut self,
        id: &str,
        mut input: ReviseCatalogMaterial,
    ) -> Result<CatalogMutation<MaterialSummary>, StoreError> {
        let id = uuid(id, "재료 ID")?;
        normalize_meta(&mut input.meta)?;
        input.expected_version_id = uuid(&input.expected_version_id, "버전 ID")?;
        normalize_fields(
            &mut input.name,
            &mut input.unit,
            &input.kind,
            &mut input.description,
            true,
        )?;
        let payload = serde_json::to_string(&(id.clone(), &input)).map_err(unavailable)?;
        let meta = input.meta.clone();
        self.catalog_mutate(
            Some(&meta),
            &meta.request_id,
            "material.revise",
            &payload,
            move |tx, seq, time| {
                let old = required_version(tx, &id, &input.expected_version_id)?;
                let unit = Unit::parse(&input.unit).map_err(validation)?;
                if !unit.same_dimension(&Unit::parse(&old.unit).map_err(unavailable)?) {
                    return Err(validation("다른 차원의 관리 단위는 새 품목으로 등록하세요"));
                }
                if input.kind == "unclassified" && old.kind != "unclassified" {
                    return Err(validation("재료 종류를 선택하세요"));
                }
                let version = MaterialVersion {
                    id: Uuid::new_v4().to_string(),
                    material_id: id,
                    name: input.name,
                    unit: input.unit,
                    canonical_unit: unit.canonical().into(),
                    dimension: unit.dimension().into(),
                    kind: input.kind,
                    description: input.description,
                    archived: old.archived,
                    recorded_at: time.into(),
                    seq: Some(seq.to_string()),
                    source: "catalog".into(),
                };
                insert_version(tx, &version)?;
                summary(tx, version)
            },
        )
    }

    pub fn catalog_archive(
        &mut self,
        id: &str,
        mut input: ArchiveCatalogMaterial,
    ) -> Result<CatalogMutation<MaterialSummary>, StoreError> {
        let id = uuid(id, "재료 ID")?;
        normalize_meta(&mut input.meta)?;
        input.expected_version_id = uuid(&input.expected_version_id, "버전 ID")?;
        let payload = serde_json::to_string(&(id.clone(), &input)).map_err(unavailable)?;
        let meta = input.meta.clone();
        self.catalog_mutate(
            Some(&meta),
            &meta.request_id,
            "material.archive",
            &payload,
            move |tx, seq, time| {
                let mut version = required_version(tx, &id, &input.expected_version_id)?;
                version.id = Uuid::new_v4().to_string();
                version.archived = true;
                version.recorded_at = time.into();
                version.seq = Some(seq.to_string());
                version.source = "catalog".into();
                insert_version(tx, &version)?;
                summary(tx, version)
            },
        )
    }

    pub fn catalog_add_price(
        &mut self,
        id: &str,
        mut input: CreateCatalogPrice,
    ) -> Result<CatalogMutation<MaterialPriceVersion>, StoreError> {
        let id = uuid(id, "재료 ID")?;
        normalize_meta(&mut input.meta)?;
        input.expected_version_id = uuid(&input.expected_version_id, "버전 ID")?;
        let quantity = Quantity::parse(&input.quantity).map_err(validation)?;
        let amount = Money::parse(&input.amount).map_err(validation)?;
        if quantity.micros() == 0 {
            return Err(validation("가격 기준 수량은 0보다 커야 합니다"));
        }
        input.quantity = quantity.decimal();
        input.amount = amount.decimal();
        input.unit = input.unit.trim().to_owned();
        let unit = Unit::parse(&input.unit).map_err(validation)?;
        unit.to_canonical(quantity).map_err(validation)?;
        let payload = serde_json::to_string(&(id.clone(), &input)).map_err(unavailable)?;
        let meta = input.meta.clone();
        self.catalog_mutate(
            Some(&meta),
            &meta.request_id,
            "material.price",
            &payload,
            move |tx, seq, time| {
                apply_catalog_price(
                    tx,
                    PriceInput {
                        material_id: &id,
                        expected_version_id: &input.expected_version_id,
                        quantity: &input.quantity,
                        unit: &input.unit,
                        amount: &input.amount,
                    },
                    seq,
                    time,
                )
            },
        )
    }

    pub(super) fn legacy_create(&mut self, input: CreateMaterial) -> Result<Material, StoreError> {
        let input = validate_input(input)?;
        let previous: Option<(String, String, String)> = self
            .connection
            .query_row(
                "SELECT name,unit,material_id FROM creation_requests WHERE request_id=?1",
                [&input.request_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
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
            return self
                .connection
                .query_row(
                    "SELECT id,name,unit,created_at FROM materials WHERE id=?1",
                    [id],
                    super::material_from_row,
                )
                .map_err(unavailable);
        }
        let payload = serde_json::to_string(&input).map_err(unavailable)?;
        let request_id = input.request_id.clone();
        let result = self.catalog_mutate(
            None,
            &request_id,
            "legacy.material.create",
            &payload,
            move |tx, seq, time| {
                let item = insert_material(
                    tx,
                    input.name,
                    input.unit,
                    "unclassified".into(),
                    String::new(),
                    seq,
                    time,
                )?;
                tx.execute(
                    "INSERT INTO creation_requests(request_id,name,unit,material_id)
                     VALUES(?1,?2,?3,?4)",
                    params![input.request_id, item.name, item.unit, item.id],
                )
                .map_err(unavailable)?;
                Ok(Material {
                    id: item.id,
                    name: item.name,
                    unit: item.unit,
                    created_at: item.created_at,
                })
            },
        )?;
        Ok(result.value)
    }

    fn catalog_mutate<T: Serialize + DeserializeOwned>(
        &mut self,
        meta: Option<&MutationMeta>,
        request_id: &str,
        operation: &str,
        payload: &str,
        apply: impl FnOnce(&Transaction<'_>, i64, &str) -> Result<T, StoreError>,
    ) -> Result<CatalogMutation<T>, StoreError> {
        self.mutate(
            meta,
            request_id,
            operation,
            payload,
            |tx, seq, time, state| {
                Ok(CatalogMutation {
                    value: apply(tx, seq, time)?,
                    state: state.clone(),
                })
            },
        )
    }
}
pub(super) fn migrate_catalog(connection: &Connection) -> anyhow::Result<()> {
    connection.execute_batch("CREATE TABLE material_versions(
        id TEXT PRIMARY KEY NOT NULL,material_id TEXT NOT NULL REFERENCES materials(id),name TEXT NOT NULL CHECK(length(name) BETWEEN 1 AND 100),unit TEXT NOT NULL CHECK(length(unit) BETWEEN 1 AND 20),canonical_unit TEXT NOT NULL,dimension TEXT NOT NULL CHECK(dimension IN('mass','volume','custom')),kind TEXT NOT NULL CHECK(kind IN('raw','prepared','stocked','menu','unclassified')),description TEXT NOT NULL,archived INTEGER NOT NULL CHECK(archived IN(0,1)),recorded_at TEXT NOT NULL,sequence INTEGER REFERENCES snapshot_outbox(sequence),source TEXT NOT NULL,UNIQUE(material_id,id));
        CREATE TABLE material_heads(material_id TEXT PRIMARY KEY NOT NULL REFERENCES materials(id),current_version_id TEXT NOT NULL,FOREIGN KEY(material_id,current_version_id) REFERENCES material_versions(material_id,id));
        CREATE TABLE material_prices(id TEXT PRIMARY KEY NOT NULL,material_id TEXT NOT NULL,material_version_id TEXT NOT NULL,quantity_micros INTEGER NOT NULL CHECK(quantity_micros>0 AND quantity_micros<=9000000000000000),amount_micros INTEGER NOT NULL CHECK(amount_micros>=0 AND amount_micros<=9000000000000000000),unit TEXT NOT NULL,input_quantity TEXT NOT NULL,input_unit TEXT NOT NULL,recorded_at TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),FOREIGN KEY(material_id,material_version_id) REFERENCES material_versions(material_id,id));
        CREATE TABLE mutation_requests(request_id TEXT PRIMARY KEY NOT NULL,operation TEXT NOT NULL,payload TEXT NOT NULL,response TEXT NOT NULL,epoch TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence));
        CREATE TABLE catalog_state(singleton INTEGER PRIMARY KEY CHECK(singleton=1),epoch TEXT NOT NULL,revision INTEGER NOT NULL CHECK(revision>=0));
        CREATE INDEX material_versions_material ON material_versions(material_id,sequence);
        CREATE INDEX material_prices_material ON material_prices(material_id,sequence);")?;
    let revision: i64 = connection.query_row(
        "SELECT COALESCE(MAX(sequence),0) FROM snapshot_outbox",
        [],
        |r| r.get(0),
    )?;
    connection.execute(
        "INSERT INTO catalog_state VALUES(1,?1,?2)",
        params![Uuid::new_v4().to_string(), revision],
    )?;
    let snapshots = connection
        .prepare("SELECT sequence,payload FROM snapshot_outbox ORDER BY sequence")?
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    // A future/invalid journal must reject the entire migration, including
    // databases whose legacy materials have no creation_requests rows.
    for (seq, payload) in &snapshots {
        let snapshot: super::Snapshot = serde_json::from_str(payload)?;
        if snapshot.format_version != 1
            || !(1..=2).contains(&snapshot.schema_version)
            || snapshot.sequence != *seq
        {
            anyhow::bail!("지원하지 않거나 손상된 이전 JSON 변경 기록입니다");
        }
    }
    for material in read_materials(connection)? {
        let request: Option<String> = connection
            .query_row(
                "SELECT request_id FROM creation_requests WHERE material_id=?1",
                [&material.id],
                |r| r.get(0),
            )
            .optional()?;
        let mut sequence = None;
        if let Some(request) = request {
            for (seq, payload) in &snapshots {
                let snapshot: super::Snapshot = serde_json::from_str(payload)?;
                if snapshot.request_id == request
                    && snapshot.sequence == *seq
                    && snapshot.materials.iter().any(|m| {
                        m.id == material.id && m.name == material.name && m.unit == material.unit
                    })
                {
                    sequence = Some(seq.to_string());
                    break;
                }
            }
        }
        let unit = Unit::parse(&material.unit)?;
        let source = if sequence.is_some() {
            "legacy_snapshot"
        } else {
            "legacy_migration"
        };
        insert_version(
            connection,
            &MaterialVersion {
                id: Uuid::new_v4().to_string(),
                material_id: material.id,
                name: material.name,
                unit: material.unit,
                canonical_unit: unit.canonical().into(),
                dimension: unit.dimension().into(),
                kind: "unclassified".into(),
                description: String::new(),
                archived: false,
                recorded_at: material.created_at,
                seq: sequence,
                source: source.into(),
            },
        )?;
    }
    connection.pragma_update(None, "user_version", 3)?;
    Ok(())
}

pub(super) fn validate_catalog_schema(connection: &Connection) -> anyhow::Result<()> {
    connection.prepare("SELECT id,material_id,name,unit,canonical_unit,dimension,kind,description,archived,recorded_at,sequence,source FROM material_versions LIMIT 0")?;
    connection.prepare("SELECT material_id,current_version_id FROM material_heads LIMIT 0")?;
    connection.prepare("SELECT id,material_id,material_version_id,quantity_micros,amount_micros,unit,input_quantity,input_unit,recorded_at,sequence FROM material_prices LIMIT 0")?;
    connection.prepare("SELECT request_id,operation,payload,response,epoch,sequence FROM mutation_requests LIMIT 0")?;
    let metadata = state(connection)?;
    Uuid::parse_str(&metadata.epoch)?;
    let missing:i64=connection.query_row("SELECT COUNT(*) FROM materials m LEFT JOIN material_heads h ON h.material_id=m.id WHERE h.material_id IS NULL",[],|r|r.get(0))?;
    if missing != 0 {
        anyhow::bail!("품목의 현재 버전 연결이 없습니다");
    }
    Ok(())
}
