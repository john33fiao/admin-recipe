use super::{
    Store, StoreError, catalog, correction, production, purchase_order, receipt, sale, snapshot,
    stocktake, unavailable, waste,
};
use crate::{
    business::*,
    catalog::MutationMeta,
    domain::{Money, Quantity, Unit},
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

fn validation(message: impl std::fmt::Display) -> StoreError {
    StoreError::Validation(message.to_string())
}

fn parse_kind(kind: &str) -> Result<BusinessKind, StoreError> {
    match kind {
        "catalog_price" => Ok(BusinessKind::CatalogPrice),
        "purchase_order" => Ok(BusinessKind::PurchaseOrder),
        "receipt" => Ok(BusinessKind::Receipt),
        "production" => Ok(BusinessKind::Production),
        "sale" => Ok(BusinessKind::Sale),
        "stocktake" => Ok(BusinessKind::Stocktake),
        "waste" => Ok(BusinessKind::Waste),
        "correction" => Ok(BusinessKind::Correction),
        _ => Err(unavailable("지원하지 않는 업무 종류입니다")),
    }
}
fn kind_text(kind: BusinessKind) -> &'static str {
    match kind {
        BusinessKind::CatalogPrice => "catalog_price",
        BusinessKind::PurchaseOrder => "purchase_order",
        BusinessKind::Receipt => "receipt",
        BusinessKind::Production => "production",
        BusinessKind::Sale => "sale",
        BusinessKind::Stocktake => "stocktake",
        BusinessKind::Waste => "waste",
        BusinessKind::Correction => "correction",
    }
}
fn result_material_id(result: &BusinessResult) -> Option<&str> {
    match result {
        BusinessResult::CatalogPrice { material_id, .. }
        | BusinessResult::PurchaseOrder { material_id, .. }
        | BusinessResult::Receipt { material_id, .. }
        | BusinessResult::ReconciledReceipt { material_id, .. } => Some(material_id),
        BusinessResult::Production { .. }
        | BusinessResult::Sale { .. }
        | BusinessResult::Stocktake { .. }
        | BusinessResult::Waste { .. }
        | BusinessResult::Correction { .. } => None,
    }
}
fn same_correction_target(left: &BusinessResult, right: &BusinessResult) -> bool {
    match (left, right) {
        (BusinessResult::Sale { sale_id: a, .. }, BusinessResult::Sale { sale_id: b, .. }) => {
            a == b
        }
        (BusinessResult::CatalogPrice { .. }, BusinessResult::CatalogPrice { .. }) => {
            result_material_id(left) == result_material_id(right)
        }
        _ => false,
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Record {
    pub id: String,
    pub kind: BusinessKind,
    pub created_at: String,
    pub created_sequence: String,
    pub corrects_confirmation_id: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Head {
    pub record_id: String,
    pub current_revision_id: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ConfirmationRow {
    pub id: String,
    pub record_id: String,
    pub revision_id: String,
    pub request_id: String,
    pub recorded_at: String,
    pub seq: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PriceResult {
    pub confirmation_id: String,
    pub price_id: String,
}
pub(super) struct SnapshotRows {
    pub records: Vec<Record>,
    pub revisions: Vec<BusinessRevision>,
    pub heads: Vec<Head>,
    pub confirmations: Vec<ConfirmationRow>,
    pub results: Vec<PriceResult>,
    pub corrections: Vec<BusinessCorrection>,
}

fn status_text(status: BusinessStatus) -> &'static str {
    match status {
        BusinessStatus::Draft => "draft",
        BusinessStatus::Confirmed => "confirmed",
        BusinessStatus::Archived => "archived",
    }
}
fn parse_status(status: &str) -> Result<BusinessStatus, StoreError> {
    match status {
        "draft" => Ok(BusinessStatus::Draft),
        "confirmed" => Ok(BusinessStatus::Confirmed),
        "archived" => Ok(BusinessStatus::Archived),
        _ => Err(unavailable("지원하지 않는 업무 상태입니다")),
    }
}
fn records(connection: &Connection) -> Result<Vec<Record>, StoreError> {
    let rows = connection.prepare("SELECT id,kind,created_at,created_sequence,corrects_confirmation_id FROM business_records ORDER BY created_sequence,id")
        .map_err(unavailable)?.query_map([], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,i64>(3)?,r.get::<_,Option<String>>(4)?)))
        .map_err(unavailable)?.collect::<rusqlite::Result<Vec<_>>>().map_err(unavailable)?;
    rows.into_iter()
        .map(|(id, kind, created_at, seq, corrects)| {
            Ok(Record {
                id,
                kind: parse_kind(&kind)?,
                created_at,
                created_sequence: seq.to_string(),
                corrects_confirmation_id: corrects,
            })
        })
        .collect()
}
fn record(connection: &Connection, id: &str) -> Result<Option<Record>, StoreError> {
    let row = connection.query_row(
        "SELECT id,kind,created_at,created_sequence,corrects_confirmation_id FROM business_records WHERE id=?1",
        [id], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,i64>(3)?,r.get::<_,Option<String>>(4)?)),
    ).optional().map_err(unavailable)?;
    row.map(|(id, kind, created_at, seq, corrects_confirmation_id)| {
        Ok(Record {
            id,
            kind: parse_kind(&kind)?,
            created_at,
            created_sequence: seq.to_string(),
            corrects_confirmation_id,
        })
    })
    .transpose()
}
fn revisions(
    connection: &Connection,
    id: Option<&str>,
) -> Result<Vec<BusinessRevision>, StoreError> {
    let rows = connection.prepare("SELECT id,record_id,previous_revision_id,status,payload_version,payload_json,recorded_at,sequence FROM business_revisions WHERE (?1 IS NULL OR record_id=?1) ORDER BY sequence,id")
        .map_err(unavailable)?.query_map([id], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,Option<String>>(2)?,r.get::<_,String>(3)?,r.get::<_,i64>(4)?,r.get::<_,String>(5)?,r.get::<_,String>(6)?,r.get::<_,i64>(7)?)))
        .map_err(unavailable)?.collect::<rusqlite::Result<Vec<_>>>().map_err(unavailable)?;
    rows.into_iter()
        .map(
            |(id, record_id, previous_revision_id, status, version, json, recorded_at, seq)| {
                let payload: BusinessPayload = serde_json::from_str(&json).map_err(unavailable)?;
                if version != i64::from(payload.version()) {
                    return Err(unavailable("업무 입력 버전이 일치하지 않습니다"));
                }
                normalize_business_payload(payload.clone()).map_err(unavailable)?;
                Ok(BusinessRevision {
                    id,
                    record_id,
                    previous_revision_id,
                    status: parse_status(&status)?,
                    payload,
                    recorded_at,
                    seq: seq.to_string(),
                })
            },
        )
        .collect()
}
fn current_head(connection: &Connection, id: &str) -> Result<BusinessRevision, StoreError> {
    let head: Option<String> = connection
        .query_row(
            "SELECT current_revision_id FROM business_heads WHERE record_id=?1",
            [id],
            |r| r.get(0),
        )
        .optional()
        .map_err(unavailable)?;
    revisions(connection, Some(id))?
        .into_iter()
        .find(|r| Some(&r.id) == head.as_ref())
        .ok_or_else(|| unavailable("업무의 현재 버전 연결이 없습니다"))
}
fn required_draft(
    connection: &Connection,
    id: &str,
    expected: &str,
) -> Result<(Record, BusinessRevision), StoreError> {
    let record =
        record(connection, id)?.ok_or_else(|| validation("업무 기록을 찾을 수 없습니다"))?;
    let head = current_head(connection, id)?;
    if head.payload.kind() != record.kind {
        return Err(unavailable("업무 종류와 입력 종류가 일치하지 않습니다"));
    }
    if head.id != expected {
        return Err(StoreError::StaleState(
            "업무 입력이 변경되었습니다. 최신 내용을 확인하세요".into(),
        ));
    }
    if head.status != BusinessStatus::Draft {
        return Err(validation(
            "임시 저장 상태의 업무만 변경하거나 확정할 수 있습니다",
        ));
    }
    Ok((record, head))
}
fn confirmation_rows(connection: &Connection) -> Result<Vec<ConfirmationRow>, StoreError> {
    connection.prepare("SELECT id,record_id,revision_id,request_id,recorded_at,sequence FROM business_confirmations ORDER BY sequence,id").map_err(unavailable)?
        .query_map([],|r|Ok(ConfirmationRow{id:r.get(0)?,record_id:r.get(1)?,revision_id:r.get(2)?,request_id:r.get(3)?,recorded_at:r.get(4)?,seq:r.get::<_,i64>(5)?.to_string()})).map_err(unavailable)?
        .collect::<rusqlite::Result<_>>().map_err(unavailable)
}
pub(super) fn confirmation(
    connection: &Connection,
    id: &str,
) -> Result<Option<BusinessConfirmation>, StoreError> {
    let row = connection.query_row(
        "SELECT id,record_id,revision_id,request_id,recorded_at,sequence FROM business_confirmations WHERE id=?1",
        [id], |r| Ok(ConfirmationRow{id:r.get(0)?,record_id:r.get(1)?,revision_id:r.get(2)?,request_id:r.get(3)?,recorded_at:r.get(4)?,seq:r.get::<_,i64>(5)?.to_string()}),
    ).optional().map_err(unavailable)?;
    let Some(row) = row else { return Ok(None) };
    let owner = record(connection, &row.record_id)?
        .ok_or_else(|| unavailable("확정 업무 원본이 없습니다"))?;
    let result = match owner.kind {
        BusinessKind::CatalogPrice => connection.query_row("SELECT p.id,p.material_id,p.material_version_id FROM business_catalog_price_results r JOIN material_prices p ON p.id=r.price_id WHERE r.confirmation_id=?1",[id],|r|Ok(BusinessResult::CatalogPrice{price_id:r.get(0)?,material_id:r.get(1)?,material_version_id:r.get(2)?})).map_err(unavailable)?,
        BusinessKind::PurchaseOrder => purchase_order::result(connection,id)?,
        BusinessKind::Receipt => receipt::result(connection,id)?,
        BusinessKind::Production => production::result(connection,id)?,
        BusinessKind::Sale => sale::result(connection,id)?,
        BusinessKind::Stocktake=>stocktake::result(connection,id)?,
        BusinessKind::Waste=>waste::result(connection,id)?,
        BusinessKind::Correction=>correction::result(connection,id)?,
    };
    Ok(Some(BusinessConfirmation {
        id: row.id,
        record_id: row.record_id,
        revision_id: row.revision_id,
        request_id: row.request_id,
        recorded_at: row.recorded_at,
        seq: row.seq,
        result,
    }))
}
fn corrections(connection: &Connection) -> Result<Vec<BusinessCorrection>, StoreError> {
    connection.prepare("SELECT target_confirmation_id,replacement_confirmation_id FROM business_corrections ORDER BY target_confirmation_id").map_err(unavailable)?
        .query_map([],|r|Ok(BusinessCorrection{target_confirmation_id:r.get(0)?,replacement_confirmation_id:r.get(1)?})).map_err(unavailable)?
        .collect::<rusqlite::Result<_>>().map_err(unavailable)
}
fn chain(
    connection: &Connection,
    seed: Option<&str>,
) -> Result<(Vec<BusinessConfirmation>, Vec<BusinessCorrection>), StoreError> {
    let Some(seed) = seed else {
        return Ok((vec![], vec![]));
    };
    let edges = corrections(connection)?;
    let mut root = seed.to_owned();
    let mut seen = HashSet::new();
    loop {
        if !seen.insert(root.clone()) {
            return Err(unavailable("정정 연결이 순환합니다"));
        }
        match edges.iter().find(|e| e.replacement_confirmation_id == root) {
            Some(edge) => root = edge.target_confirmation_id.clone(),
            None => break,
        }
    }
    let mut events = vec![];
    let mut links = vec![];
    seen.clear();
    loop {
        if !seen.insert(root.clone()) {
            return Err(unavailable("정정 연결이 순환합니다"));
        }
        let event = confirmation(connection, &root)?
            .ok_or_else(|| unavailable("정정 원기록 연결이 없습니다"))?;
        if let Some(previous) = events.last() {
            let previous: &BusinessConfirmation = previous;
            if !same_correction_target(&previous.result, &event.result) {
                return Err(unavailable("정정 대상 품목이 일치하지 않습니다"));
            }
        }
        events.push(event);
        match edges.iter().find(|e| e.target_confirmation_id == root) {
            Some(edge) => {
                root = edge.replacement_confirmation_id.clone();
                links.push(edge.clone());
            }
            None => break,
        }
    }
    Ok((events, links))
}
fn own_confirmation(connection: &Connection, id: &str) -> Result<Option<String>, StoreError> {
    connection
        .query_row(
            "SELECT id FROM business_confirmations WHERE record_id=?1",
            [id],
            |r| r.get(0),
        )
        .optional()
        .map_err(unavailable)
}
fn issue(
    issues: &mut Vec<BusinessIssue>,
    field: &str,
    code: &str,
    message: impl std::fmt::Display,
) {
    issues.push(BusinessIssue {
        field: field.into(),
        code: code.into(),
        message: message.to_string(),
    });
}
fn inspect(
    connection: &Connection,
    record: &Record,
    head: &BusinessRevision,
    today: chrono::NaiveDate,
) -> Result<(Vec<BusinessIssue>, Option<BusinessEffect>), StoreError> {
    match &head.payload {
        BusinessPayload::CatalogPrice { .. } => inspect_price(connection, record, head),
        BusinessPayload::PurchaseOrder { data, .. } => {
            purchase_order::inspect_order(connection, data)
        }
        BusinessPayload::Receipt { data, .. } => receipt::inspect_receipt(connection, data, today),
        BusinessPayload::Production { data, .. } => {
            production::inspect_production(connection, data)
        }
        BusinessPayload::Sale { data, .. } => {
            sale::inspect_sale(connection, data, record.corrects_confirmation_id.as_deref())
        }
        BusinessPayload::Stocktake { data, .. } => stocktake::inspect_stocktake(connection, data),
        BusinessPayload::Waste { data, .. } => waste::inspect_waste(connection, data),
        BusinessPayload::Correction { data, .. } => correction::inspect_correction(
            connection,
            data,
            record.corrects_confirmation_id.as_deref(),
            today,
        ),
    }
}
fn inspect_price(
    connection: &Connection,
    record: &Record,
    head: &BusinessRevision,
) -> Result<(Vec<BusinessIssue>, Option<BusinessEffect>), StoreError> {
    let BusinessPayload::CatalogPrice { data, .. } = &head.payload else {
        return Err(unavailable("가격 입력 종류가 올바르지 않습니다"));
    };
    let mut issues = vec![];
    let material = match &data.material_id {
        Some(id) => catalog::current_version(connection, id)?,
        None => None,
    };
    if material.is_none() {
        issue(
            &mut issues,
            "material_id",
            "missing_reference",
            "재료를 선택하세요",
        );
    }
    if let Some(material) = &material {
        if data.material_version_id.as_deref() != Some(&material.id) {
            issue(
                &mut issues,
                "material_version_id",
                "stale_reference",
                "최신 재료 정보를 선택해 다시 저장하세요",
            );
        }
    } else if data.material_version_id.is_none() {
        issue(
            &mut issues,
            "material_version_id",
            "missing_reference",
            "재료 버전을 선택하세요",
        );
    }
    let quantity = match Quantity::parse(&data.quantity) {
        Ok(q) if q.micros() > 0 => Some(q),
        Ok(_) => {
            issue(
                &mut issues,
                "quantity",
                "invalid_quantity",
                "가격 기준 수량은 0보다 커야 합니다",
            );
            None
        }
        Err(e) => {
            issue(&mut issues, "quantity", "invalid_quantity", e);
            None
        }
    };
    let amount = match Money::parse(&data.amount) {
        Ok(a) => Some(a),
        Err(e) => {
            issue(&mut issues, "amount", "invalid_amount", e);
            None
        }
    };
    let unit = match Unit::parse(&data.unit) {
        Ok(u) => Some(u),
        Err(e) => {
            issue(&mut issues, "unit", "invalid_unit", e);
            None
        }
    };
    let mut canonical_quantity = None;
    if let Some(unit) = &unit {
        if let Some(material) = &material
            && !unit.same_dimension(&Unit::parse(&material.unit).map_err(unavailable)?)
        {
            issue(
                &mut issues,
                "unit",
                "incompatible_unit",
                "재료와 같은 차원 단위를 사용하세요",
            );
        }
        if let Some(quantity) = quantity {
            match unit.to_canonical(quantity) {
                Ok(q) => canonical_quantity = Some(q),
                Err(e) => issue(&mut issues, "quantity", "invalid_quantity", e),
            }
        }
    }
    let mut corrected_price = None;
    if let Some(target) = &record.corrects_confirmation_id {
        let event = confirmation(connection, target)?
            .ok_or_else(|| unavailable("정정 원기록이 없습니다"))?;
        if corrections(connection)?
            .iter()
            .any(|e| e.target_confirmation_id == *target)
        {
            issue(
                &mut issues,
                "corrects_confirmation_id",
                "stale_reference",
                "대상은 이미 정정되었습니다. 최신 확정을 확인하세요",
            );
        }
        let BusinessResult::CatalogPrice {
            material_id,
            price_id,
            ..
        } = event.result
        else {
            return Err(unavailable("가격 정정 대상 종류가 다릅니다"));
        };
        if data.material_id.as_deref() != Some(&material_id) {
            issue(
                &mut issues,
                "material_id",
                "wrong_correction_target",
                "정정 대상과 같은 재료를 선택하세요",
            );
        }
        if data.note.trim().is_empty() {
            issue(
                &mut issues,
                "note",
                "required_reason",
                "정정 이유를 입력하세요",
            );
        }
        corrected_price = catalog::prices(connection, Some(&material_id))?
            .into_iter()
            .find(|p| p.id == price_id);
        if corrected_price.is_none() {
            return Err(unavailable("정정 대상 가격 결과가 없습니다"));
        }
    }
    if !issues.is_empty() {
        return Ok((issues, None));
    }
    let material = material.ok_or_else(|| unavailable("재료 검증 결과가 없습니다"))?;
    let effect = BusinessEffect::CatalogPrice {
        material_id: material.material_id.clone(),
        material_version_id: material.id,
        quantity: canonical_quantity
            .ok_or_else(|| unavailable("수량 검증 결과가 없습니다"))?
            .decimal(),
        amount: amount
            .ok_or_else(|| unavailable("금액 검증 결과가 없습니다"))?
            .decimal(),
        unit: unit
            .ok_or_else(|| unavailable("단위 검증 결과가 없습니다"))?
            .canonical()
            .into(),
        corrects_confirmation_id: record.corrects_confirmation_id.clone(),
        previous_price: catalog::prices(connection, Some(&material.material_id))?.pop(),
        corrected_price,
    };
    Ok((issues, Some(effect)))
}
fn detail(
    connection: &Connection,
    id: &str,
    today: chrono::NaiveDate,
) -> Result<Option<BusinessDetail>, StoreError> {
    let Some(record) = record(connection, id)? else {
        return Ok(None);
    };
    let head = current_head(connection, id)?;
    let own = own_confirmation(connection, id)?;
    let seed = own
        .as_deref()
        .or(record.corrects_confirmation_id.as_deref());
    let (confirmations, corrections) = chain(connection, seed)?;
    let issues = if head.status == BusinessStatus::Draft {
        inspect(connection, &record, &head, today)?.0
    } else {
        vec![]
    };
    let material_name = match &head.payload {
        BusinessPayload::CatalogPrice { data, .. } => {
            match (&data.material_id, &data.material_version_id) {
                (Some(material), Some(version)) => catalog::versions(connection, Some(material))?
                    .into_iter()
                    .find(|v| v.id == *version)
                    .map(|v| v.name),
                _ => None,
            }
        }
        BusinessPayload::PurchaseOrder { data, .. } => {
            purchase_order::material_name(connection, data)?
        }
        BusinessPayload::Receipt { data, .. } => receipt::material_name(connection, data)?,
        BusinessPayload::Production { data, .. } => production::material_name(connection, data)?,
        BusinessPayload::Sale { data, .. } => sale::material_name(connection, data)?,
        BusinessPayload::Stocktake { data, .. } => stocktake::material_name(connection, data)?,
        BusinessPayload::Waste { data, .. } => waste::material_name(connection, data)?,
        BusinessPayload::Correction { data, .. } => {
            correction::material_name(connection, data, record.corrects_confirmation_id.as_deref())?
        }
    };
    let item = BusinessSummary {
        id: record.id,
        kind: record.kind,
        head_id: head.id.clone(),
        status: head.status,
        created_at: record.created_at,
        updated_at: head.recorded_at.clone(),
        corrects_confirmation_id: record.corrects_confirmation_id,
        confirmation_id: own,
        effective_confirmation_id: confirmations.last().map(|c| c.id.clone()),
        issue_count: issues.len(),
        material_name,
    };
    Ok(Some(BusinessDetail {
        item,
        head,
        revisions: revisions(connection, Some(id))?,
        confirmations,
        corrections,
        issues,
        state: catalog::state(connection)?,
    }))
}
fn write_result(
    connection: &Connection,
    id: &str,
    today: chrono::NaiveDate,
) -> Result<BusinessWrite, StoreError> {
    let detail = detail(connection, id, today)?
        .ok_or_else(|| unavailable("저장한 업무를 찾을 수 없습니다"))?;
    let confirmation = detail
        .confirmations
        .into_iter()
        .find(|c| Some(&c.id) == detail.item.confirmation_id.as_ref());
    Ok(BusinessWrite {
        item: detail.item,
        head: detail.head,
        confirmation,
        issues: detail.issues,
    })
}
fn append_revision(
    connection: &Connection,
    id: &str,
    previous: Option<&str>,
    status: BusinessStatus,
    payload: &BusinessPayload,
    seq: i64,
    time: &str,
) -> Result<String, StoreError> {
    let revision = Uuid::new_v4().to_string();
    connection.execute("INSERT INTO business_revisions(id,record_id,previous_revision_id,status,payload_version,payload_json,recorded_at,sequence) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
        params![revision,id,previous,status_text(status),payload.version(),serde_json::to_string(payload).map_err(unavailable)?,time,seq]).map_err(unavailable)?;
    connection.execute("INSERT INTO business_heads(record_id,current_revision_id) VALUES(?1,?2) ON CONFLICT(record_id) DO UPDATE SET current_revision_id=excluded.current_revision_id",params![id,revision]).map_err(unavailable)?;
    Ok(revision)
}
fn normalize_meta(meta: &mut BusinessMeta) -> Result<MutationMeta, StoreError> {
    let mut normalized: MutationMeta = meta.clone().into();
    catalog::normalize_meta(&mut normalized)?;
    meta.request_id = normalized.request_id.clone();
    meta.epoch = normalized.epoch.clone();
    meta.expected_revision = normalized.expected_revision.clone();
    Ok(normalized)
}
fn check_target(
    connection: &Connection,
    target: Option<&str>,
    payload: &BusinessPayload,
) -> Result<(), StoreError> {
    if let BusinessPayload::Correction { data, .. } = payload {
        return correction::check_target(connection, target, data);
    }
    if let Some(target) = target {
        if let BusinessPayload::Sale { data, .. } = payload {
            return sale::check_correction(connection, target, data);
        }
        let BusinessPayload::CatalogPrice { data, .. } = payload else {
            return Err(validation(
                "주문 수정·취소에는 가격 정정 연결을 사용할 수 없습니다",
            ));
        };
        let event = confirmation(connection, target)?
            .ok_or_else(|| validation("정정할 확정 기록을 찾을 수 없습니다"))?;
        if corrections(connection)?
            .iter()
            .any(|e| e.target_confirmation_id == target)
        {
            return Err(StoreError::StaleState(
                "대상은 이미 정정되었습니다. 최신 확정을 확인하세요".into(),
            ));
        }
        let BusinessResult::CatalogPrice { material_id, .. } = &event.result else {
            return Err(validation("가격 정정 대상 종류가 다릅니다"));
        };
        if data
            .material_id
            .as_ref()
            .is_some_and(|id| id != material_id)
        {
            return Err(validation("정정 대상의 재료를 변경할 수 없습니다"));
        }
    }
    Ok(())
}

