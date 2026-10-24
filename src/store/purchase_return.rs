//! Returns retain the original receipt while withdrawing current average value.
use super::{
    Store, StoreError, business, catalog, correction, holding, receipt, stocktake, unavailable,
};
use crate::{
    business::{
        BusinessEffect, BusinessIssue, BusinessKind, BusinessPayload, BusinessResult,
        BusinessStatus,
    },
    catalog::MaterialVersion,
    correction::{
        CorrectionEventValue, CorrectionHistory, CorrectionPlanRef, CorrectionRefKind,
        CorrectionSubjectKind,
    },
    domain::{Money, Quantity, Unit},
    purchase_return::*,
    receipt::{ReceiptRecord, ReceiptSourceDraft},
};
use rusqlite::{Connection, params};
use std::collections::HashSet;
use uuid::Uuid;

fn invalid(e: impl std::fmt::Display) -> StoreError {
    StoreError::Validation(e.to_string())
}
fn issue(
    field: &str,
    code: &str,
    message: impl ToString,
) -> (Vec<BusinessIssue>, Option<BusinessEffect>) {
    (
        vec![BusinessIssue {
            field: field.into(),
            code: code.into(),
            message: message.to_string(),
        }],
        None,
    )
}
fn reference_parts(r: &CorrectionPlanRef) -> Result<(&str, &str), StoreError> {
    match r {
        CorrectionPlanRef::Existing {
            entity: CorrectionRefKind::Receipt,
            id,
            version_id,
        } => Ok((id, version_id)),
        _ => Err(unavailable("반품의 실제 원입고 버전이 없습니다")),
    }
}

