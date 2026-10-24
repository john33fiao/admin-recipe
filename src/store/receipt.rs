use super::{Store, StoreError, catalog, holding, purchase_order, snapshot, unavailable};
use crate::{
    business::{BusinessEffect, BusinessIssue, BusinessResult},
    domain::{Money, Quantity, Unit},
    holding::project_receipt,
    purchase_order::{
        OrderForReceipt, OrderReceiptClosure, OrderReferences, OrderStatus, ReceiptReferences,
        ReceiptResult, close_order,
    },
    receipt::*,
};
use chrono::NaiveDate;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

fn signed_quantity(micros: i64) -> Result<String, StoreError> {
    let magnitude = Quantity::from_micros(micros.abs())
        .map_err(unavailable)?
        .decimal();
    Ok(if micros < 0 {
        format!("-{magnitude}")
    } else {
        magnitude
    })
}
fn validation(e: impl std::fmt::Display) -> StoreError {
    StoreError::Validation(e.to_string())
}
fn issue(issues: &mut Vec<BusinessIssue>, field: &str, code: &str, e: impl std::fmt::Display) {
    issues.push(BusinessIssue {
        field: field.into(),
        code: code.into(),
        message: e.to_string(),
    });
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ClosureRow {
    pub order_id: String,
    pub receipt_id: String,
    pub confirmation_id: String,
    pub initial_version_id: String,
    pub ordered_version_id: String,
    pub received_version_id: String,
    pub received_quantity: String,
    pub difference: String,
}

pub(super) fn records(
    connection: &Connection,
    material: Option<&str>,
    receipt: Option<&str>,
) -> Result<Vec<ReceiptRecord>, StoreError> {
    let rows=connection.prepare("SELECT r.id,r.confirmation_id,c.record_id,r.material_id,r.material_version_id,m.name,r.quantity_micros,r.unit,r.input_quantity,r.input_unit,r.amount_micros,r.receipt_date,r.note,r.recorded_at,r.sequence,r.source_kind,r.order_id,r.ordered_version_id FROM receipts r JOIN business_confirmations c ON c.id=r.confirmation_id JOIN material_versions m ON m.id=r.material_version_id AND m.material_id=r.material_id WHERE (?1 IS NULL OR r.material_id=?1) AND (?2 IS NULL OR r.id=?2) ORDER BY r.sequence,r.id").map_err(unavailable)?
        .query_map(params![material,receipt],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,String>(5)?,r.get::<_,i64>(6)?,r.get::<_,String>(7)?,r.get::<_,String>(8)?,r.get::<_,String>(9)?,r.get::<_,i64>(10)?,r.get::<_,String>(11)?,r.get::<_,String>(12)?,r.get::<_,String>(13)?,r.get::<_,i64>(14)?,r.get::<_,String>(15)?,r.get::<_,Option<String>>(16)?,r.get::<_,Option<String>>(17)?))).map_err(unavailable)?.collect::<rusqlite::Result<Vec<_>>>().map_err(unavailable)?;
    rows.into_iter()
        .map(
            |(
                id,
                confirmation_id,
                business_record_id,
                material_id,
                material_version_id,
                material_name,
                q,
                unit,
                input_quantity,
                input_unit,
                a,
                receipt_date,
                note,
                recorded_at,
                seq,
                kind,
                order_id,
                ordered_version_id,
            )| {
                let source = match (kind.as_str(), order_id, ordered_version_id) {
                    ("direct", None, None) => ReceiptSource::Direct {},
                    ("order", Some(order_id), Some(ordered_version_id)) => ReceiptSource::Order {
                        order_id,
                        ordered_version_id,
                    },
                    _ => return Err(unavailable("입고 원천 참조가 올바르지 않습니다")),
                };
                let row = ReceiptRecord {
                    id,
                    confirmation_id,
                    business_record_id,
                    material_id,
                    material_version_id,
                    material_name,
                    quantity: Quantity::from_micros(q).map_err(unavailable)?.decimal(),
                    unit,
                    input_quantity,
                    input_unit,
                    amount: Money::from_micros(a).map_err(unavailable)?.decimal(),
                    receipt_date,
                    note,
                    recorded_at,
                    seq: seq.to_string(),
                    source,
                };
                validate_record(&row)?;
                Ok(row)
            },
        )
        .collect()
}

