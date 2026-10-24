use super::{Store, StoreError, catalog, snapshot, unavailable};
use crate::{
    business::{BusinessEffect, BusinessIssue, BusinessResult},
    domain::{Money, Quantity, Unit},
    purchase_order::*,
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

fn validation(message: impl std::fmt::Display) -> StoreError {
    StoreError::Validation(message.to_string())
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
pub(super) fn action_text(action: OrderAction) -> &'static str {
    match action {
        OrderAction::Place => "place",
        OrderAction::Revise => "revise",
        OrderAction::Cancel => "cancel",
        OrderAction::Receive => "receive",
        OrderAction::Correct => "correct",
    }
}
fn status_text(status: OrderStatus) -> &'static str {
    match status {
        OrderStatus::Open => "open",
        OrderStatus::Received => "received",
        OrderStatus::Cancelled => "cancelled",
    }
}
fn parse_action(value: &str) -> Result<OrderAction, StoreError> {
    match value {
        "place" => Ok(OrderAction::Place),
        "revise" => Ok(OrderAction::Revise),
        "cancel" => Ok(OrderAction::Cancel),
        "receive" => Ok(OrderAction::Receive),
        "correct" => Ok(OrderAction::Correct),
        _ => Err(unavailable("지원하지 않는 주문 동작입니다")),
    }
}
fn parse_status(value: &str) -> Result<OrderStatus, StoreError> {
    match value {
        "open" => Ok(OrderStatus::Open),
        "cancelled" => Ok(OrderStatus::Cancelled),
        "received" => Ok(OrderStatus::Received),
        _ => Err(unavailable(
            "입고 연결이 없거나 지원하지 않는 주문 상태입니다",
        )),
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OrderRecord {
    pub id: String,
    pub first_confirmation_id: String,
    pub created_at: String,
    pub seq: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OrderHead {
    pub order_id: String,
    pub current_version_id: String,
}
pub(super) struct OrderSnapshotRows {
    pub orders: Vec<OrderRecord>,
    pub versions: Vec<OrderVersion>,
    pub heads: Vec<OrderHead>,
}

const VERSION_COLUMNS: &str = "v.id,v.order_id,v.previous_version_id,v.confirmation_id,c.record_id,v.action,v.status,v.material_id, v.material_version_id,m.name,v.quantity_micros,v.unit,v.input_quantity,v.input_unit,v.supplier,v.expected_date,v.estimated_amount_micros,v.note,v.recorded_at,v.sequence,v.correction_projection_id";
const VERSION_JOINS: &str = "FROM purchase_order_versions v JOIN business_confirmations c ON c.id=v.confirmation_id JOIN material_versions m ON m.id=v.material_version_id AND m.material_id=v.material_id";

pub(super) fn read_versions(
    connection: &Connection,
    order_id: Option<&str>,
    version_id: Option<&str>,
    heads_only: bool,
) -> Result<Vec<OrderVersion>, StoreError> {
    let sql = format!(
        "SELECT {VERSION_COLUMNS} {VERSION_JOINS} WHERE (?1 IS NULL OR v.order_id=?1) AND (?2 IS NULL OR v.id=?2) AND (?3=0 OR EXISTS(SELECT 1 FROM purchase_order_heads h WHERE h.order_id=v.order_id AND h.current_version_id=v.id)) ORDER BY v.sequence,v.id"
    );
    let rows = connection
        .prepare(&sql)
        .map_err(unavailable)?
        .query_map(params![order_id, version_id, heads_only], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, String>(6)?,
                r.get::<_, String>(7)?,
                r.get::<_, String>(8)?,
                r.get::<_, String>(9)?,
                r.get::<_, i64>(10)?,
                r.get::<_, String>(11)?,
                r.get::<_, String>(12)?,
                r.get::<_, String>(13)?,
                r.get::<_, Option<String>>(14)?,
                r.get::<_, Option<String>>(15)?,
                r.get::<_, Option<i64>>(16)?,
                r.get::<_, String>(17)?,
                r.get::<_, String>(18)?,
                r.get::<_, i64>(19)?,
                r.get::<_, Option<String>>(20)?,
            ))
        })
        .map_err(unavailable)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(unavailable)?;
    rows.into_iter()
        .map(
            |(
                id,
                order_id,
                previous_version_id,
                confirmation_id,
                business_record_id,
                action,
                status,
                material_id,
                material_version_id,
                material_name,
                quantity,
                unit,
                input_quantity,
                input_unit,
                supplier,
                expected_date,
                estimated_amount,
                note,
                recorded_at,
                seq,
                correction_projection_id,
            )| {
                let version = OrderVersion {
                    correction_projection_id,
                    id,
                    order_id,
                    previous_version_id,
                    confirmation_id,
                    business_record_id,
                    action: parse_action(&action)?,
                    status: parse_status(&status)?,
                    material_id,
                    material_version_id,
                    material_name,
                    quantity: Quantity::from_micros(quantity)
                        .map_err(unavailable)?
                        .decimal(),
                    unit,
                    input_quantity,
                    input_unit,
                    supplier,
                    expected_date,
                    estimated_amount: estimated_amount
                        .map(|a| Money::from_micros(a).map(|a| a.decimal()))
                        .transpose()
                        .map_err(unavailable)?,
                    note,
                    recorded_at,
                    seq: seq.to_string(),
                };
                validate_stored_version(&version)?;
                Ok(version)
            },
        )
        .collect()
}
pub(super) fn current_version(
    connection: &Connection,
    id: &str,
) -> Result<Option<OrderVersion>, StoreError> {
    let exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM purchase_orders WHERE id=?1)",
            [id],
            |r| r.get(0),
        )
        .map_err(unavailable)?;
    let result = read_versions(connection, Some(id), None, true)?.pop();
    if exists && result.is_none() {
        return Err(unavailable("주문의 현재 버전 연결이 없습니다"));
    }
    Ok(result)
}
pub(super) fn validate_stored_version(version: &OrderVersion) -> Result<(), StoreError> {
    if !matches!(
        (version.action, version.status),
        (OrderAction::Place, OrderStatus::Open)
            | (OrderAction::Revise, OrderStatus::Open)
            | (OrderAction::Cancel, OrderStatus::Cancelled)
            | (OrderAction::Receive, OrderStatus::Received)
            | (
                OrderAction::Correct,
                OrderStatus::Open | OrderStatus::Received
            )
    ) {
        return Err(unavailable(
            "주문 동작과 상태가 일치하지 않거나 입고 연결이 없습니다",
        ));
    }
    if (version.action == OrderAction::Correct) != version.correction_projection_id.is_some() {
        return Err(unavailable(
            "주문 정정 동작과 실제 결과 참조가 일치하지 않습니다",
        ));
    }
    let q = Quantity::parse(&version.quantity).map_err(unavailable)?;
    let input = Quantity::parse(&version.input_quantity).map_err(unavailable)?;
    let unit = Unit::parse(&version.unit).map_err(unavailable)?;
    let input_unit = Unit::parse(&version.input_unit).map_err(unavailable)?;
    if q.micros() == 0
        || unit.canonical() != version.unit
        || !unit.same_dimension(&input_unit)
        || input_unit.to_canonical(input).map_err(unavailable)? != q
    {
        return Err(unavailable("주문 수량과 기준 단위가 일치하지 않습니다"));
    }
    if let Some(amount) = &version.estimated_amount {
        Money::parse(amount).map_err(unavailable)?;
    }
    if let Some(date) = &version.expected_date {
        parse_expected_date(date).map_err(unavailable)?;
    }
    Ok(())
}
fn summary(connection: &Connection, version: OrderVersion) -> Result<OrderSummary, StoreError> {
    let created_at = connection
        .query_row(
            "SELECT created_at FROM purchase_orders WHERE id=?1",
            [&version.order_id],
            |r| r.get(0),
        )
        .map_err(unavailable)?;
    Ok(OrderSummary {
        id: version.order_id,
        head_id: version.id,
        status: version.status,
        material_id: version.material_id,
        material_version_id: version.material_version_id,
        material_name: version.material_name,
        quantity: version.quantity,
        unit: version.unit,
        input_quantity: version.input_quantity,
        input_unit: version.input_unit,
        supplier: version.supplier,
        expected_date: version.expected_date,
        estimated_amount: version.estimated_amount,
        created_at,
        updated_at: version.recorded_at,
    })
}
pub(super) fn result(
    connection: &Connection,
    confirmation_id: &str,
) -> Result<BusinessResult, StoreError> {
    let (order, version): (String, String) = connection
        .query_row(
            "SELECT order_id,id FROM purchase_order_versions WHERE confirmation_id=?1",
            [confirmation_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(unavailable)?;
    let version = read_versions(connection, Some(&order), Some(&version), false)?
        .pop()
        .ok_or_else(|| unavailable("주문 확정 결과가 없습니다"))?;
    Ok(BusinessResult::PurchaseOrder {
        action: version.action,
        order_id: version.order_id,
        order_version_id: version.id,
        material_id: version.material_id,
        material_version_id: version.material_version_id,
    })
}

pub(super) fn same_action_target(old: &DraftOrderActionV1, new: &DraftOrderActionV1) -> bool {
    match (old, new) {
        (DraftOrderActionV1::Place { .. }, DraftOrderActionV1::Place { .. }) => true,
        (
            DraftOrderActionV1::Revise { order_id: a, .. },
            DraftOrderActionV1::Revise { order_id: b, .. },
        )
        | (
            DraftOrderActionV1::Cancel { order_id: a, .. },
            DraftOrderActionV1::Cancel { order_id: b, .. },
        ) => a == b,
        _ => false,
    }
}
pub(super) fn material_name(
    connection: &Connection,
    data: &DraftOrderActionV1,
) -> Result<Option<String>, StoreError> {
    let ids = match data {
        DraftOrderActionV1::Place {
            material_id,
            material_version_id,
            ..
        } => material_id.clone().zip(material_version_id.clone()),
        DraftOrderActionV1::Revise {
            order_id,
            material_version_id,
            ..
        } => current_version(connection, order_id)?
            .and_then(|v| material_version_id.clone().map(|id| (v.material_id, id))),
        DraftOrderActionV1::Cancel {
            order_id,
            expected_order_version_id,
            ..
        } => read_versions(
            connection,
            Some(order_id),
            Some(expected_order_version_id),
            false,
        )?
        .pop()
        .map(|v| (v.material_id, v.material_version_id)),
    };
    let Some((material, version)) = ids else {
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
fn optional_text(text: &str) -> Option<String> {
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

pub(super) fn inspect_order(
    connection: &Connection,
    data: &DraftOrderActionV1,
) -> Result<(Vec<BusinessIssue>, Option<BusinessEffect>), StoreError> {
    let mut issues = vec![];
    let (before, order_id) = match data {
        DraftOrderActionV1::Place { .. } => (None, None),
        DraftOrderActionV1::Revise {
            order_id,
            expected_order_version_id,
            ..
        }
        | DraftOrderActionV1::Cancel {
            order_id,
            expected_order_version_id,
            ..
        } => {
            let before = current_version(connection, order_id)?;
            match &before {
                None => issue(
                    &mut issues,
                    "order_id",
                    "missing_reference",
                    "주문을 찾을 수 없습니다",
                ),
                Some(v) if v.id != *expected_order_version_id || v.status != OrderStatus::Open => {
                    issue(
                        &mut issues,
                        "expected_order_version_id",
                        "stale_reference",
                        "주문이 변경되거나 종료되었습니다. 최신 상태를 확인하세요",
                    )
                }
                _ => {}
            }
            (before, Some(order_id.clone()))
        }
    };
    if let DraftOrderActionV1::Cancel { note, .. } = data {
        if !issues.is_empty() {
            return Ok((issues, None));
        }
        let old = before
            .as_ref()
            .ok_or_else(|| unavailable("주문 조회 결과가 없습니다"))?;
        let after = OrderProposal {
            status: OrderStatus::Cancelled,
            material_id: old.material_id.clone(),
            material_version_id: old.material_version_id.clone(),
            quantity: old.quantity.clone(),
            unit: old.unit.clone(),
            input_quantity: old.input_quantity.clone(),
            input_unit: old.input_unit.clone(),
            supplier: old.supplier.clone(),
            expected_date: old.expected_date.clone(),
            estimated_amount: old.estimated_amount.clone(),
            note: note.clone(),
        };
        return Ok((
            issues,
            Some(BusinessEffect::PurchaseOrder {
                action: data.action(),
                order_id,
                before,
                after,
            }),
        ));
    }
    let (material_id, version_id, quantity, unit, supplier, date, amount, note) = match data {
        DraftOrderActionV1::Place {
            material_id,
            material_version_id,
            quantity,
            unit,
            supplier,
            expected_date,
            estimated_amount,
            note,
        } => (
            material_id.clone(),
            material_version_id,
            quantity,
            unit,
            supplier,
            expected_date,
            estimated_amount,
            note,
        ),
        DraftOrderActionV1::Revise {
            material_version_id,
            quantity,
            unit,
            supplier,
            expected_date,
            estimated_amount,
            note,
            ..
        } => (
            before.as_ref().map(|v| v.material_id.clone()),
            material_version_id,
            quantity,
            unit,
            supplier,
            expected_date,
            estimated_amount,
            note,
        ),
        DraftOrderActionV1::Cancel { .. } => unreachable!("cancel handled above"),
    };
    let material = match &material_id {
        Some(id) => catalog::current_version(connection, id)?,
        None => None,
    };
    if material.is_none() {
        issue(
            &mut issues,
            "material_id",
            "missing_reference",
            "재료를 선택하세요",
        )
    }
    if let Some(material) = &material
        && version_id.as_deref() != Some(material.id.as_str())
    {
        issue(
            &mut issues,
            "material_version_id",
            "stale_reference",
            "최신 재료 버전을 선택하세요",
        )
    }
    let q = match Quantity::parse(quantity) {
        Ok(q) if q.micros() > 0 => Some(q),
        Ok(_) => {
            issue(
                &mut issues,
                "quantity",
                "invalid_quantity",
                "주문 수량은 0보다 커야 합니다",
            );
            None
        }
        Err(e) => {
            issue(&mut issues, "quantity", "invalid_quantity", e);
            None
        }
    };
    let parsed_unit = match Unit::parse(unit) {
        Ok(u) => Some(u),
        Err(e) => {
            issue(&mut issues, "unit", "invalid_unit", e);
            None
        }
    };
    let mut canonical = None;
    if let Some(u) = &parsed_unit {
        if let Some(material) = &material
            && !u.same_dimension(&Unit::parse(&material.unit).map_err(unavailable)?)
        {
            issue(
                &mut issues,
                "unit",
                "incompatible_unit",
                "재료와 같은 차원 단위를 사용하세요",
            )
        }
        if let Some(q) = q {
            match u.to_canonical(q) {
                Ok(q) => canonical = Some(q),
                Err(e) => issue(&mut issues, "quantity", "invalid_quantity", e),
            }
        }
    }
    let estimated_amount = match optional_text(amount) {
        None => None,
        Some(a) => match Money::parse(&a) {
            Ok(a) => Some(a.decimal()),
            Err(e) => {
                issue(&mut issues, "estimated_amount", "invalid_amount", e);
                None
            }
        },
    };
    let expected_date = match parse_expected_date(date) {
        Ok(d) => d,
        Err(e) => {
            issue(&mut issues, "expected_date", "invalid_date", e);
            None
        }
    };
    if !issues.is_empty() {
        return Ok((issues, None));
    }
    let material = material.ok_or_else(|| unavailable("재료 검증 결과가 없습니다"))?;
    let after = OrderProposal {
        status: OrderStatus::Open,
        material_id: material.material_id,
        material_version_id: material.id,
        quantity: canonical
            .ok_or_else(|| unavailable("수량 검증 결과가 없습니다"))?
            .decimal(),
        unit: parsed_unit
            .ok_or_else(|| unavailable("단위 검증 결과가 없습니다"))?
            .canonical()
            .into(),
        input_quantity: q
            .ok_or_else(|| unavailable("입력 수량 검증 결과가 없습니다"))?
            .decimal(),
        input_unit: unit.trim().into(),
        supplier: optional_text(supplier),
        expected_date,
        estimated_amount,
        note: note.clone(),
    };
    Ok((
        issues,
        Some(BusinessEffect::PurchaseOrder {
            action: data.action(),
            order_id,
            before,
            after,
        }),
    ))
}
pub(super) fn apply_order(
    connection: &Connection,
    data: &DraftOrderActionV1,
    confirmation_id: &str,
    seq: i64,
    time: &str,
) -> Result<BusinessResult, StoreError> {
    let (issues, effect) = inspect_order(connection, data)?;
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
    let Some(BusinessEffect::PurchaseOrder {
        action,
        order_id,
        before,
        after,
    }) = effect
    else {
        return Err(unavailable("주문 검증 결과가 없습니다"));
    };
    let order_id = order_id.unwrap_or_else(|| Uuid::new_v4().to_string());
    let version_id = Uuid::new_v4().to_string();
    if action == OrderAction::Place {
        connection.execute("INSERT INTO purchase_orders(id,first_confirmation_id,created_at,sequence) VALUES(?1,?2,?3,?4)",params![order_id,confirmation_id,time,seq]).map_err(unavailable)?;
    }
    let quantity = Quantity::parse(&after.quantity)
        .map_err(unavailable)?
        .micros();
    let amount = after
        .estimated_amount
        .as_ref()
        .map(|a| Money::parse(a).map(|a| a.micros()))
        .transpose()
        .map_err(unavailable)?;
    connection.execute("INSERT INTO purchase_order_versions(id,order_id,previous_version_id,confirmation_id,action,status,material_id,material_version_id,quantity_micros,unit,input_quantity,input_unit,supplier,expected_date,estimated_amount_micros,note,recorded_at,sequence) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)",params![version_id,order_id,before.as_ref().map(|v|v.id.as_str()),confirmation_id,action_text(action),status_text(after.status),after.material_id,after.material_version_id,quantity,after.unit,after.input_quantity,after.input_unit,after.supplier,after.expected_date,amount,after.note,time,seq]).map_err(unavailable)?;
    connection.execute("INSERT INTO purchase_order_heads(order_id,current_version_id) VALUES(?1,?2) ON CONFLICT(order_id) DO UPDATE SET current_version_id=excluded.current_version_id",params![order_id,version_id]).map_err(unavailable)?;
    Ok(BusinessResult::PurchaseOrder {
        action,
        order_id,
        order_version_id: version_id,
        material_id: after.material_id,
        material_version_id: after.material_version_id,
    })
}

impl Store {
    pub fn order_list(&self, filter: OrderFilter) -> Result<OrderList, StoreError> {
        let heads = read_versions(&self.connection, None, None, true)?;
        let mut items = vec![];
        for head in heads {
            let matches = match filter.status {
                OrderStatusFilter::All => true,
                OrderStatusFilter::Open => head.status == OrderStatus::Open,
                OrderStatusFilter::Received => head.status == OrderStatus::Received,
                OrderStatusFilter::Cancelled => head.status == OrderStatus::Cancelled,
            };
            if matches {
                items.push(summary(&self.connection, head)?)
            }
        }
        items.sort_by(|a, b| (&a.created_at, &a.id).cmp(&(&b.created_at, &b.id)));
        Ok(OrderList {
            items,
            state: catalog::state(&self.connection)?,
        })
    }
    pub fn order_detail(&self, id: &str) -> Result<Option<OrderDetail>, StoreError> {
        let id = catalog::uuid(id, "주문 ID")?;
        let Some(head) = current_version(&self.connection, &id)? else {
            return Ok(None);
        };
        Ok(Some(OrderDetail {
            item: summary(&self.connection, head.clone())?,
            head,
            versions: read_versions(&self.connection, Some(&id), None, false)?,
            state: catalog::state(&self.connection)?,
        }))
    }
}
pub(super) fn order_snapshot_rows(
    connection: &Connection,
) -> Result<OrderSnapshotRows, StoreError> {
    let orders=connection.prepare("SELECT id,first_confirmation_id,created_at,sequence FROM purchase_orders ORDER BY sequence,id").map_err(unavailable)?.query_map([],|r|Ok(OrderRecord{id:r.get(0)?,first_confirmation_id:r.get(1)?,created_at:r.get(2)?,seq:r.get::<_,i64>(3)?.to_string()})).map_err(unavailable)?.collect::<rusqlite::Result<_>>().map_err(unavailable)?;
    let heads = connection
        .prepare("SELECT order_id,current_version_id FROM purchase_order_heads ORDER BY order_id")
        .map_err(unavailable)?
        .query_map([], |r| {
            Ok(OrderHead {
                order_id: r.get(0)?,
                current_version_id: r.get(1)?,
            })
        })
        .map_err(unavailable)?
        .collect::<rusqlite::Result<_>>()
        .map_err(unavailable)?;
    Ok(OrderSnapshotRows {
        orders,
        versions: read_versions(connection, None, None, false)?,
        heads,
    })
}
pub(super) fn migrate_orders(connection: &Connection) -> anyhow::Result<()> {
    let rows = connection
        .prepare("SELECT sequence,payload FROM snapshot_outbox ORDER BY sequence")?
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (seq, payload) in rows {
        snapshot::validate_source(&payload, seq, 4)?;
    }
    connection.execute_batch("CREATE TABLE purchase_orders(id TEXT PRIMARY KEY NOT NULL,first_confirmation_id TEXT NOT NULL UNIQUE REFERENCES business_confirmations(id),created_at TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence));
        CREATE TABLE purchase_order_versions(id TEXT PRIMARY KEY NOT NULL,order_id TEXT NOT NULL REFERENCES purchase_orders(id),previous_version_id TEXT,confirmation_id TEXT NOT NULL UNIQUE REFERENCES business_confirmations(id),action TEXT NOT NULL,status TEXT NOT NULL,material_id TEXT NOT NULL,material_version_id TEXT NOT NULL,quantity_micros INTEGER NOT NULL CHECK(quantity_micros>0 AND quantity_micros<=9000000000000000),unit TEXT NOT NULL,input_quantity TEXT NOT NULL,input_unit TEXT NOT NULL,supplier TEXT,expected_date TEXT,estimated_amount_micros INTEGER CHECK(estimated_amount_micros>=0 AND estimated_amount_micros<=9000000000000000000),note TEXT NOT NULL,recorded_at TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),UNIQUE(order_id,id),FOREIGN KEY(order_id,previous_version_id) REFERENCES purchase_order_versions(order_id,id),FOREIGN KEY(material_id,material_version_id) REFERENCES material_versions(material_id,id),CHECK((action IN('place','revise') AND status='open') OR (action='cancel' AND status='cancelled')));
        CREATE TABLE purchase_order_heads(order_id TEXT PRIMARY KEY NOT NULL REFERENCES purchase_orders(id),current_version_id TEXT NOT NULL,FOREIGN KEY(order_id,current_version_id) REFERENCES purchase_order_versions(order_id,id));
        CREATE INDEX purchase_order_versions_order ON purchase_order_versions(order_id,sequence);
        PRAGMA user_version=5;")?;
    Ok(())
}
pub(super) fn validate_order_schema(connection: &Connection) -> anyhow::Result<()> {
    connection.prepare(
        "SELECT id,first_confirmation_id,created_at,sequence FROM purchase_orders LIMIT 0",
    )?;
    connection.prepare("SELECT order_id,current_version_id FROM purchase_order_heads LIMIT 0")?;
    let rows = order_snapshot_rows(connection)?;
    if rows.orders.len() != rows.heads.len() {
        anyhow::bail!("주문의 현재 버전 연결이 없습니다")
    }
    for order in rows.orders {
        let versions: Vec<_> = rows
            .versions
            .iter()
            .filter(|v| v.order_id == order.id)
            .collect();
        let Some(first) = versions.first() else {
            anyhow::bail!("주문 최초 버전이 없습니다")
        };
        if first.action != OrderAction::Place
            || first.previous_version_id.is_some()
            || first.confirmation_id != order.first_confirmation_id
        {
            anyhow::bail!("주문 최초 확정 연결이 올바르지 않습니다")
        }
        for pair in versions.windows(2) {
            if pair[1].previous_version_id.as_deref() != Some(pair[0].id.as_str())
                || pair[1].material_id != first.material_id
                || (pair[0].status != OrderStatus::Open && pair[1].action != OrderAction::Correct)
            {
                anyhow::bail!("주문 버전 연결이 올바르지 않습니다")
            }
        }
        if rows
            .heads
            .iter()
            .find(|h| h.order_id == order.id)
            .map(|h| h.current_version_id.as_str())
            != versions.last().map(|v| v.id.as_str())
        {
            anyhow::bail!("주문의 현재 버전이 최신 이력과 다릅니다")
        }
    }
    Ok(())
}