/// `before` bounds the logical return prefix, independently of history's as-of revision.
pub(super) fn source_from_history(
    history: &CorrectionHistory,
    id: &str,
    before: Option<i64>,
) -> Result<Option<PurchaseReturnSourceState>, StoreError> {
    let Some(event) = history.events.iter().find(|e| {
        e.reference.kind == CorrectionSubjectKind::Receipt && e.reference.original_id == id
    }) else {
        return Ok(None);
    };
    let Some(CorrectionEventValue::Receipt { input, .. }) = &event.effective_value else {
        return Ok(None);
    };
    let (material_id, version) = match &input.source {
        ReceiptSourceDraft::Direct {
            material_id: Some(id),
            material_version_id: Some(version),
        } => (id.as_str(), version.as_str()),
        ReceiptSourceDraft::Order {
            order_id: Some(order),
            material_version_id: Some(version),
            ..
        } => {
            let o = history
                .orders
                .iter()
                .find(|o| &o.order_id == order)
                .ok_or_else(|| unavailable("원입고 주문이 없습니다"))?;
            (o.material_id.as_str(), version.as_str())
        }
        _ => return Err(unavailable("유효 원입고의 품목 참조가 없습니다")),
    };
    let material = history
        .materials
        .iter()
        .find(|m| m.id == version && m.material_id == material_id)
        .ok_or_else(|| unavailable("원입고 품목 버전이 없습니다"))?;
    let quantity = Unit::parse(&input.unit)
        .map_err(unavailable)?
        .to_canonical(Quantity::parse(&input.quantity).map_err(unavailable)?)
        .map_err(unavailable)?;
    let mut returned = 0i128;
    for e in &history.events {
        if before.is_some_and(|n| e.reference.logical_seq.parse::<i64>().is_ok_and(|s| s >= n)) {
            continue;
        }
        if let Some(CorrectionEventValue::PurchaseReturn { input, result }) = &e.effective_value
            && input.receipt_id.as_deref() == Some(id)
        {
            returned = returned
                .checked_add(i128::from(
                    Quantity::parse(&result.quantity)
                        .map_err(unavailable)?
                        .micros(),
                ))
                .ok_or_else(|| unavailable("반품 합계 범위를 넘습니다"))?;
        }
    }
    if returned > i128::from(quantity.micros()) {
        return Err(unavailable("유효 반품 합계가 원입고 한도를 넘습니다"));
    }
    let returned = i64::try_from(returned).map_err(unavailable)?;
    Ok(Some(PurchaseReturnSourceState {
        reference: CorrectionPlanRef::Existing {
            entity: CorrectionRefKind::Receipt,
            id: id.into(),
            version_id: event.reference.version_id.clone(),
        },
        material_id: material.material_id.clone(),
        material_version_id: material.id.clone(),
        material_name: material.name.clone(),
        unit: material.canonical_unit.clone(),
        quantity: quantity.decimal(),
        amount: Money::parse(&input.amount).map_err(unavailable)?.decimal(),
        returned_quantity: Quantity::from_micros(returned)
            .map_err(unavailable)?
            .decimal(),
        remaining_quantity: Quantity::from_micros(quantity.micros() - returned)
            .map_err(unavailable)?
            .decimal(),
        logical_seq: event.reference.logical_seq.clone(),
        seq: event.reference.seq.clone(),
    }))
}
pub(super) fn check_source(
    connection: &Connection,
    data: &DraftPurchaseReturnV1,
) -> Result<(), StoreError> {
    let Some(id) = data.receipt_id.as_deref() else {
        return Ok(());
    };
    let Some(expected) = data.expected_receipt_version_id.as_deref() else {
        return Ok(());
    };
    let history = correction::effective_history(connection, chrono::Local::now().date_naive())?;
    let source = source_from_history(&history, id, None)?;
    if source
        .as_ref()
        .map(|s| reference_parts(&s.reference).map(|(_, v)| v))
        .transpose()?
        != Some(expected)
    {
        return Err(StoreError::StaleState(
            "원입고의 유효 버전이 바뀌었습니다. 입력을 유지하고 현재 원입고를 확인하세요".into(),
        ));
    }
    Ok(())
}
pub(super) fn material_name(
    connection: &Connection,
    data: &DraftPurchaseReturnV1,
) -> Result<Option<String>, StoreError> {
    let Some(id) = data.receipt_id.as_deref() else {
        return Ok(None);
    };
    let h = correction::effective_history(connection, chrono::Local::now().date_naive())?;
    Ok(source_from_history(&h, id, None)?.map(|s| s.material_name))
}
pub(super) fn inspect_purchase_return(
    connection: &Connection,
    data: &DraftPurchaseReturnV1,
) -> Result<(Vec<BusinessIssue>, Option<BusinessEffect>), StoreError> {
    let Some(id) = data.receipt_id.as_deref() else {
        return Ok(issue(
            "receipt_id",
            "required",
            "반품할 원입고를 선택하세요",
        ));
    };
    let Some(original) = receipt::records(connection, None, Some(id))?.pop() else {
        return Ok(issue(
            "receipt_id",
            "invalid_reference",
            "원입고를 찾을 수 없습니다",
        ));
    };
    let history = correction::effective_history(connection, chrono::Local::now().date_naive())?;
    let Some(source) = source_from_history(&history, id, None)? else {
        return Ok(issue(
            "receipt_id",
            "invalid_reference",
            "취소된 원입고에는 반품을 등록할 수 없습니다",
        ));
    };
    if data.expected_receipt_version_id.as_deref() != Some(reference_parts(&source.reference)?.1) {
        return Ok(issue(
            "expected_receipt_version_id",
            "stale_reference",
            "현재 원입고 버전을 다시 확인하세요",
        ));
    }
    let current = holding::current_input(connection, &source.material_id)?;
    match calculate_purchase_return(
        data,
        &PurchaseReturnContext {
            original,
            source,
            holding: current,
        },
    ) {
        Ok(PurchaseReturnCheck::Ready(purchase_return)) => Ok((
            vec![],
            Some(BusinessEffect::PurchaseReturn { purchase_return }),
        )),
        Ok(PurchaseReturnCheck::NeedsReconciliation(c)) => {
            Ok(issue(&c.field, "needs_reconciliation", c.message))
        }
        Err(e) => Ok(issue("quantity", "invalid_input", e)),
    }
}
pub(super) fn apply_purchase_return(
    connection: &Connection,
    data: &DraftPurchaseReturnV1,
    confirmation: &str,
    seq: i64,
    time: &str,
) -> Result<(), StoreError> {
    check_source(connection, data)?;
    let (issues, effect) = inspect_purchase_return(connection, data)?;
    if !issues.is_empty() {
        return Err(invalid(
            issues
                .iter()
                .map(|i| i.message.as_str())
                .collect::<Vec<_>>()
                .join("; "),
        ));
    }
    let Some(BusinessEffect::PurchaseReturn {
        purchase_return: effect,
    }) = effect
    else {
        return Err(unavailable("반품 계산이 없습니다"));
    };
    let id = Uuid::new_v4().to_string();
    let (receipt, version) = reference_parts(&effect.source.reference)?;
    let before = holding::current_input(connection, &effect.material_id)?;
    connection.execute("INSERT INTO purchase_returns(id,confirmation_id,receipt_id,receipt_version_id,material_id,material_version_id,before_holding_version_id,quantity_micros,deducted_value_micros,note,effect_json,recorded_at,sequence) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",params![id,confirmation,receipt,version,effect.material_id,effect.material_version_id,effect.before.source_id,Quantity::parse(&effect.quantity).map_err(unavailable)?.micros(),Money::parse(&effect.deducted_value).map_err(unavailable)?.micros(),data.note,serde_json::to_string(&effect).map_err(unavailable)?,time,seq]).map_err(unavailable)?;
    holding::append(
        connection,
        holding::AppendHolding {
            material_id: &effect.material_id,
            material_version_id: &effect.material_version_id,
            receipt_id: None,
            production_effect_id: None,
            observation_version_id: None,
            reconciliation_effect_id: None,
            waste_id: None,
            purchase_return_id: Some(&id),
            before: &before,
            after: &effect.after,
            seq,
            time,
        },
    )?;
    Ok(())
}