pub(super) fn validate_record(row: &ReceiptRecord) -> Result<(), StoreError> {
    for id in [
        &row.id,
        &row.confirmation_id,
        &row.business_record_id,
        &row.material_id,
        &row.material_version_id,
    ] {
        Uuid::parse_str(id).map_err(unavailable)?;
    }
    if row
        .seq
        .parse::<i64>()
        .ok()
        .filter(|n| *n > 0)
        .map(|n| n.to_string())
        .as_deref()
        != Some(&row.seq)
    {
        return Err(unavailable("입고 순서가 올바르지 않습니다"));
    }
    chrono::DateTime::parse_from_rfc3339(&row.recorded_at).map_err(unavailable)?;
    let q = Quantity::parse(&row.quantity).map_err(unavailable)?;
    let unit = Unit::parse(&row.unit).map_err(unavailable)?;
    let input_unit = Unit::parse(&row.input_unit).map_err(unavailable)?;
    if q.micros() == 0
        || unit.canonical() != row.unit
        || !unit.same_dimension(&input_unit)
        || input_unit
            .to_canonical(Quantity::parse(&row.input_quantity).map_err(unavailable)?)
            .map_err(unavailable)?
            != q
    {
        return Err(unavailable("입고 수량과 기준 단위가 다릅니다"));
    }
    Money::parse(&row.amount).map_err(unavailable)?;
    // A historical local day is immutable; opening on a different timezone
    // must not apply today's calendar validation to already confirmed rows.
    parse_receipt_date(
        &row.receipt_date,
        NaiveDate::from_ymd_opt(9999, 12, 31).unwrap(),
    )
    .map_err(unavailable)?;
    Ok(())
}

pub(super) fn closures(connection: &Connection) -> Result<Vec<ClosureRow>, StoreError> {
    connection.prepare("SELECT order_id,receipt_id,confirmation_id,initial_version_id,ordered_version_id,received_version_id,received_quantity,difference FROM order_receipt_closures ORDER BY order_id").map_err(unavailable)?
        .query_map([],|r|Ok(ClosureRow{order_id:r.get(0)?,receipt_id:r.get(1)?,confirmation_id:r.get(2)?,initial_version_id:r.get(3)?,ordered_version_id:r.get(4)?,received_version_id:r.get(5)?,received_quantity:r.get(6)?,difference:r.get(7)?})).map_err(unavailable)?.collect::<rusqlite::Result<_>>().map_err(unavailable)
}

fn order_for_receipt(
    connection: &Connection,
    order: &crate::purchase_order::OrderVersion,
) -> Result<OrderForReceipt, StoreError> {
    let first = purchase_order::read_versions(connection, Some(&order.order_id), None, false)?
        .into_iter()
        .next()
        .ok_or_else(|| unavailable("최초 주문 버전이 없습니다"))?;
    OrderForReceipt::new(
        OrderReferences {
            order_id: Uuid::parse_str(&order.order_id).map_err(unavailable)?,
            material_id: Uuid::parse_str(&order.material_id).map_err(unavailable)?,
            initial_order_version_id: Uuid::parse_str(&first.id).map_err(unavailable)?,
            current_order_version_id: Uuid::parse_str(&order.id).map_err(unavailable)?,
        },
        order.status,
        Quantity::parse(&first.quantity).map_err(unavailable)?,
        Quantity::parse(&order.quantity).map_err(unavailable)?,
        Unit::parse(&order.unit).map_err(unavailable)?,
    )
    .map_err(unavailable)
}
fn actual_closure(
    connection: &Connection,
    receipt: &ReceiptRecord,
) -> Result<Option<OrderReceiptClosure>, StoreError> {
    let ReceiptSource::Order {
        order_id,
        ordered_version_id,
    } = &receipt.source
    else {
        return Ok(None);
    };
    let before =
        purchase_order::read_versions(connection, Some(order_id), Some(ordered_version_id), false)?
            .pop()
            .ok_or_else(|| unavailable("입고 전 주문 버전이 없습니다"))?;
    let order = order_for_receipt(connection, &before)?;
    let actual = ReceiptResult::new(
        ReceiptReferences {
            receipt_id: Uuid::parse_str(&receipt.id).map_err(unavailable)?,
            confirmation_id: Uuid::parse_str(&receipt.confirmation_id).map_err(unavailable)?,
            order_id: Uuid::parse_str(order_id).map_err(unavailable)?,
            expected_order_version_id: Uuid::parse_str(ordered_version_id).map_err(unavailable)?,
            material_id: Uuid::parse_str(&receipt.material_id).map_err(unavailable)?,
            material_version_id: Uuid::parse_str(&receipt.material_version_id)
                .map_err(unavailable)?,
        },
        Quantity::parse(&receipt.quantity).map_err(unavailable)?,
        Unit::parse(&receipt.unit).map_err(unavailable)?,
    )
    .map_err(unavailable)?;
    Ok(Some(close_order(&order, &actual).map_err(unavailable)?))
}