impl Store {
    pub fn business_list(&self, filter: BusinessFilter) -> Result<BusinessList, StoreError> {
        let today = (self.today)();
        let mut items = vec![];
        for record in records(&self.connection)? {
            let detail = detail(&self.connection, &record.id, today)?
                .ok_or_else(|| unavailable("업무 기록이 없습니다"))?;
            let matches = match filter.status {
                BusinessStatusFilter::All => true,
                BusinessStatusFilter::Draft => detail.item.status == BusinessStatus::Draft,
                BusinessStatusFilter::Confirmed => detail.item.status == BusinessStatus::Confirmed,
                BusinessStatusFilter::Archived => detail.item.status == BusinessStatus::Archived,
            };
            if matches && filter.kind.is_none_or(|kind| kind == detail.item.kind) {
                items.push(detail.item)
            }
        }
        Ok(BusinessList {
            items,
            state: catalog::state(&self.connection)?,
        })
    }
    pub fn business_detail(&self, id: &str) -> Result<Option<BusinessDetail>, StoreError> {
        detail(
            &self.connection,
            &catalog::uuid(id, "업무 ID")?,
            (self.today)(),
        )
    }
    pub fn business_create(
        &mut self,
        mut input: CreateBusinessDraft,
    ) -> Result<BusinessMutation, StoreError> {
        let meta = normalize_meta(&mut input.meta)?;
        input.payload = normalize_business_payload(input.payload).map_err(validation)?;
        input.corrects_confirmation_id = input
            .corrects_confirmation_id
            .map(|id| catalog::uuid(&id, "정정 대상 ID"))
            .transpose()?;
        let payload = serde_json::to_string(&input).map_err(unavailable)?;
        let today = (self.today)();
        self.mutate(
            Some(&meta), &meta.request_id, "business.create", &payload,
            move |tx, seq, time, state| {
                check_target(tx, input.corrects_confirmation_id.as_deref(), &input.payload)?;
                let id = Uuid::new_v4().to_string();
                tx.execute(
                    "INSERT INTO business_records(id,kind,created_at,created_sequence,corrects_confirmation_id)
                     VALUES(?1,?2,?3,?4,?5)",
                    params![id, kind_text(input.payload.kind()), time, seq, input.corrects_confirmation_id],
                ).map_err(unavailable)?;
                append_revision(tx, &id, None, BusinessStatus::Draft, &input.payload, seq, time)?;
                Ok(BusinessMutation {
                    result: write_result(tx, &id, today)?,
                    state: state.clone(),
                })
            },
        )
    }
    pub fn business_revise(
        &mut self,
        id: &str,
        mut input: ReviseBusinessDraft,
    ) -> Result<BusinessMutation, StoreError> {
        let id = catalog::uuid(id, "업무 ID")?;
        let meta = normalize_meta(&mut input.meta)?;
        input.expected_head_id = catalog::uuid(&input.expected_head_id, "업무 버전 ID")?;
        input.payload = normalize_business_payload(input.payload).map_err(validation)?;
        let payload = serde_json::to_string(&(id.clone(), &input)).map_err(unavailable)?;
        let today = (self.today)();
        self.mutate(
            Some(&meta),
            &meta.request_id,
            "business.revise",
            &payload,
            move |tx, seq, time, state| {
                let (record, head) = required_draft(tx, &id, &input.expected_head_id)?;
                if record.kind != input.payload.kind() {
                    return Err(validation("저장한 업무의 종류를 바꿀 수 없습니다"));
                }
                if let (
                    BusinessPayload::PurchaseOrder { data: old, .. },
                    BusinessPayload::PurchaseOrder { data: new, .. },
                ) = (&head.payload, &input.payload)
                    && !purchase_order::same_action_target(old, new)
                {
                    return Err(validation(
                        "저장한 주문 동작이나 대상 주문을 바꿀 수 없습니다",
                    ));
                }
                check_target(
                    tx,
                    record.corrects_confirmation_id.as_deref(),
                    &input.payload,
                )?;
                append_revision(
                    tx,
                    &id,
                    Some(&head.id),
                    BusinessStatus::Draft,
                    &input.payload,
                    seq,
                    time,
                )?;
                Ok(BusinessMutation {
                    result: write_result(tx, &id, today)?,
                    state: state.clone(),
                })
            },
        )
    }
    pub fn business_preview(
        &self,
        id: &str,
        input: PreviewBusiness,
    ) -> Result<BusinessPreview, StoreError> {
        let id = catalog::uuid(id, "업무 ID")?;
        let expected = catalog::uuid(&input.expected_head_id, "업무 버전 ID")?;
        let (record, head) = required_draft(&self.connection, &id, &expected)?;
        let (issues, effect) = inspect(&self.connection, &record, &head, (self.today)())?;
        Ok(BusinessPreview {
            record_id: id,
            head_id: head.id,
            state: catalog::state(&self.connection)?,
            can_confirm: issues.is_empty(),
            issues,
            effect,
        })
    }
    pub fn business_confirm(
        &mut self,
        id: &str,
        mut input: ConfirmBusiness,
    ) -> Result<BusinessMutation, StoreError> {
        let id = catalog::uuid(id, "업무 ID")?;
        let meta = normalize_meta(&mut input.meta)?;
        input.expected_head_id = catalog::uuid(&input.expected_head_id, "업무 버전 ID")?;
        let payload = serde_json::to_string(&(id.clone(), &input)).map_err(unavailable)?;
        let today = (self.today)();
        self.mutate(
            Some(&meta), &meta.request_id, "business.confirm", &payload,
            move |tx, seq, time, state| {
                let (record, head) = required_draft(tx, &id, &input.expected_head_id)?;
                check_target(tx, record.corrects_confirmation_id.as_deref(), &head.payload)?;
                let (issues, _) = inspect(tx, &record, &head, today)?;
                if !issues.is_empty() {
                    let message = issues.iter().map(|i| i.message.as_str()).collect::<Vec<_>>().join("; ");
                    if issues.iter().any(|i| i.code == "stale_reference") {
                        return Err(StoreError::StaleState(message));
                    }
                    return Err(validation(message));
                }
                let revision = append_revision(tx, &id, Some(&head.id), BusinessStatus::Confirmed, &head.payload, seq, time)?;
                let event = Uuid::new_v4().to_string();
                tx.execute(
                    "INSERT INTO business_confirmations(id,record_id,revision_id,request_id,recorded_at,sequence)
                     VALUES(?1,?2,?3,?4,?5,?6)",
                    params![event, id, revision, input.meta.request_id, time, seq],
                ).map_err(unavailable)?;
                match &head.payload {
                    BusinessPayload::CatalogPrice { data, .. } => {
                        let price = catalog::apply_catalog_price(tx, catalog::PriceInput {
                            material_id: data.material_id.as_deref().ok_or_else(||validation("재료를 선택하세요"))?,
                            expected_version_id: data.material_version_id.as_deref().ok_or_else(||validation("재료 버전을 선택하세요"))?,
                            quantity: &data.quantity, unit: &data.unit, amount: &data.amount,
                        }, seq, time)?;
                        tx.execute("INSERT INTO business_catalog_price_results(confirmation_id,price_id) VALUES(?1,?2)",params![event,price.id]).map_err(unavailable)?;
                    }
                    BusinessPayload::PurchaseOrder { data, .. } => {
                        purchase_order::apply_order(tx,data,&event,seq,time)?;
                    }
                    BusinessPayload::Receipt {data,..} => receipt::apply_receipt(tx,data,&event,seq,time,today)?,
                    BusinessPayload::Production {data,..} => production::apply_production(tx,data,&event,seq,time)?,
                    BusinessPayload::Sale {data,..} => sale::apply_sale(tx,data,record.corrects_confirmation_id.as_deref(),&event)?,
                    BusinessPayload::Stocktake{data,..}=>stocktake::apply_stocktake(tx,data,&event,seq,time)?,
                    BusinessPayload::Waste{data,..}=>waste::apply_waste(tx,data,&event,seq,time)?,
                    BusinessPayload::Correction{data,..}=>correction::apply_correction(tx,data,record.corrects_confirmation_id.as_deref().ok_or_else(||validation("정정 대상 원확정을 선택하세요"))?,&ConfirmationRow{id:event.clone(),record_id:id.clone(),revision_id:revision.clone(),request_id:input.meta.request_id.clone(),recorded_at:time.into(),seq:seq.to_string()},seq,time,today)?,
                }
                if let Some(target) = record.corrects_confirmation_id
                    && !matches!(head.payload,BusinessPayload::Correction{..}) {
                    tx.execute(
                        "INSERT INTO business_corrections(target_confirmation_id,replacement_confirmation_id) VALUES(?1,?2)",
                        params![target, event],
                    ).map_err(unavailable)?;
                }
                Ok(BusinessMutation {
                    result: write_result(tx, &id, today)?,
                    state: state.clone(),
                })
            },
        )
    }
    pub fn business_archive(
        &mut self,
        id: &str,
        mut input: ArchiveBusiness,
    ) -> Result<BusinessMutation, StoreError> {
        let id = catalog::uuid(id, "업무 ID")?;
        let meta = normalize_meta(&mut input.meta)?;
        input.expected_head_id = catalog::uuid(&input.expected_head_id, "업무 버전 ID")?;
        let payload = serde_json::to_string(&(id.clone(), &input)).map_err(unavailable)?;
        let today = (self.today)();
        self.mutate(
            Some(&meta),
            &meta.request_id,
            "business.archive",
            &payload,
            move |tx, seq, time, state| {
                let (_, head) = required_draft(tx, &id, &input.expected_head_id)?;
                append_revision(
                    tx,
                    &id,
                    Some(&head.id),
                    BusinessStatus::Archived,
                    &head.payload,
                    seq,
                    time,
                )?;
                Ok(BusinessMutation {
                    result: write_result(tx, &id, today)?,
                    state: state.clone(),
                })
            },
        )
    }
}