struct Stored {
    id: String,
    confirmation: String,
    record: String,
    receipt: String,
    version: String,
    material: String,
    material_version: String,
    before: String,
    quantity: i64,
    value: i64,
    note: String,
    json: String,
    time: String,
    seq: i64,
}
pub(super) fn records(
    connection: &Connection,
    material: Option<&str>,
    id: Option<&str>,
) -> Result<Vec<PurchaseReturnRecord>, StoreError> {
    let rows=connection.prepare("SELECT r.id,r.confirmation_id,c.record_id,r.receipt_id,r.receipt_version_id,r.material_id,r.material_version_id,r.before_holding_version_id,r.quantity_micros,r.deducted_value_micros,r.note,r.effect_json,r.recorded_at,r.sequence FROM purchase_returns r JOIN business_confirmations c ON c.id=r.confirmation_id WHERE (?1 IS NULL OR r.material_id=?1) AND (?2 IS NULL OR r.id=?2) ORDER BY r.sequence,r.id").map_err(unavailable)?
        .query_map(params![material,id],|r|Ok(Stored{id:r.get(0)?,confirmation:r.get(1)?,record:r.get(2)?,receipt:r.get(3)?,version:r.get(4)?,material:r.get(5)?,material_version:r.get(6)?,before:r.get(7)?,quantity:r.get(8)?,value:r.get(9)?,note:r.get(10)?,json:r.get(11)?,time:r.get(12)?,seq:r.get(13)?})).map_err(unavailable)?.collect::<rusqlite::Result<Vec<_>>>().map_err(unavailable)?;
    let holdings = holding::versions(connection, material)?;
    rows.into_iter()
        .map(|r| {
            let mut effect: PurchaseReturnEffect =
                serde_json::from_str(&r.json).map_err(unavailable)?;
            let (source, version) = reference_parts(&effect.source.reference)?;
            if effect.after.source_id.is_some()
                || source != r.receipt
                || version != r.version
                || effect.material_id != r.material
                || effect.material_version_id != r.material_version
                || effect.before.source_id.as_deref() != Some(&r.before)
                || effect.quantity
                    != Quantity::from_micros(r.quantity)
                        .map_err(unavailable)?
                        .decimal()
                || effect.deducted_value
                    != Money::from_micros(r.value).map_err(unavailable)?.decimal()
            {
                return Err(unavailable("반품 원본과 물리 저장 값이 다릅니다"));
            }
            let mut matches = holdings
                .iter()
                .filter(|h| h.purchase_return_id.as_deref() == Some(r.id.as_str()));
            let h = matches
                .next()
                .ok_or_else(|| unavailable("반품 보유 결과가 없습니다"))?;
            if matches.next().is_some()
                || h.material_id != r.material
                || h.material_version_id != r.material_version
                || h.previous_version_id.as_deref() != Some(r.before.as_str())
                || h.quantity != effect.after.quantity
                || Some(h.value.as_str()) != effect.after.value.as_deref()
                || h.seq != r.seq.to_string()
                || h.recorded_at != r.time
            {
                return Err(unavailable("반품의 실제 보유 연결이 다릅니다"));
            }
            effect.after.source_id = Some(h.id.clone());
            Ok(PurchaseReturnRecord {
                id: r.id,
                confirmation_id: r.confirmation,
                business_record_id: r.record,
                effect,
                note: r.note,
                recorded_at: r.time,
                seq: r.seq.to_string(),
            })
        })
        .collect()
}
pub(super) fn snapshot_rows(c: &Connection) -> Result<Vec<PurchaseReturnRecord>, StoreError> {
    records(c, None, None)
}
pub(super) fn result(c: &Connection, confirmation: &str) -> Result<BusinessResult, StoreError> {
    let id = c
        .query_row(
            "SELECT id FROM purchase_returns WHERE confirmation_id=?1",
            [confirmation],
            |r| r.get(0),
        )
        .map_err(unavailable)?;
    Ok(BusinessResult::PurchaseReturn {
        purchase_return_id: id,
    })
}
fn effective(history: &CorrectionHistory, id: &str) -> Option<PurchaseReturnEffectiveState> {
    let event = history.events.iter().find(|e| {
        e.reference.kind == CorrectionSubjectKind::PurchaseReturn && e.reference.original_id == id
    })?;
    if event.reference.version_id == id {
        return None;
    }
    Some(match &event.effective_value {
        Some(CorrectionEventValue::PurchaseReturn { input, result }) => {
            PurchaseReturnEffectiveState::Active {
                version_id: event.reference.version_id.clone(),
                effect: Box::new(result.clone()),
                note: input.note.clone(),
            }
        }
        _ => PurchaseReturnEffectiveState::Void {
            version_id: event.reference.version_id.clone(),
        },
    })
}
impl Store {
    pub fn purchase_return_source(
        &self,
        id: &str,
    ) -> Result<Option<PurchaseReturnSource>, StoreError> {
        let id = catalog::uuid(id, "원입고 ID")?;
        let Some(original) = receipt::records(&self.connection, None, Some(&id))?.pop() else {
            return Ok(None);
        };
        let h = correction::effective_history(&self.connection, (self.today)())?;
        Ok(Some(PurchaseReturnSource {
            original,
            current: source_from_history(&h, &id, None)?,
            state: catalog::state(&self.connection)?,
        }))
    }
    pub fn purchase_return_list(
        &self,
        filter: PurchaseReturnFilter,
    ) -> Result<PurchaseReturnList, StoreError> {
        let material = filter
            .material_id
            .as_deref()
            .map(|s| catalog::uuid(s, "품목 ID"))
            .transpose()?;
        let receipt = filter
            .receipt_id
            .as_deref()
            .map(|s| catalog::uuid(s, "원입고 ID"))
            .transpose()?;
        let h = correction::effective_history(&self.connection, (self.today)())?;
        let mut items = records(&self.connection, None, None)?
            .into_iter()
            .map(|item| PurchaseReturnSummary {
                effective: effective(&h, &item.id),
                item,
            })
            .filter(|item| {
                let e = match &item.effective {
                    Some(PurchaseReturnEffectiveState::Active { effect, .. }) => effect.as_ref(),
                    _ => &item.item.effect,
                };
                material.as_ref().is_none_or(|m| m == &e.material_id)
                    && receipt
                        .as_ref()
                        .is_none_or(|id| correction::ref_id(&e.source.reference) == id)
            })
            .collect::<Vec<_>>();
        items.reverse();
        Ok(PurchaseReturnList {
            items,
            state: catalog::state(&self.connection)?,
        })
    }
    pub fn purchase_return_detail(
        &self,
        id: &str,
    ) -> Result<Option<PurchaseReturnDetail>, StoreError> {
        let id = catalog::uuid(id, "반품 ID")?;
        let Some(item) = records(&self.connection, None, Some(&id))?.pop() else {
            return Ok(None);
        };
        let h = correction::effective_history(&self.connection, (self.today)())?;
        Ok(Some(PurchaseReturnDetail {
            effective: effective(&h, &id),
            item,
            state: catalog::state(&self.connection)?,
        }))
    }
}