pub(super) fn material_name(
    connection: &Connection,
    data: &DraftReceiptV1,
) -> Result<Option<String>, StoreError> {
    let pair = match &data.source {
        ReceiptSourceDraft::Direct {
            material_id,
            material_version_id,
        } => material_id.clone().zip(material_version_id.clone()),
        ReceiptSourceDraft::Order {
            order_id,
            material_version_id,
            ..
        } => match order_id {
            Some(id) => purchase_order::current_version(connection, id)?
                .and_then(|o| material_version_id.clone().map(|v| (o.material_id, v))),
            None => None,
        },
    };
    let Some((material, version)) = pair else {
        return Ok(None);
    };
    connection
        .query_row(
            "SELECT name FROM material_versions WHERE material_id=?1 AND id=?2",
            params![material, version],
            |r| r.get(0),
        )
        .optional()
        .map_err(unavailable)
}

pub(super) fn inspect_receipt(
    connection: &Connection,
    data: &DraftReceiptV1,
    today: NaiveDate,
) -> Result<(Vec<BusinessIssue>, Option<BusinessEffect>), StoreError> {
    let mut issues = vec![];
    if matches!(
        data.stock_effect,
        ReceiptStockEffectDraft::NeedsReconciliation { .. }
    ) {
        issue(
            &mut issues,
            "stock_effect",
            "needs_reconciliation",
            "이미 포함되었거나 확인이 필요한 입고는 초안으로 보관합니다. 보정 연결 전에는 확정할 수 없습니다",
        );
    }
    let (material_id, version, order) = match &data.source {
        ReceiptSourceDraft::Direct {
            material_id,
            material_version_id,
        } => (material_id.clone(), material_version_id, None),
        ReceiptSourceDraft::Order {
            order_id,
            expected_order_version_id,
            material_version_id,
        } => {
            let order = match order_id {
                Some(id) => purchase_order::current_version(connection, id)?,
                None => None,
            };
            match &order {
                None => issue(
                    &mut issues,
                    "source.order_id",
                    "missing_reference",
                    "입고할 주문을 선택하세요",
                ),
                Some(o)
                    if o.status != OrderStatus::Open
                        || expected_order_version_id.as_deref() != Some(&o.id) =>
                {
                    issue(
                        &mut issues,
                        "source.expected_order_version_id",
                        "stale_reference",
                        "주문이 변경되거나 종료되었습니다. 최신 주문을 확인하세요",
                    )
                }
                _ => {}
            }
            (
                order.as_ref().map(|o| o.material_id.clone()),
                material_version_id,
                order,
            )
        }
    };
    let material = match material_id {
        Some(id) => catalog::current_version(connection, &id)?,
        None => None,
    };
    match &material {
        None => issue(
            &mut issues,
            "source.material_id",
            "missing_reference",
            "입고할 재료를 선택하세요",
        ),
        Some(m) if version.as_deref() != Some(&m.id) => issue(
            &mut issues,
            "source.material_version_id",
            "stale_reference",
            "최신 재료 버전을 선택하세요",
        ),
        _ => {}
    }
    let q = match Quantity::parse(&data.quantity) {
        Ok(q) if q.micros() > 0 => Some(q),
        Ok(_) => {
            issue(
                &mut issues,
                "quantity",
                "invalid_quantity",
                "실입고 수량은 0보다 커야 합니다",
            );
            None
        }
        Err(e) => {
            issue(&mut issues, "quantity", "invalid_quantity", e);
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
    let amount = match Money::parse(&data.amount) {
        Ok(a) => Some(a),
        Err(e) => {
            issue(&mut issues, "amount", "invalid_amount", e);
            None
        }
    };
    let date = match parse_receipt_date(&data.receipt_date, today) {
        Ok(d) => Some(d),
        Err(e) => {
            issue(&mut issues, "receipt_date", "invalid_date", e);
            None
        }
    };
    let mut canonical = None;
    if let Some(u) = &unit {
        if let Some(m) = &material
            && !u.same_dimension(&Unit::parse(&m.unit).map_err(unavailable)?)
        {
            issue(
                &mut issues,
                "unit",
                "incompatible_unit",
                "재료와 호환되는 단위를 사용하세요",
            );
        }
        if let Some(q) = q {
            match u.to_canonical(q) {
                Ok(q) => canonical = Some(q),
                Err(e) => issue(&mut issues, "quantity", "invalid_quantity", e),
            }
        }
    }
    if !issues.is_empty() {
        return Ok((issues, None));
    }
    let material = material.ok_or_else(|| unavailable("재료 검증 결과가 없습니다"))?;
    let quantity = canonical.ok_or_else(|| unavailable("수량 검증 결과가 없습니다"))?;
    let amount = amount.ok_or_else(|| unavailable("금액 검증 결과가 없습니다"))?;
    let holding = holding::current_input(connection, &material.material_id)?;
    let projection = match project_receipt(&holding, quantity, amount) {
        Ok(v) => v,
        Err(e) => {
            issue(&mut issues, "quantity", "invalid_holding", e);
            return Ok((issues, None));
        }
    };
    let order_after = order
        .as_ref()
        .map(|o| {
            let typed = order_for_receipt(connection, o)?;
            Ok::<_, StoreError>(OrderReceiptPreview {
                status: OrderStatus::Received,
                order_id: o.order_id.clone(),
                initial_order_version_id: typed.references().initial_order_version_id.to_string(),
                initial_ordered_quantity: typed.initial_quantity().decimal(),
                ordered_version_id: o.id.clone(),
                ordered_quantity: o.quantity.clone(),
                received_quantity: quantity.decimal(),
                difference: signed_quantity(quantity.micros() - typed.ordered_quantity().micros())?,
                unit: o.unit.clone(),
            })
        })
        .transpose()?;
    Ok((
        issues,
        Some(BusinessEffect::Receipt {
            material_id: material.material_id,
            material_version_id: material.id,
            quantity: quantity.decimal(),
            amount: amount.decimal(),
            unit: unit.as_ref().unwrap().canonical().into(),
            input_quantity: data.quantity.trim().into(),
            input_unit: data.unit.trim().into(),
            receipt_date: date.unwrap().to_string(),
            before: projection.before,
            after: projection.after,
            basis_before: projection.basis_before,
            observation: projection.observation,
            order_before: order.map(Box::new),
            order_after,
        }),
    ))
}

pub(super) fn apply_receipt(
    connection: &Connection,
    data: &DraftReceiptV1,
    confirmation_id: &str,
    seq: i64,
    time: &str,
    today: NaiveDate,
) -> Result<(), StoreError> {
    let (issues, effect) = inspect_receipt(connection, data, today)?;
    if !issues.is_empty() {
        let message = issues
            .iter()
            .map(|i| i.message.as_str())
            .collect::<Vec<_>>()
            .join("; ");
        return Err(if issues.iter().any(|i| i.code == "stale_reference") {
            StoreError::StaleState(message)
        } else {
            validation(message)
        });
    }
    let Some(BusinessEffect::Receipt {
        material_id,
        material_version_id,
        quantity,
        amount,
        unit,
        input_quantity,
        input_unit,
        receipt_date,
        order_before,
        ..
    }) = effect
    else {
        return Err(unavailable("입고 검증 결과가 없습니다"));
    };
    let id = Uuid::new_v4().to_string();
    let q = Quantity::parse(&quantity).map_err(unavailable)?;
    let a = Money::parse(&amount).map_err(unavailable)?;
    connection.execute("INSERT INTO receipts(id,confirmation_id,material_id,material_version_id,quantity_micros,unit,input_quantity,input_unit,amount_micros,receipt_date,note,recorded_at,sequence,source_kind,order_id,ordered_version_id) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",params![id,confirmation_id,material_id,material_version_id,q.micros(),unit,input_quantity,input_unit,a.micros(),receipt_date,data.note,time,seq,if order_before.is_some(){"order"}else{"direct"},order_before.as_ref().map(|o|o.order_id.as_str()),order_before.as_ref().map(|o|o.id.as_str())]).map_err(unavailable)?;
    holding::apply_receipt(
        connection,
        holding::ReceiptHoldingInput {
            receipt_id: &id,
            material_id: &material_id,
            material_version_id: &material_version_id,
            quantity: q,
            amount: a,
            seq,
            time,
        },
    )?;
    if let Some(before) = order_before {
        let row = records(connection, None, Some(&id))?
            .pop()
            .ok_or_else(|| unavailable("저장 입고가 없습니다"))?;
        let closure = actual_closure(connection, &row)?
            .ok_or_else(|| unavailable("주문 종료 결과가 없습니다"))?;
        let next = Uuid::new_v4().to_string();
        connection.execute("INSERT INTO purchase_order_versions(id,order_id,previous_version_id,confirmation_id,action,status,material_id,material_version_id,quantity_micros,unit,input_quantity,input_unit,supplier,expected_date,estimated_amount_micros,note,recorded_at,sequence) SELECT ?1,order_id,id,?2,'receive','received',material_id,material_version_id,quantity_micros,unit,input_quantity,input_unit,supplier,expected_date,estimated_amount_micros,note,?3,?4 FROM purchase_order_versions WHERE id=?5",params![next,confirmation_id,time,seq,before.id]).map_err(unavailable)?;
        connection
            .execute(
                "UPDATE purchase_order_heads SET current_version_id=?1 WHERE order_id=?2",
                params![next, before.order_id],
            )
            .map_err(unavailable)?;
        connection.execute("INSERT INTO order_receipt_closures(order_id,receipt_id,confirmation_id,initial_version_id,ordered_version_id,received_version_id,received_quantity,difference) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![before.order_id,id,confirmation_id,closure.initial_order_version_id.to_string(),closure.ordered_version_id.to_string(),next,closure.received_quantity,closure.difference]).map_err(unavailable)?;
    }
    Ok(())
}

pub(super) fn result(
    connection: &Connection,
    confirmation_id: &str,
) -> Result<BusinessResult, StoreError> {
    connection.query_row("SELECT r.id,h.id,r.material_id,r.material_version_id,r.order_id,c.received_version_id FROM receipts r JOIN holding_versions h ON h.receipt_id=r.id LEFT JOIN order_receipt_closures c ON c.receipt_id=r.id WHERE r.confirmation_id=?1",[confirmation_id],|r|Ok(BusinessResult::Receipt{receipt_id:r.get(0)?,holding_version_id:r.get(1)?,material_id:r.get(2)?,material_version_id:r.get(3)?,order_id:r.get(4)?,order_version_id:r.get(5)?})).map_err(unavailable)
}

impl Store {
    pub fn receipt_list(&self, filter: ReceiptFilter) -> Result<ReceiptList, StoreError> {
        let id = filter
            .material_id
            .map(|id| catalog::uuid(&id, "재료 ID"))
            .transpose()?;
        Ok(ReceiptList {
            items: records(&self.connection, id.as_deref(), None)?,
            state: catalog::state(&self.connection)?,
        })
    }
    pub fn receipt_detail(&self, id: &str) -> Result<Option<ReceiptDetail>, StoreError> {
        let id = catalog::uuid(id, "입고 ID")?;
        let Some(item) = records(&self.connection, None, Some(&id))?.pop() else {
            return Ok(None);
        };
        let holding_version_id = self
            .connection
            .query_row(
                "SELECT id FROM holding_versions WHERE receipt_id=?1",
                [&id],
                |r| r.get(0),
            )
            .map_err(unavailable)?;
        Ok(Some(ReceiptDetail {
            order_closure: actual_closure(&self.connection, &item)?,
            item,
            holding_version_id,
            state: catalog::state(&self.connection)?,
        }))
    }
}

pub(super) fn migrate_receipts(connection: &Connection) -> anyhow::Result<()> {
    let rows = connection
        .prepare("SELECT sequence,payload FROM snapshot_outbox ORDER BY sequence")?
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (seq, payload) in rows {
        snapshot::validate_source(&payload, seq, 5)?;
    }
    connection.execute_batch("PRAGMA defer_foreign_keys=ON;
        CREATE TABLE purchase_order_versions_next(id TEXT PRIMARY KEY NOT NULL,order_id TEXT NOT NULL REFERENCES purchase_orders(id),previous_version_id TEXT,confirmation_id TEXT NOT NULL UNIQUE REFERENCES business_confirmations(id),action TEXT NOT NULL,status TEXT NOT NULL,material_id TEXT NOT NULL,material_version_id TEXT NOT NULL,quantity_micros INTEGER NOT NULL CHECK(quantity_micros>0 AND quantity_micros<=9000000000000000),unit TEXT NOT NULL,input_quantity TEXT NOT NULL,input_unit TEXT NOT NULL,supplier TEXT,expected_date TEXT,estimated_amount_micros INTEGER CHECK(estimated_amount_micros>=0 AND estimated_amount_micros<=9000000000000000000),note TEXT NOT NULL,recorded_at TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),UNIQUE(order_id,id),FOREIGN KEY(order_id,previous_version_id) REFERENCES purchase_order_versions_next(order_id,id),FOREIGN KEY(material_id,material_version_id) REFERENCES material_versions(material_id,id),CHECK((action IN('place','revise') AND status='open') OR (action='cancel' AND status='cancelled') OR(action='receive' AND status='received')));
        INSERT INTO purchase_order_versions_next SELECT * FROM purchase_order_versions;
        CREATE TABLE purchase_order_heads_next(order_id TEXT PRIMARY KEY NOT NULL REFERENCES purchase_orders(id),current_version_id TEXT NOT NULL,FOREIGN KEY(order_id,current_version_id) REFERENCES purchase_order_versions_next(order_id,id));
        INSERT INTO purchase_order_heads_next SELECT * FROM purchase_order_heads;
        DROP TABLE purchase_order_heads;
        DROP TABLE purchase_order_versions;
        ALTER TABLE purchase_order_versions_next RENAME TO purchase_order_versions;
        ALTER TABLE purchase_order_heads_next RENAME TO purchase_order_heads;
        CREATE INDEX purchase_order_versions_order ON purchase_order_versions(order_id,sequence);
        CREATE TABLE receipts(id TEXT PRIMARY KEY NOT NULL,confirmation_id TEXT NOT NULL UNIQUE REFERENCES business_confirmations(id),material_id TEXT NOT NULL,material_version_id TEXT NOT NULL,quantity_micros INTEGER NOT NULL CHECK(quantity_micros>0 AND quantity_micros<=9000000000000000),unit TEXT NOT NULL,input_quantity TEXT NOT NULL,input_unit TEXT NOT NULL,amount_micros INTEGER NOT NULL CHECK(amount_micros>=0 AND amount_micros<=9000000000000000000),receipt_date TEXT NOT NULL,note TEXT NOT NULL,recorded_at TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),source_kind TEXT NOT NULL,order_id TEXT REFERENCES purchase_orders(id),ordered_version_id TEXT,FOREIGN KEY(material_id,material_version_id) REFERENCES material_versions(material_id,id),FOREIGN KEY(order_id,ordered_version_id) REFERENCES purchase_order_versions(order_id,id),CHECK((source_kind='direct' AND order_id IS NULL AND ordered_version_id IS NULL) OR (source_kind='order' AND order_id IS NOT NULL AND ordered_version_id IS NOT NULL)));
        CREATE TABLE holding_versions(id TEXT PRIMARY KEY NOT NULL,material_id TEXT NOT NULL,material_version_id TEXT NOT NULL,previous_version_id TEXT,receipt_id TEXT NOT NULL UNIQUE REFERENCES receipts(id),baseline_receipt_id TEXT NOT NULL REFERENCES receipts(id),baseline_sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),quantity_micros INTEGER NOT NULL CHECK(quantity_micros>0 AND quantity_micros<=9000000000000000),value_micros INTEGER NOT NULL CHECK(value_micros>=0 AND value_micros<=9000000000000000000),recorded_at TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),UNIQUE(material_id,id),FOREIGN KEY(material_id,material_version_id) REFERENCES material_versions(material_id,id),FOREIGN KEY(material_id,previous_version_id) REFERENCES holding_versions(material_id,id));
        CREATE TABLE holding_heads(material_id TEXT PRIMARY KEY NOT NULL REFERENCES materials(id),current_version_id TEXT NOT NULL,last_valid_version_id TEXT NOT NULL,FOREIGN KEY(material_id,current_version_id) REFERENCES holding_versions(material_id,id),FOREIGN KEY(material_id,last_valid_version_id) REFERENCES holding_versions(material_id,id));
        CREATE TABLE order_receipt_closures(order_id TEXT PRIMARY KEY NOT NULL REFERENCES purchase_orders(id),receipt_id TEXT NOT NULL UNIQUE REFERENCES receipts(id),confirmation_id TEXT NOT NULL UNIQUE REFERENCES business_confirmations(id),initial_version_id TEXT NOT NULL,ordered_version_id TEXT NOT NULL,received_version_id TEXT NOT NULL UNIQUE,received_quantity TEXT NOT NULL,difference TEXT NOT NULL,FOREIGN KEY(order_id,initial_version_id) REFERENCES purchase_order_versions(order_id,id),FOREIGN KEY(order_id,ordered_version_id) REFERENCES purchase_order_versions(order_id,id),FOREIGN KEY(order_id,received_version_id) REFERENCES purchase_order_versions(order_id,id));
        CREATE INDEX receipts_material ON receipts(material_id,sequence);
        CREATE INDEX holding_versions_material ON holding_versions(material_id,sequence);
        PRAGMA user_version=6;")?;
    Ok(())
}