pub(super) fn snapshot_rows(connection: &Connection) -> Result<SnapshotRows, StoreError> {
    let heads = connection
        .prepare("SELECT record_id,current_revision_id FROM business_heads ORDER BY record_id")
        .map_err(unavailable)?
        .query_map([], |r| {
            Ok(Head {
                record_id: r.get(0)?,
                current_revision_id: r.get(1)?,
            })
        })
        .map_err(unavailable)?
        .collect::<rusqlite::Result<_>>()
        .map_err(unavailable)?;
    let results=connection.prepare("SELECT confirmation_id,price_id FROM business_catalog_price_results ORDER BY confirmation_id").map_err(unavailable)?
        .query_map([],|r|Ok(PriceResult{confirmation_id:r.get(0)?,price_id:r.get(1)?})).map_err(unavailable)?.collect::<rusqlite::Result<_>>().map_err(unavailable)?;
    Ok(SnapshotRows {
        records: records(connection)?,
        revisions: revisions(connection, None)?,
        heads,
        confirmations: confirmation_rows(connection)?,
        results,
        corrections: corrections(connection)?,
    })
}

pub(super) fn migrate_business(connection: &Connection) -> anyhow::Result<()> {
    let rows = connection
        .prepare("SELECT sequence,payload FROM snapshot_outbox ORDER BY sequence")?
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (seq, payload) in rows {
        snapshot::validate_source(&payload, seq, 3)?;
    }
    connection.execute_batch(
        "CREATE TABLE business_records(
            id TEXT PRIMARY KEY NOT NULL,kind TEXT NOT NULL,
            created_at TEXT NOT NULL,created_sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),
            corrects_confirmation_id TEXT REFERENCES business_confirmations(id));
         CREATE TABLE business_revisions(
            id TEXT PRIMARY KEY NOT NULL,record_id TEXT NOT NULL REFERENCES business_records(id),
            previous_revision_id TEXT,status TEXT NOT NULL CHECK(status IN('draft','confirmed','archived')),
            payload_version INTEGER NOT NULL CHECK(payload_version=1),payload_json TEXT NOT NULL,
            recorded_at TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),
            UNIQUE(record_id,id),FOREIGN KEY(record_id,previous_revision_id) REFERENCES business_revisions(record_id,id));
         CREATE TABLE business_heads(
            record_id TEXT PRIMARY KEY NOT NULL REFERENCES business_records(id),current_revision_id TEXT NOT NULL,
            FOREIGN KEY(record_id,current_revision_id) REFERENCES business_revisions(record_id,id));
         CREATE TABLE business_confirmations(
            id TEXT PRIMARY KEY NOT NULL,record_id TEXT NOT NULL UNIQUE REFERENCES business_records(id),
            revision_id TEXT NOT NULL UNIQUE,request_id TEXT NOT NULL UNIQUE REFERENCES mutation_requests(request_id) DEFERRABLE INITIALLY DEFERRED,
            recorded_at TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),
            FOREIGN KEY(record_id,revision_id) REFERENCES business_revisions(record_id,id));
         CREATE TABLE business_catalog_price_results(
            confirmation_id TEXT PRIMARY KEY NOT NULL REFERENCES business_confirmations(id),
            price_id TEXT NOT NULL UNIQUE REFERENCES material_prices(id));
         CREATE TABLE business_corrections(
            target_confirmation_id TEXT PRIMARY KEY NOT NULL REFERENCES business_confirmations(id),
            replacement_confirmation_id TEXT NOT NULL UNIQUE REFERENCES business_confirmations(id),
            CHECK(target_confirmation_id<>replacement_confirmation_id));
         CREATE INDEX business_revisions_record ON business_revisions(record_id,sequence);
         PRAGMA user_version=4;"
    )?;
    Ok(())
}
pub(super) fn validate_business_schema(connection: &Connection) -> anyhow::Result<()> {
    connection.prepare("SELECT id,kind,created_at,created_sequence,corrects_confirmation_id FROM business_records LIMIT 0")?;
    connection.prepare("SELECT id,record_id,previous_revision_id,status,payload_version,payload_json,recorded_at,sequence FROM business_revisions LIMIT 0")?;
    connection.prepare("SELECT record_id,current_revision_id FROM business_heads LIMIT 0")?;
    connection.prepare("SELECT id,record_id,revision_id,request_id,recorded_at,sequence FROM business_confirmations LIMIT 0")?;
    connection
        .prepare("SELECT confirmation_id,price_id FROM business_catalog_price_results LIMIT 0")?;
    connection.prepare("SELECT target_confirmation_id,replacement_confirmation_id FROM business_corrections LIMIT 0")?;
    for record in records(connection)? {
        detail(connection, &record.id, chrono::Local::now().date_naive())?;
    }
    revisions(connection, None)?;
    let invalid:i64=connection.query_row("SELECT COUNT(*) FROM business_confirmations c JOIN business_revisions r ON r.id=c.revision_id WHERE r.status<>'confirmed' OR c.sequence<>r.sequence",[],|r|r.get(0))?;
    if invalid != 0 {
        anyhow::bail!("업무 확정과 버전 연결이 일치하지 않습니다")
    }
    Ok(())
}