pub(super) struct Validation<'a> {
    pub records: &'a [PurchaseReturnRecord],
    pub receipts: &'a [ReceiptRecord],
    pub holdings: &'a [holding::HoldingVersion],
    pub observations: &'a [stocktake::VersionRow],
    pub material_versions: &'a [MaterialVersion],
    pub business_records: &'a [business::Record],
    pub revisions: &'a [crate::business::BusinessRevision],
    pub confirmations: &'a [business::ConfirmationRow],
    pub mutations: &'a [(&'a str, &'a str, &'a str)],
    pub correction: Option<&'a correction::Validation<'a>>,
    pub sequence: i64,
}
pub(super) fn validate_snapshot(s: Validation<'_>) -> Result<(), StoreError> {
    let bad = || unavailable("반품 원본·원입고·계산·확정 근거가 일치하지 않습니다");
    let mut ids = HashSet::new();
    let mut confirmations = HashSet::new();
    for r in s.records {
        for id in [
            &r.id,
            &r.confirmation_id,
            &r.business_record_id,
            &r.effect.material_id,
            &r.effect.material_version_id,
        ] {
            if Uuid::parse_str(id).map_err(unavailable)?.to_string() != *id {
                return Err(bad());
            }
        }
        let seq = r.seq.parse::<i64>().map_err(unavailable)?;
        if seq <= 0
            || seq > s.sequence
            || seq.to_string() != r.seq
            || chrono::DateTime::parse_from_rfc3339(&r.recorded_at).is_err()
            || !ids.insert(&r.id)
            || !confirmations.insert(&r.confirmation_id)
        {
            return Err(bad());
        }
        let c = s
            .confirmations
            .iter()
            .find(|c| c.id == r.confirmation_id)
            .ok_or_else(bad)?;
        let b = s
            .business_records
            .iter()
            .find(|b| b.id == r.business_record_id)
            .ok_or_else(bad)?;
        let revision = s
            .revisions
            .iter()
            .find(|v| v.id == c.revision_id)
            .ok_or_else(bad)?;
        if c.record_id != b.id
            || c.seq != r.seq
            || c.recorded_at != r.recorded_at
            || b.kind != BusinessKind::PurchaseReturn
            || b.corrects_confirmation_id.is_some()
            || revision.record_id != b.id
            || revision.status != BusinessStatus::Confirmed
            || revision.seq != r.seq
            || revision.recorded_at != r.recorded_at
            || s.mutations
                .iter()
                .filter(|(n, op, id)| {
                    *n == r.seq && *op == "business.confirm" && *id == c.request_id
                })
                .count()
                != 1
        {
            return Err(bad());
        }
        let BusinessPayload::PurchaseReturn { version: 1, data } = &revision.payload else {
            return Err(bad());
        };
        let source_id = reference_parts(&r.effect.source.reference)?.0;
        let original = s
            .receipts
            .iter()
            .find(|o| o.id == source_id)
            .ok_or_else(bad)?;
        let correction = s.correction.ok_or_else(bad)?;
        let history =
            correction::history_from_snapshot(correction, seq - 1, chrono::NaiveDate::MAX)?;
        let source = source_from_history(&history, source_id, Some(seq))?.ok_or_else(bad)?;
        if source.logical_seq.parse::<i64>().map_err(unavailable)? >= seq {
            return Err(bad());
        }
        let before = holding::input_at(
            s.holdings,
            s.observations,
            correction.projection_versions,
            &source.material_id,
            seq - 1,
        )?;
        let PurchaseReturnCheck::Ready(mut expected) = calculate_purchase_return(
            data,
            &PurchaseReturnContext {
                original: original.clone(),
                source,
                holding: before,
            },
        )
        .map_err(unavailable)?
        else {
            return Err(bad());
        };
        let after = s
            .holdings
            .iter()
            .find(|h| h.purchase_return_id.as_deref() == Some(r.id.as_str()))
            .ok_or_else(bad)?;
        expected.after.source_id = Some(after.id.clone());
        if *expected != r.effect
            || data.note != r.note
            || after.material_id != r.effect.material_id
            || after.material_version_id != r.effect.material_version_id
            || after.previous_version_id != r.effect.before.source_id
            || after.quantity != r.effect.after.quantity
            || Some(after.value.as_str()) != r.effect.after.value.as_deref()
            || after.seq != r.seq
            || after.recorded_at != r.recorded_at
            || !s.material_versions.iter().any(|m| {
                m.id == r.effect.material_version_id && m.material_id == r.effect.material_id
            })
        {
            return Err(bad());
        }
    }
    for c in s.confirmations {
        if s.business_records
            .iter()
            .any(|b| b.id == c.record_id && b.kind == BusinessKind::PurchaseReturn)
            && !confirmations.contains(&c.id)
        {
            return Err(bad());
        }
    }
    Ok(())
}
pub(super) fn validate_purchase_return_schema(connection: &Connection) -> anyhow::Result<()> {
    let sequence = catalog::state(connection)?.revision.parse::<i64>()?;
    super::snapshot::with_correction_validation(connection, sequence, |s| {
        validate_snapshot(Validation {
            records: s.purchase_returns,
            receipts: s.receipts,
            holdings: s.holdings,
            observations: s.observation_versions,
            material_versions: s.materials,
            business_records: s.business_records,
            revisions: s.revisions,
            confirmations: s.confirmations,
            mutations: s.mutations,
            correction: Some(&s),
            sequence,
        })
    })?;
    Ok(())
}

pub(super) fn validate_physical_columns(connection: &Connection) -> Result<(), StoreError> {
    let rows = snapshot_rows(connection)?;
    for row in rows {
        let (json,seq,time):(String,i64,String)=connection.query_row("SELECT v.payload_json,c.sequence,c.recorded_at FROM business_confirmations c JOIN business_revisions v ON v.id=c.revision_id WHERE c.id=?1",[&row.confirmation_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(unavailable)?;
        let BusinessPayload::PurchaseReturn { version: 1, data } =
            serde_json::from_str(&json).map_err(unavailable)?
        else {
            return Err(unavailable("반품 확정 입력 종류가 다릅니다"));
        };
        let (source, version) = reference_parts(&row.effect.source.reference)?;
        if data.note != row.note
            || data.quantity != row.effect.input_quantity
            || data.unit != row.effect.input_unit
            || data.receipt_id.as_deref() != Some(source)
            || data.expected_receipt_version_id.as_deref() != Some(version)
            || row.seq != seq.to_string()
            || row.recorded_at != time
        {
            return Err(unavailable("반품 원입력과 실제 저장 값이 다릅니다"));
        }
    }
    Ok(())
}