pub(super) fn validate_receipt_schema(connection: &Connection) -> anyhow::Result<()> {
    let records = records(connection, None, None)?;
    let closures = closures(connection)?;
    let orders = purchase_order::read_versions(connection, None, None, false)?;
    let linked = records
        .iter()
        .filter(|r| matches!(r.source, ReceiptSource::Order { .. }))
        .count();
    if linked != closures.len()
        || orders
            .iter()
            .filter(|o| o.status == OrderStatus::Received)
            .count()
            != closures.len()
    {
        anyhow::bail!("입고와 주문 종료의 연결 수가 다릅니다")
    }
    for receipt in records {
        let (kind,seq,time):(String,i64,String)=connection.query_row("SELECT b.kind,c.sequence,c.recorded_at FROM business_confirmations c JOIN business_records b ON b.id=c.record_id WHERE c.id=?1",[&receipt.confirmation_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
        if kind != "receipt" || seq.to_string() != receipt.seq || time != receipt.recorded_at {
            anyhow::bail!("입고와 업무 확정이 다릅니다")
        }
        if let Some(actual) = actual_closure(connection, &receipt)? {
            let row = closures
                .iter()
                .find(|c| c.receipt_id == receipt.id)
                .ok_or_else(|| anyhow::anyhow!("실제 주문 종료 연결이 없습니다"))?;
            let before = orders
                .iter()
                .find(|o| o.id == row.ordered_version_id)
                .ok_or_else(|| anyhow::anyhow!("주문 이전 버전이 없습니다"))?;
            let after = orders
                .iter()
                .find(|o| o.id == row.received_version_id)
                .ok_or_else(|| anyhow::anyhow!("주문 종료 버전이 없습니다"))?;
            if row.order_id != actual.order_id.to_string()
                || row.confirmation_id != receipt.confirmation_id
                || row.initial_version_id != actual.initial_order_version_id.to_string()
                || row.ordered_version_id != actual.ordered_version_id.to_string()
                || row.received_quantity != actual.received_quantity
                || row.difference != actual.difference
                || after.status != OrderStatus::Received
                || after.confirmation_id != receipt.confirmation_id
                || after.previous_version_id.as_deref() != Some(&before.id)
                || after.material_id != receipt.material_id
                || after.seq != receipt.seq
                || after.quantity != before.quantity
                || after.material_version_id != before.material_version_id
                || after.unit != before.unit
                || after.input_quantity != before.input_quantity
                || after.input_unit != before.input_unit
                || after.supplier != before.supplier
                || after.expected_date != before.expected_date
                || after.estimated_amount != before.estimated_amount
                || after.note != before.note
            {
                anyhow::bail!("실제 입고와 주문 종료 이력이 다릅니다")
            }
        }
    }
    Ok(())
}

pub(super) fn validate_snapshot(
    receipts: &[ReceiptRecord],
    closures: &[ClosureRow],
    orders: &[crate::purchase_order::OrderVersion],
    holdings: &[holding::LegacyHoldingVersion],
    heads: &[holding::HoldingHead],
) -> Result<(), StoreError> {
    validate_receipt_links(receipts, closures, orders)?;
    let bad = || unavailable("JSON 입고·보유·주문 종료 참조가 일치하지 않습니다");
    if holdings.len() != receipts.len() {
        return Err(bad());
    }
    validate_legacy_holdings(receipts, holdings, heads)
}

pub(super) fn validate_receipt_links(
    receipts: &[ReceiptRecord],
    closures: &[ClosureRow],
    orders: &[crate::purchase_order::OrderVersion],
) -> Result<(), StoreError> {
    let bad = || unavailable("JSON 입고·주문 종료 참조가 일치하지 않습니다");
    let mut ids = std::collections::HashSet::new();
    for receipt in receipts {
        validate_record(receipt)?;
        if !ids.insert(&receipt.id) {
            return Err(bad());
        }
    }
    if closures.len()
        != receipts
            .iter()
            .filter(|r| matches!(r.source, ReceiptSource::Order { .. }))
            .count()
        || closures.len()
            != orders
                .iter()
                .filter(|o| o.status == OrderStatus::Received)
                .count()
    {
        return Err(bad());
    }
    let mut seen_orders = std::collections::HashSet::new();
    let mut seen_receipts = std::collections::HashSet::new();
    for row in closures {
        if !seen_orders.insert(&row.order_id) || !seen_receipts.insert(&row.receipt_id) {
            return Err(bad());
        }
        let receipt = receipts
            .iter()
            .find(|r| r.id == row.receipt_id)
            .ok_or_else(bad)?;
        let ReceiptSource::Order {
            order_id,
            ordered_version_id,
        } = &receipt.source
        else {
            return Err(bad());
        };
        let first = orders
            .iter()
            .find(|o| o.id == row.initial_version_id)
            .ok_or_else(bad)?;
        let before = orders
            .iter()
            .find(|o| o.id == row.ordered_version_id)
            .ok_or_else(bad)?;
        let after = orders
            .iter()
            .find(|o| o.id == row.received_version_id)
            .ok_or_else(bad)?;
        if order_id != &row.order_id
            || ordered_version_id != &row.ordered_version_id
            || row.confirmation_id != receipt.confirmation_id
            || first.order_id != *order_id
            || first.previous_version_id.is_some()
            || before.order_id != *order_id
            || after.order_id != *order_id
            || before.status != OrderStatus::Open
            || after.status != OrderStatus::Received
            || after.previous_version_id.as_deref() != Some(&before.id)
            || after.confirmation_id != receipt.confirmation_id
            || after.business_record_id != receipt.business_record_id
            || after.seq != receipt.seq
            || after.recorded_at != receipt.recorded_at
            || before.material_id != receipt.material_id
            || after.material_id != before.material_id
            || after.material_version_id != before.material_version_id
            || after.quantity != before.quantity
            || after.unit != before.unit
            || after.input_quantity != before.input_quantity
            || after.input_unit != before.input_unit
            || after.supplier != before.supplier
            || after.expected_date != before.expected_date
            || after.estimated_amount != before.estimated_amount
            || after.note != before.note
            || row.received_quantity != receipt.quantity
            || row.difference
                != signed_quantity(
                    Quantity::parse(&receipt.quantity)
                        .map_err(unavailable)?
                        .micros()
                        - Quantity::parse(&before.quantity)
                            .map_err(unavailable)?
                            .micros(),
                )?
        {
            return Err(bad());
        }
    }
    Ok(())
}

fn validate_legacy_holdings(
    receipts: &[ReceiptRecord],
    holdings: &[holding::LegacyHoldingVersion],
    heads: &[holding::HoldingHead],
) -> Result<(), StoreError> {
    let bad = || unavailable("JSON 이전 보유·입고 참조가 일치하지 않습니다");
    let mut grouped =
        std::collections::BTreeMap::<&str, Vec<&holding::LegacyHoldingVersion>>::new();
    let mut versions = std::collections::HashSet::new();
    let mut used_receipts = std::collections::HashSet::new();
    for version in holdings {
        Uuid::parse_str(&version.id).map_err(unavailable)?;
        if !versions.insert(&version.id) || !used_receipts.insert(&version.receipt_id) {
            return Err(bad());
        }
        grouped
            .entry(&version.material_id)
            .or_default()
            .push(version);
    }
    if grouped.len() != heads.len() {
        return Err(bad());
    }
    for (material, mut history) in grouped {
        history.sort_by_key(|v| v.seq.parse::<i64>().unwrap_or(-1));
        let first = history[0];
        let mut previous = None;
        let mut quantity = 0i64;
        let mut value = 0i64;
        for version in &history {
            let receipt = receipts
                .iter()
                .find(|r| r.id == version.receipt_id)
                .ok_or_else(bad)?;
            quantity = quantity
                .checked_add(
                    Quantity::parse(&receipt.quantity)
                        .map_err(unavailable)?
                        .micros(),
                )
                .ok_or_else(bad)?;
            value = value
                .checked_add(Money::parse(&receipt.amount).map_err(unavailable)?.micros())
                .ok_or_else(bad)?;
            if version.material_id != receipt.material_id
                || version.material_version_id != receipt.material_version_id
                || version.previous_version_id.as_deref() != previous
                || version.baseline_receipt_id != first.receipt_id
                || version.baseline_seq != first.seq
                || version.seq != receipt.seq
                || version.recorded_at != receipt.recorded_at
                || Quantity::parse(&version.quantity)
                    .map_err(unavailable)?
                    .micros()
                    != quantity
                || Money::parse(&version.value).map_err(unavailable)?.micros() != value
            {
                return Err(bad());
            }
            previous = Some(version.id.as_str());
        }
        let head = heads
            .iter()
            .find(|h| h.material_id == material)
            .ok_or_else(bad)?;
        if Some(head.current_version_id.as_str()) != previous
            || head.last_valid_version_id != head.current_version_id
        {
            return Err(bad());
        }
    }
    Ok(())
}
