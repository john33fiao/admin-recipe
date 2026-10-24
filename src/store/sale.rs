use super::{Store, StoreError, business, catalog, holding, recipe, snapshot, unavailable};
use crate::{
    business::{BusinessEffect, BusinessIssue, BusinessResult},
    catalog::{MaterialPriceVersion, MaterialVersion},
    domain::{HoldingBasis, Money, Quantity, UnitPrice, resolve_price},
    recipe::{
        ExactRatio, RecipeCostInput, RecipeCostLineInput, RecipeRatioUnavailable, cost_recipe,
        parse_basis_count,
    },
    sale::*,
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RecordRow {
    pub id: String,
    pub original_confirmation_id: String,
    pub business_record_id: String,
    pub material_id: String,
    pub material_version_id: String,
    pub material_name: String,
    pub recipe_version_id: String,
    pub basis_count: String,
    pub selling_price: Option<String>,
    pub known_subtotal: String,
    pub total_cost: Option<String>,
    pub cost_ratio: Option<ExactRatio>,
    pub ratio_unavailable_reason: Option<RecipeRatioUnavailable>,
    pub recorded_at: String,
    pub seq: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CostRow {
    pub sale_id: String,
    pub position: String,
    pub row: SaleRecipeCostRow,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Head {
    pub sale_id: String,
    pub current_version_id: String,
}
pub(super) struct SnapshotRows {
    pub records: Vec<RecordRow>,
    pub cost_rows: Vec<CostRow>,
    pub versions: Vec<SaleVersion>,
    pub heads: Vec<Head>,
}
pub(super) struct Validation<'a> {
    pub records: &'a [RecordRow],
    pub cost_rows: &'a [CostRow],
    pub versions: &'a [SaleVersion],
    pub heads: &'a [Head],
    pub recipe_versions: &'a [recipe::VersionRow],
    pub recipe_lines: &'a [recipe::LineRow],
    pub holdings: &'a [holding::HoldingVersion],
    pub prices: &'a [MaterialPriceVersion],
    pub material_versions: &'a [MaterialVersion],
    pub business_records: &'a [business::Record],
    pub revisions: &'a [crate::business::BusinessRevision],
    pub confirmations: &'a [business::ConfirmationRow],
    pub corrections: &'a [crate::business::BusinessCorrection],
    pub other_confirmation_ids: &'a [&'a str],
    pub mutations: &'a [(&'a str, &'a str, &'a str)],
    pub sequence: i64,
}

fn validation(error: impl std::fmt::Display) -> StoreError {
    StoreError::Validation(error.to_string())
}

fn money(value: i64) -> Result<String, StoreError> {
    Ok(Money::from_micros(value).map_err(unavailable)?.decimal())
}
fn optional_money(value: Option<i64>) -> Result<Option<String>, StoreError> {
    value.map(money).transpose()
}
fn amount(value: &Option<String>) -> Result<Option<i64>, StoreError> {
    value
        .as_deref()
        .map(|v| Money::parse(v).map(|m| m.micros()).map_err(unavailable))
        .transpose()
}
fn records(connection: &Connection, id: Option<&str>) -> Result<Vec<RecordRow>, StoreError> {
    let rows = connection.prepare("SELECT s.id,s.original_confirmation_id,c.record_id,s.material_id,s.material_version_id,m.name,s.recipe_version_id,s.basis_count,s.selling_price_micros,s.known_subtotal_micros,s.total_cost_micros,s.ratio_json,s.ratio_reason_json,c.recorded_at,c.sequence FROM sales s JOIN material_versions m ON m.id=s.material_version_id JOIN business_confirmations c ON c.id=s.original_confirmation_id WHERE (?1 IS NULL OR s.id=?1) ORDER BY c.sequence,s.id").map_err(unavailable)?
        .query_map([id], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,String>(5)?,r.get::<_,String>(6)?,r.get::<_,i64>(7)?,r.get::<_,Option<i64>>(8)?,r.get::<_,i64>(9)?,r.get::<_,Option<i64>>(10)?,r.get::<_,String>(11)?,r.get::<_,String>(12)?,r.get::<_,String>(13)?,r.get::<_,i64>(14)?))).map_err(unavailable)?
        .collect::<rusqlite::Result<Vec<_>>>().map_err(unavailable)?;
    rows.into_iter()
        .map(
            |(
                id,
                original_confirmation_id,
                business_record_id,
                material_id,
                material_version_id,
                material_name,
                recipe_version_id,
                basis,
                selling,
                known,
                total,
                ratio,
                reason,
                recorded_at,
                seq,
            )| {
                Ok(RecordRow {
                    id,
                    original_confirmation_id,
                    business_record_id,
                    material_id,
                    material_version_id,
                    material_name,
                    recipe_version_id,
                    basis_count: basis.to_string(),
                    selling_price: optional_money(selling)?,
                    known_subtotal: money(known)?,
                    total_cost: optional_money(total)?,
                    cost_ratio: serde_json::from_str(&ratio).map_err(unavailable)?,
                    ratio_unavailable_reason: serde_json::from_str(&reason).map_err(unavailable)?,
                    recorded_at,
                    seq: seq.to_string(),
                })
            },
        )
        .collect()
}
fn cost_rows(connection: &Connection, sale_id: Option<&str>) -> Result<Vec<CostRow>, StoreError> {
    let rows = connection.prepare("SELECT r.sale_id,r.position,r.recipe_line_id,l.material_id,l.material_version_id,m.name,l.quantity_micros,s.basis_count,l.unit,r.source_kind,COALESCE(r.holding_version_id,r.material_price_id),r.price_amount_micros,r.price_quantity_micros,r.missing_reason,r.cost_micros FROM sale_cost_rows r JOIN sales s ON s.id=r.sale_id JOIN recipe_lines l ON l.id=r.recipe_line_id JOIN material_versions m ON m.id=l.material_version_id WHERE (?1 IS NULL OR r.sale_id=?1) ORDER BY r.sale_id,r.position").map_err(unavailable)?
        .query_map([sale_id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,String>(5)?,r.get::<_,i64>(6)?,r.get::<_,i64>(7)?,r.get::<_,String>(8)?,r.get::<_,String>(9)?,r.get::<_,Option<String>>(10)?,r.get::<_,Option<i64>>(11)?,r.get::<_,Option<i64>>(12)?,r.get::<_,Option<String>>(13)?,r.get::<_,Option<i64>>(14)?))).map_err(unavailable)?
        .collect::<rusqlite::Result<Vec<_>>>().map_err(unavailable)?;
    rows.into_iter()
        .map(
            |(
                sale_id,
                position,
                line_id,
                material_id,
                material_version_id,
                material_name,
                q,
                basis,
                unit,
                source,
                source_id,
                a,
                pq,
                reason,
                cost,
            )| {
                let price = SalePriceSnapshot {
                    source: serde_json::from_value(serde_json::Value::String(source))
                        .map_err(unavailable)?,
                    source_id,
                    amount: optional_money(a)?,
                    quantity: pq
                        .map(|q| {
                            Quantity::from_micros(q)
                                .map(|q| q.decimal())
                                .map_err(unavailable)
                        })
                        .transpose()?,
                    reason,
                };
                price.validate().map_err(unavailable)?;
                Ok(CostRow {
                    sale_id,
                    position: position.to_string(),
                    row: SaleRecipeCostRow {
                        line_id,
                        material_id,
                        material_version_id,
                        material_name,
                        numerator_quantity: Quantity::from_micros(q)
                            .map_err(unavailable)?
                            .decimal(),
                        basis_count: basis.to_string(),
                        unit,
                        price,
                        cost: optional_money(cost)?,
                    },
                })
            },
        )
        .collect()
}
fn versions(
    connection: &Connection,
    sale_id: Option<&str>,
) -> Result<Vec<SaleVersion>, StoreError> {
    let rows = connection.prepare("SELECT v.id,v.sale_id,v.previous_version_id,v.confirmation_id,c.record_id,v.count,v.amount_micros,v.note,c.recorded_at,c.sequence FROM sale_versions v JOIN business_confirmations c ON c.id=v.confirmation_id WHERE (?1 IS NULL OR v.sale_id=?1) ORDER BY c.sequence,v.id").map_err(unavailable)?
        .query_map([sale_id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,Option<String>>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,i64>(5)?,r.get::<_,Option<i64>>(6)?,r.get::<_,String>(7)?,r.get::<_,String>(8)?,r.get::<_,i64>(9)?))).map_err(unavailable)?
        .collect::<rusqlite::Result<Vec<_>>>().map_err(unavailable)?;
    rows.into_iter()
        .map(
            |(
                id,
                sale_id,
                previous_version_id,
                confirmation_id,
                business_record_id,
                count,
                a,
                note,
                recorded_at,
                seq,
            )| {
                Ok(SaleVersion {
                    id,
                    sale_id,
                    previous_version_id,
                    confirmation_id,
                    business_record_id,
                    count: count.to_string(),
                    amount: optional_money(a)?,
                    note,
                    recorded_at,
                    seq: seq.to_string(),
                })
            },
        )
        .collect()
}
fn heads(connection: &Connection) -> Result<Vec<Head>, StoreError> {
    connection
        .prepare("SELECT sale_id,current_version_id FROM sale_heads ORDER BY sale_id")
        .map_err(unavailable)?
        .query_map([], |r| {
            Ok(Head {
                sale_id: r.get(0)?,
                current_version_id: r.get(1)?,
            })
        })
        .map_err(unavailable)?
        .collect::<rusqlite::Result<_>>()
        .map_err(unavailable)
}
fn full_original(row: RecordRow, rows: Vec<SaleRecipeCostRow>) -> SaleOriginal {
    let missing_price_line_ids = rows
        .iter()
        .filter(|r| r.price.source == SalePriceSource::Missing)
        .map(|r| r.line_id.clone())
        .collect();
    SaleOriginal {
        id: row.id,
        original_confirmation_id: row.original_confirmation_id,
        business_record_id: row.business_record_id,
        material_id: row.material_id,
        material_version_id: row.material_version_id,
        material_name: row.material_name,
        recipe_version_id: row.recipe_version_id.clone(),
        recorded_at: row.recorded_at,
        seq: row.seq,
        recipe_cost: SaleRecipeCost {
            recipe_version_id: row.recipe_version_id,
            basis_count: row.basis_count,
            rows,
            missing_price_line_ids,
            known_subtotal: row.known_subtotal,
            total_cost: row.total_cost,
            selling_price: row.selling_price,
            cost_ratio: row.cost_ratio,
            ratio_unavailable_reason: row.ratio_unavailable_reason,
        },
    }
}
fn detail(connection: &Connection, id: &str) -> Result<Option<SaleDetail>, StoreError> {
    let Some(row) = records(connection, Some(id))?.into_iter().next() else {
        return Ok(None);
    };
    let current: String = connection
        .query_row(
            "SELECT current_version_id FROM sale_heads WHERE sale_id=?1",
            [id],
            |r| r.get(0),
        )
        .map_err(unavailable)?;
    let versions = versions(connection, Some(id))?;
    let head = versions
        .iter()
        .find(|v| v.id == current)
        .cloned()
        .ok_or_else(|| unavailable("판매의 현재 개정이 없습니다"))?;
    let item = full_original(
        row,
        cost_rows(connection, Some(id))?
            .into_iter()
            .map(|r| r.row)
            .collect(),
    );
    Ok(Some(SaleDetail {
        item,
        head,
        versions,
        consumption_status: SaleConsumptionStatus::Unconfirmed,
        state: catalog::state(connection)?,
    }))
}
pub(super) fn result(
    connection: &Connection,
    confirmation: &str,
) -> Result<BusinessResult, StoreError> {
    connection
        .query_row(
            "SELECT sale_id,id FROM sale_versions WHERE confirmation_id=?1",
            [confirmation],
            |r| {
                Ok(BusinessResult::Sale {
                    sale_id: r.get(0)?,
                    sale_version_id: r.get(1)?,
                })
            },
        )
        .map_err(unavailable)
}
fn target(connection: &Connection, confirmation: &str) -> Result<SaleDetail, StoreError> {
    let id: Option<String> = connection
        .query_row(
            "SELECT sale_id FROM sale_versions WHERE confirmation_id=?1",
            [confirmation],
            |r| r.get(0),
        )
        .optional()
        .map_err(unavailable)?;
    let item = id
        .and_then(|id| detail(connection, &id).transpose())
        .transpose()?
        .ok_or_else(|| validation("판매 정정 대상을 찾을 수 없습니다"))?;
    Ok(item)
}
pub(super) fn check_correction(
    connection: &Connection,
    confirmation: &str,
    data: &DraftSaleV1,
) -> Result<(), StoreError> {
    let detail = target(connection, confirmation)?;
    if detail.head.confirmation_id != confirmation {
        return Err(StoreError::StaleState(
            "대상 판매는 이미 정정되었습니다. 최신 확정을 확인하세요".into(),
        ));
    }
    let item = detail.item;
    if data.material_id.as_deref() != Some(&item.material_id)
        || data.material_version_id.as_deref() != Some(&item.material_version_id)
        || data.recipe_version_id.as_deref() != Some(&item.recipe_version_id)
    {
        return Err(validation(
            "판매 정정의 당시 품목과 레시피를 변경할 수 없습니다",
        ));
    }
    Ok(())
}
pub(super) fn material_name(
    connection: &Connection,
    data: &DraftSaleV1,
) -> Result<Option<String>, StoreError> {
    match (&data.material_id, &data.material_version_id) {
        (Some(id), Some(version)) => Ok(catalog::versions(connection, Some(id))?
            .into_iter()
            .find(|v| &v.id == version)
            .map(|v| v.name)),
        _ => Ok(None),
    }
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
fn current_cost(
    connection: &Connection,
    owner: &str,
    version: &str,
) -> Result<SaleRecipeCost, StoreError> {
    let cost = recipe::cost_at_current(connection, owner, Some(version))?
        .ok_or_else(|| validation("판매할 레시피를 선택하세요"))?;
    let materials = catalog::versions(connection, None)?;
    let rows = cost
        .rows
        .into_iter()
        .map(|r| {
            let material = materials
                .iter()
                .find(|m| m.id == r.material_version_id)
                .ok_or_else(|| unavailable("레시피 구성 품목 버전이 없습니다"))?;
            Ok(SaleRecipeCostRow {
                line_id: r.line_id,
                material_id: r.material_id,
                material_version_id: r.material_version_id,
                material_name: material.name.clone(),
                numerator_quantity: r.numerator_quantity,
                basis_count: r.basis_count,
                unit: r.unit,
                price: SalePriceSnapshot::from_resolved(r.price).map_err(unavailable)?,
                cost: r.cost,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    Ok(SaleRecipeCost {
        recipe_version_id: cost.recipe_version_id,
        basis_count: cost.basis_count,
        rows,
        missing_price_line_ids: cost.missing_price_line_ids,
        known_subtotal: cost.known_subtotal,
        total_cost: cost.total_cost,
        selling_price: cost.selling_price,
        cost_ratio: cost.cost_ratio,
        ratio_unavailable_reason: cost.ratio_unavailable_reason,
    })
}
pub(super) fn inspect_sale(
    connection: &Connection,
    data: &DraftSaleV1,
    corrects: Option<&str>,
) -> Result<(Vec<BusinessIssue>, Option<BusinessEffect>), StoreError> {
    let mut issues = vec![];
    let count = match SaleCount::parse(&data.count) {
        Ok(c) => Some(c),
        Err(e) => {
            issue(&mut issues, "count", "invalid_count", e);
            None
        }
    };
    let previous = corrects.map(|id| target(connection, id)).transpose()?;
    let basis = if let Some(old) = &previous {
        if Some(old.head.confirmation_id.as_str()) != corrects {
            issue(
                &mut issues,
                "corrects_confirmation_id",
                "stale_reference",
                "대상 판매는 이미 정정되었습니다. 최신 확정을 확인하세요",
            );
        }
        if data.material_id.as_deref() != Some(&old.item.material_id)
            || data.material_version_id.as_deref() != Some(&old.item.material_version_id)
            || data.recipe_version_id.as_deref() != Some(&old.item.recipe_version_id)
        {
            issue(
                &mut issues,
                "recipe_version_id",
                "wrong_correction_target",
                "판매 당시 품목과 레시피를 유지하세요",
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
        Some((
            old.item.material_id.clone(),
            old.item.material_version_id.clone(),
            old.item.material_name.clone(),
            old.item.recipe_version_id.clone(),
            old.item.recipe_cost.clone(),
        ))
    } else {
        let material = data
            .material_id
            .as_deref()
            .map(|id| catalog::current_version(connection, id))
            .transpose()?
            .flatten();
        if let Some(m) = &material {
            if data.material_version_id.as_deref() != Some(&m.id) {
                issue(
                    &mut issues,
                    "material_version_id",
                    "stale_reference",
                    "최신 품목을 선택해 다시 저장하세요",
                );
            }
        } else {
            issue(
                &mut issues,
                "material_id",
                "missing_reference",
                "판매할 품목을 선택하세요",
            );
        }
        let current = material
            .as_ref()
            .map(|m| {
                connection
                    .query_row(
                        "SELECT current_version_id FROM recipe_heads WHERE material_id=?1",
                        [&m.material_id],
                        |r| r.get::<_, String>(0),
                    )
                    .optional()
                    .map_err(unavailable)
            })
            .transpose()?
            .flatten();
        if current.is_none() {
            issue(
                &mut issues,
                "recipe_version_id",
                "missing_reference",
                "레시피를 등록한 품목을 선택하세요",
            );
        } else if data.recipe_version_id.as_ref() != current.as_ref() {
            issue(
                &mut issues,
                "recipe_version_id",
                "stale_reference",
                "최신 레시피를 선택해 다시 저장하세요",
            );
        }
        match (material, current) {
            (Some(m), Some(recipe)) if issues.is_empty() => {
                match current_cost(connection, &m.material_id, &recipe) {
                    Ok(cost) => Some((m.material_id, m.id, m.name, recipe, cost)),
                    Err(StoreError::Validation(message)) => {
                        issue(&mut issues, "recipe_version_id", "invalid_cost", message);
                        None
                    }
                    Err(error) => return Err(error),
                }
            }
            _ => None,
        }
    };
    if let (
        Some(count),
        Some((material_id, material_version_id, material_name, recipe_version_id, recipe_cost)),
    ) = (count, basis)
    {
        let selling = recipe_cost
            .selling_price
            .as_deref()
            .map(Money::parse)
            .transpose()
            .map_err(unavailable)?;
        match calculate_sale_amount(count, selling) {
            Ok(amount) if issues.is_empty() => {
                return Ok((
                    issues,
                    Some(BusinessEffect::Sale {
                        sale: SaleProposal {
                            material_id,
                            material_version_id,
                            material_name,
                            recipe_version_id,
                            count: count.decimal(),
                            amount: amount.map(|m| m.decimal()),
                            recipe_cost,
                        },
                        corrects_confirmation_id: corrects.map(str::to_owned),
                        previous: previous.map(|d| d.head),
                    }),
                ));
            }
            Ok(_) => {}
            Err(e) => issue(&mut issues, "count", "invalid_amount", e),
        }
    }
    Ok((issues, None))
}

pub(super) fn apply_sale(
    connection: &Connection,
    data: &DraftSaleV1,
    corrects: Option<&str>,
    confirmation: &str,
) -> Result<(), StoreError> {
    let (_, Some(BusinessEffect::Sale { sale, previous, .. })) =
        inspect_sale(connection, data, corrects)?
    else {
        return Err(validation("판매 입력을 확인하세요"));
    };
    let sale_id = if let Some(previous) = &previous {
        previous.sale_id.clone()
    } else {
        let id = Uuid::new_v4().to_string();
        let cost = &sale.recipe_cost;
        connection.execute("INSERT INTO sales(id,original_confirmation_id,material_id,material_version_id,recipe_version_id,basis_count,selling_price_micros,known_subtotal_micros,total_cost_micros,ratio_json,ratio_reason_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![id,confirmation,sale.material_id,sale.material_version_id,sale.recipe_version_id,parse_basis_count(&cost.basis_count).map_err(unavailable)?,amount(&cost.selling_price)?,Money::parse(&cost.known_subtotal).map_err(unavailable)?.micros(),amount(&cost.total_cost)?,serde_json::to_string(&cost.cost_ratio).map_err(unavailable)?,serde_json::to_string(&cost.ratio_unavailable_reason).map_err(unavailable)?]).map_err(unavailable)?;
        for (position, row) in cost.rows.iter().enumerate() {
            let (holding, price) = match row.price.source {
                SalePriceSource::HoldingAverage | SalePriceSource::LastValid => {
                    (row.price.source_id.as_deref(), None)
                }
                SalePriceSource::Preregistered => (None, row.price.source_id.as_deref()),
                SalePriceSource::Missing => (None, None),
            };
            let source = serde_json::to_value(row.price.source)
                .map_err(unavailable)?
                .as_str()
                .ok_or_else(|| unavailable("가격 출처가 없습니다"))?
                .to_owned();
            let q = row
                .price
                .quantity
                .as_deref()
                .map(Quantity::parse)
                .transpose()
                .map_err(unavailable)?
                .map(|q| q.micros());
            connection.execute("INSERT INTO sale_cost_rows(sale_id,position,recipe_line_id,source_kind,holding_version_id,material_price_id,price_amount_micros,price_quantity_micros,missing_reason,cost_micros) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                params![id,position as i64,row.line_id,source,holding,price,amount(&row.price.amount)?,q,row.price.reason,amount(&row.cost)?]).map_err(unavailable)?;
        }
        id
    };
    let version_id = Uuid::new_v4().to_string();
    connection.execute("INSERT INTO sale_versions(id,sale_id,previous_version_id,confirmation_id,count,amount_micros,note) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![version_id,sale_id,previous.map(|v|v.id),confirmation,SaleCount::parse(&sale.count).map_err(unavailable)?.value(),amount(&sale.amount)?,data.note]).map_err(unavailable)?;
    connection.execute("INSERT INTO sale_heads(sale_id,current_version_id) VALUES(?1,?2) ON CONFLICT(sale_id) DO UPDATE SET current_version_id=excluded.current_version_id",params![sale_id,version_id]).map_err(unavailable)?;
    Ok(())
}

impl Store {
    pub fn sale_detail(&self, id: &str) -> Result<Option<SaleDetail>, StoreError> {
        detail(&self.connection, &catalog::uuid(id, "판매 ID")?)
    }
    pub fn sale_list(&self, filter: SaleFilter) -> Result<SaleList, StoreError> {
        let material = filter
            .material_id
            .as_deref()
            .map(|id| catalog::uuid(id, "품목 ID"))
            .transpose()?;
        let mut items = vec![];
        let mut amounts = vec![];
        for row in records(&self.connection, None)?
            .into_iter()
            .filter(|r| material.as_ref().is_none_or(|id| id == &r.material_id))
        {
            let sale = detail(&self.connection, &row.id)?
                .ok_or_else(|| unavailable("판매 원본이 없습니다"))?;
            amounts.push(
                sale.head
                    .amount
                    .as_deref()
                    .map(Money::parse)
                    .transpose()
                    .map_err(unavailable)?,
            );
            items.push(SaleSummary {
                id: row.id,
                material_id: row.material_id,
                material_name: row.material_name,
                recipe_version_id: row.recipe_version_id,
                current_version_id: sale.head.id,
                count: sale.head.count,
                selling_price: row.selling_price,
                amount: sale.head.amount,
                original_seq: row.seq,
                recorded_at: row.recorded_at,
                updated_at: sale.head.recorded_at,
                consumption_status: SaleConsumptionStatus::Unconfirmed,
            });
        }
        let (known_revenue, missing_revenue_count) =
            sum_sale_amounts(&amounts).map_err(unavailable)?;
        Ok(SaleList {
            items,
            known_revenue,
            missing_revenue_count,
            state: catalog::state(&self.connection)?,
        })
    }
    pub fn allocation_inputs(
        &self,
        material_id: &str,
        after_seq: i64,
        before_seq: i64,
        as_of_revision: i64,
    ) -> Result<Vec<SaleConsumptionSource>, StoreError> {
        let material_id = catalog::uuid(material_id, "배분 품목 ID")?;
        let current = catalog::state(&self.connection)?
            .revision
            .parse::<i64>()
            .map_err(unavailable)?;
        if as_of_revision < 0 || as_of_revision > current {
            return Err(validation("배분 기준 순번을 확인하세요"));
        }
        let all_versions = versions(&self.connection, None)?;
        let all_rows = cost_rows(&self.connection, None)?;
        let mut sources = vec![];
        for record in records(&self.connection, None)? {
            let Some(version) = all_versions
                .iter()
                .filter(|v| {
                    v.sale_id == record.id
                        && v.seq.parse::<i64>().is_ok_and(|s| s <= as_of_revision)
                })
                .max_by_key(|v| v.seq.parse::<i64>().unwrap_or(0))
            else {
                continue;
            };
            let mut direct_material_ids = all_rows
                .iter()
                .filter(|r| r.sale_id == record.id)
                .map(|r| r.row.material_id.clone())
                .collect::<Vec<_>>();
            direct_material_ids.sort();
            direct_material_ids.dedup();
            sources.push(SaleConsumptionSource {
                sale_id: record.id,
                sale_version_id: version.id.clone(),
                recipe_version_id: record.recipe_version_id,
                original_seq: record.seq.parse().map_err(unavailable)?,
                count: SaleCount::parse(&version.count).map_err(unavailable)?,
                direct_material_ids,
            });
        }
        eligible_sales(
            &SaleInterval {
                material_id,
                after_seq,
                before_seq,
            },
            &sources,
        )
        .map_err(validation)
    }
}

pub(super) fn snapshot_rows(connection: &Connection) -> Result<SnapshotRows, StoreError> {
    Ok(SnapshotRows {
        records: records(connection, None)?,
        cost_rows: cost_rows(connection, None)?,
        versions: versions(connection, None)?,
        heads: heads(connection)?,
    })
}
pub(super) fn migrate_sales(connection: &Connection) -> anyhow::Result<()> {
    let journals = connection
        .prepare("SELECT sequence,payload FROM snapshot_outbox ORDER BY sequence")?
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (seq, payload) in journals {
        snapshot::validate_source(&payload, seq, 8)?;
    }
    connection.execute_batch("CREATE TABLE sales(
        id TEXT PRIMARY KEY NOT NULL,original_confirmation_id TEXT NOT NULL UNIQUE REFERENCES business_confirmations(id),
        material_id TEXT NOT NULL,material_version_id TEXT NOT NULL,recipe_version_id TEXT NOT NULL,
        basis_count INTEGER NOT NULL CHECK(basis_count>0 AND basis_count<=1000000000),
        selling_price_micros INTEGER CHECK(selling_price_micros>=0 AND selling_price_micros<=9000000000000000000),
        known_subtotal_micros INTEGER NOT NULL CHECK(known_subtotal_micros>=0 AND known_subtotal_micros<=9000000000000000000),
        total_cost_micros INTEGER CHECK(total_cost_micros>=0 AND total_cost_micros<=9000000000000000000),
        ratio_json TEXT NOT NULL,ratio_reason_json TEXT NOT NULL,
        FOREIGN KEY(material_id,material_version_id) REFERENCES material_versions(material_id,id),
        FOREIGN KEY(material_id,recipe_version_id) REFERENCES recipe_versions(material_id,id));
      CREATE TABLE sale_cost_rows(
        sale_id TEXT NOT NULL REFERENCES sales(id),position INTEGER NOT NULL CHECK(position>=0 AND position<100),
        recipe_line_id TEXT NOT NULL REFERENCES recipe_lines(id),source_kind TEXT NOT NULL CHECK(source_kind IN('holding_average','last_valid','preregistered','missing')),
        holding_version_id TEXT REFERENCES holding_versions(id),material_price_id TEXT REFERENCES material_prices(id),
        price_amount_micros INTEGER CHECK(price_amount_micros>=0 AND price_amount_micros<=9000000000000000000),
        price_quantity_micros INTEGER CHECK(price_quantity_micros>0 AND price_quantity_micros<=9000000000000000),
        missing_reason TEXT,cost_micros INTEGER CHECK(cost_micros>=0 AND cost_micros<=9000000000000000000),
        PRIMARY KEY(sale_id,position),UNIQUE(sale_id,recipe_line_id),
        CHECK((source_kind='missing' AND holding_version_id IS NULL AND material_price_id IS NULL AND price_amount_micros IS NULL AND price_quantity_micros IS NULL AND missing_reason IS NOT NULL)
           OR (source_kind='preregistered' AND holding_version_id IS NULL AND material_price_id IS NOT NULL AND price_amount_micros IS NOT NULL AND price_quantity_micros IS NOT NULL AND missing_reason IS NULL)
           OR (source_kind IN('holding_average','last_valid') AND holding_version_id IS NOT NULL AND material_price_id IS NULL AND price_amount_micros IS NOT NULL AND price_quantity_micros IS NOT NULL AND missing_reason IS NULL)));
      CREATE TABLE sale_versions(
        id TEXT PRIMARY KEY NOT NULL,sale_id TEXT NOT NULL REFERENCES sales(id),previous_version_id TEXT,
        confirmation_id TEXT NOT NULL UNIQUE REFERENCES business_confirmations(id),count INTEGER NOT NULL CHECK(count>=0 AND count<=1000000000),
        amount_micros INTEGER CHECK(amount_micros>=0 AND amount_micros<=9000000000000000000),note TEXT NOT NULL,
        UNIQUE(sale_id,id),FOREIGN KEY(sale_id,previous_version_id) REFERENCES sale_versions(sale_id,id));
      CREATE TABLE sale_heads(
        sale_id TEXT PRIMARY KEY NOT NULL REFERENCES sales(id),current_version_id TEXT NOT NULL,
        FOREIGN KEY(sale_id,current_version_id) REFERENCES sale_versions(sale_id,id));
      CREATE INDEX sales_material ON sales(material_id);
      CREATE INDEX sale_versions_sale ON sale_versions(sale_id);
      PRAGMA user_version=9;")?;
    Ok(())
}

fn sequence(text: &str) -> Result<i64, StoreError> {
    let value = text.parse::<i64>().map_err(unavailable)?;
    if value <= 0 || value.to_string() != text {
        return Err(unavailable("판매 근거의 순번이 올바르지 않습니다"));
    }
    Ok(value)
}
fn identifier(text: &str) -> Result<(), StoreError> {
    if Uuid::parse_str(text).map_err(unavailable)?.to_string() != text {
        return Err(unavailable("판매 근거의 ID가 올바르지 않습니다"));
    }
    Ok(())
}
fn existed_before(material: &MaterialVersion, seq: i64) -> bool {
    match material.seq.as_deref() {
        Some(s) => sequence(s).is_ok_and(|s| s < seq),
        None => material.source == "legacy_migration",
    }
}
fn historical_price(
    state: &Validation<'_>,
    material: &str,
    seq: i64,
) -> Result<SalePriceSnapshot, StoreError> {
    let holdings = state
        .holdings
        .iter()
        .filter(|h| h.material_id == material && sequence(&h.seq).is_ok_and(|s| s < seq))
        .collect::<Vec<_>>();
    let current = holdings
        .iter()
        .max_by_key(|h| sequence(&h.seq).unwrap_or(0));
    let holding = match current {
        Some(h) => HoldingBasis::Known {
            quantity: Quantity::parse(&h.quantity).map_err(unavailable)?,
            value: Money::parse(&h.value).map_err(unavailable)?,
            source_id: h.id.clone(),
        },
        None => HoldingBasis::NotEstablished,
    };
    let last = holdings
        .iter()
        .filter(|h| Quantity::parse(&h.quantity).is_ok_and(|q| q.micros() > 0))
        .max_by_key(|h| sequence(&h.seq).unwrap_or(0))
        .map(|h| {
            UnitPrice::new(
                Money::parse(&h.value)?,
                Quantity::parse(&h.quantity)?,
                h.id.clone(),
            )
        })
        .transpose()
        .map_err(unavailable)?;
    let preregistered = state
        .prices
        .iter()
        .filter(|p| p.material_id == material && sequence(&p.seq).is_ok_and(|s| s < seq))
        .max_by_key(|p| sequence(&p.seq).unwrap_or(0))
        .map(|p| {
            UnitPrice::new(
                Money::parse(&p.amount)?,
                Quantity::parse(&p.quantity)?,
                p.id.clone(),
            )
        })
        .transpose()
        .map_err(unavailable)?;
    SalePriceSnapshot::from_resolved(resolve_price(holding, last, preregistered))
        .map_err(unavailable)
}
pub(super) fn validate_snapshot(state: Validation<'_>) -> Result<(), StoreError> {
    use crate::business::{BusinessKind, BusinessPayload, BusinessStatus};
    let bad = || unavailable("판매의 원본·개정·레시피·가격·확정 근거가 일치하지 않습니다");
    let materials = state
        .material_versions
        .iter()
        .map(|m| (&m.id, m))
        .collect::<HashMap<_, _>>();
    let recipes = state
        .recipe_versions
        .iter()
        .map(|r| (&r.id, r))
        .collect::<HashMap<_, _>>();
    let records = state
        .business_records
        .iter()
        .map(|r| (&r.id, r))
        .collect::<HashMap<_, _>>();
    let confirmations = state
        .confirmations
        .iter()
        .map(|c| (&c.id, c))
        .collect::<HashMap<_, _>>();
    let revisions = state
        .revisions
        .iter()
        .map(|r| (&r.id, r))
        .collect::<HashMap<_, _>>();
    if materials.len() != state.material_versions.len()
        || recipes.len() != state.recipe_versions.len()
        || records.len() != state.business_records.len()
        || confirmations.len() != state.confirmations.len()
        || revisions.len() != state.revisions.len()
    {
        return Err(bad());
    }
    let mut sale_ids = HashSet::new();
    let mut version_ids = HashSet::new();
    let mut confirmation_ids = HashSet::new();
    let mut sequences = HashSet::new();
    let mut used_rows = 0;
    for item in state.records {
        for id in [
            &item.id,
            &item.original_confirmation_id,
            &item.business_record_id,
            &item.material_id,
            &item.material_version_id,
            &item.recipe_version_id,
        ] {
            identifier(id)?;
        }
        chrono::DateTime::parse_from_rfc3339(&item.recorded_at).map_err(unavailable)?;
        let seq = sequence(&item.seq)?;
        let material = materials.get(&item.material_version_id).ok_or_else(bad)?;
        let recipe = recipes.get(&item.recipe_version_id).ok_or_else(bad)?;
        let latest_owner = state
            .material_versions
            .iter()
            .filter(|m| m.material_id == item.material_id && existed_before(m, seq))
            .max_by_key(|m| m.seq.as_deref().and_then(|s| sequence(s).ok()).unwrap_or(0))
            .ok_or_else(bad)?;
        let latest_recipe = state
            .recipe_versions
            .iter()
            .filter(|r| {
                r.material_id == item.material_id && sequence(&r.seq).is_ok_and(|s| s < seq)
            })
            .max_by_key(|r| sequence(&r.seq).unwrap_or(0))
            .ok_or_else(bad)?;
        if !sale_ids.insert(&item.id)
            || seq > state.sequence
            || material.material_id != item.material_id
            || material.name != item.material_name
            || latest_owner.id != material.id
            || latest_recipe.id != recipe.id
            || recipe.material_id != item.material_id
            || item.basis_count != recipe.basis_count
            || item.selling_price != recipe.selling_price
        {
            return Err(bad());
        }
        let mut lines = state
            .recipe_lines
            .iter()
            .filter(|l| l.recipe_version_id == recipe.id)
            .collect::<Vec<_>>();
        lines.sort_by_key(|l| l.line.position.parse::<usize>().unwrap_or(usize::MAX));
        let mut rows = state
            .cost_rows
            .iter()
            .filter(|r| r.sale_id == item.id)
            .collect::<Vec<_>>();
        rows.sort_by_key(|r| r.position.parse::<usize>().unwrap_or(usize::MAX));
        if rows.len() != lines.len() {
            return Err(bad());
        }
        let mut inputs = Vec::new();
        for (position, (row, line)) in rows.iter().zip(&lines).enumerate() {
            let r = &row.row;
            let l = &line.line;
            let component = materials.get(&l.material_version_id).ok_or_else(bad)?;
            if row.position != position.to_string()
                || l.position != row.position
                || r.line_id != l.id
                || r.material_id != l.material_id
                || r.material_version_id != l.material_version_id
                || r.material_name != component.name
                || r.numerator_quantity != l.quantity
                || r.basis_count != item.basis_count
                || r.unit != l.unit
            {
                return Err(bad());
            }
            r.price.validate().map_err(unavailable)?;
            let expected = historical_price(&state, &l.material_id, seq)?;
            if r.price.source != expected.source
                || r.price.source_id != expected.source_id
                || r.price.amount != expected.amount
                || r.price.quantity != expected.quantity
            {
                return Err(bad());
            }
            let price = match (&r.price.amount, &r.price.quantity, &r.price.source_id) {
                (Some(a), Some(q), Some(id)) => Some(
                    UnitPrice::new(
                        Money::parse(a).map_err(unavailable)?,
                        Quantity::parse(q).map_err(unavailable)?,
                        id.clone(),
                    )
                    .map_err(unavailable)?,
                ),
                (None, None, None) => None,
                _ => return Err(bad()),
            };
            inputs.push(RecipeCostLineInput {
                numerator: Quantity::parse(&l.quantity).map_err(unavailable)?,
                price,
            });
        }
        let selling = item
            .selling_price
            .as_deref()
            .map(Money::parse)
            .transpose()
            .map_err(unavailable)?;
        let computed = cost_recipe(&RecipeCostInput {
            basis_count: parse_basis_count(&item.basis_count).map_err(unavailable)?,
            rows: inputs,
            selling_price: selling,
        })
        .map_err(unavailable)?;
        if item.known_subtotal != computed.known_subtotal.decimal()
            || item.total_cost != computed.total_cost.map(|m| m.decimal())
            || item.cost_ratio != computed.cost_ratio
            || item.ratio_unavailable_reason != computed.ratio_unavailable_reason
        {
            return Err(bad());
        }
        for (row, cost) in rows.iter().zip(computed.row_costs) {
            if row.row.cost != cost.map(|m| m.decimal()) {
                return Err(bad());
            }
        }
        used_rows += rows.len();
        let mut versions = state
            .versions
            .iter()
            .filter(|v| v.sale_id == item.id)
            .collect::<Vec<_>>();
        versions.sort_by_key(|v| sequence(&v.seq).unwrap_or(0));
        if versions.is_empty() {
            return Err(bad());
        }
        let mut previous: Option<&SaleVersion> = None;
        for version in &versions {
            for id in [
                &version.id,
                &version.sale_id,
                &version.confirmation_id,
                &version.business_record_id,
            ] {
                identifier(id)?;
            }
            chrono::DateTime::parse_from_rfc3339(&version.recorded_at).map_err(unavailable)?;
            let version_seq = sequence(&version.seq)?;
            let c = confirmations
                .get(&version.confirmation_id)
                .ok_or_else(bad)?;
            identifier(&c.request_id)?;
            let record = records.get(&version.business_record_id).ok_or_else(bad)?;
            let revision = revisions.get(&c.revision_id).ok_or_else(bad)?;
            let BusinessPayload::Sale { version: 1, data } = &revision.payload else {
                return Err(bad());
            };
            let count = SaleCount::parse(&version.count).map_err(unavailable)?;
            if !version_ids.insert(&version.id)
                || !confirmation_ids.insert(&version.confirmation_id)
                || !sequences.insert(version_seq)
                || version_seq > state.sequence
                || state
                    .other_confirmation_ids
                    .contains(&version.confirmation_id.as_str())
                || state
                    .mutations
                    .iter()
                    .filter(|(s, op, request)| {
                        *s == version.seq && *op == "business.confirm" && *request == c.request_id
                    })
                    .count()
                    != 1
                || c.record_id != version.business_record_id
                || c.seq != version.seq
                || c.recorded_at != version.recorded_at
                || record.kind != BusinessKind::Sale
                || revision.record_id != record.id
                || revision.status != BusinessStatus::Confirmed
                || revision.seq != version.seq
                || revision.recorded_at != version.recorded_at
                || count.decimal() != version.count
                || SaleCount::parse(&data.count).map_err(unavailable)? != count
                || version.amount
                    != calculate_sale_amount(count, selling)
                        .map_err(unavailable)?
                        .map(|m| m.decimal())
                || version.note != data.note
                || data.material_id.as_deref() != Some(&item.material_id)
                || data.material_version_id.as_deref() != Some(&item.material_version_id)
                || data.recipe_version_id.as_deref() != Some(&item.recipe_version_id)
            {
                return Err(bad());
            }
            if let Some(old) = previous {
                if version.previous_version_id.as_ref() != Some(&old.id)
                    || record.corrects_confirmation_id.as_ref() != Some(&old.confirmation_id)
                    || version.note.trim().is_empty()
                    || sequence(&old.seq)? >= version_seq
                    || state
                        .corrections
                        .iter()
                        .filter(|e| {
                            e.target_confirmation_id == old.confirmation_id
                                && e.replacement_confirmation_id == version.confirmation_id
                        })
                        .count()
                        != 1
                {
                    return Err(bad());
                }
            } else if version.previous_version_id.is_some()
                || record.corrects_confirmation_id.is_some()
                || version.confirmation_id != item.original_confirmation_id
                || version.business_record_id != item.business_record_id
                || version.seq != item.seq
                || version.recorded_at != item.recorded_at
            {
                return Err(bad());
            }
            previous = Some(version);
        }
        let heads = state
            .heads
            .iter()
            .filter(|h| h.sale_id == item.id)
            .collect::<Vec<_>>();
        if heads.len() != 1 || heads[0].current_version_id != versions.last().ok_or_else(bad)?.id {
            return Err(bad());
        }
    }
    if state.heads.len() != sale_ids.len()
        || state.versions.len() != version_ids.len()
        || state.cost_rows.len() != used_rows
    {
        return Err(bad());
    }
    for c in state.confirmations {
        if records
            .get(&c.record_id)
            .is_some_and(|r| r.kind == BusinessKind::Sale)
            && !confirmation_ids.contains(&c.id)
        {
            return Err(bad());
        }
    }
    for edge in state.corrections {
        if confirmation_ids.contains(&edge.target_confirmation_id)
            || confirmation_ids.contains(&edge.replacement_confirmation_id)
        {
            let old = state
                .versions
                .iter()
                .find(|v| v.confirmation_id == edge.target_confirmation_id)
                .ok_or_else(bad)?;
            let new = state
                .versions
                .iter()
                .find(|v| v.confirmation_id == edge.replacement_confirmation_id)
                .ok_or_else(bad)?;
            if new.sale_id != old.sale_id || new.previous_version_id.as_ref() != Some(&old.id) {
                return Err(bad());
            }
        }
    }
    Ok(())
}

pub(super) fn validate_sale_schema(connection: &Connection) -> anyhow::Result<()> {
    let rows = snapshot_rows(connection)?;
    let recipe = recipe::snapshot_rows(connection)?;
    let business = business::snapshot_rows(connection)?;
    let holdings = holding::versions(connection, None)?;
    let materials = catalog::versions(connection, None)?;
    let prices = catalog::prices(connection, None)?;
    let others=connection.prepare("SELECT confirmation_id FROM business_catalog_price_results UNION ALL SELECT confirmation_id FROM purchase_order_versions UNION ALL SELECT confirmation_id FROM receipts UNION ALL SELECT confirmation_id FROM productions")?.query_map([],|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
    let mutations = connection
        .prepare("SELECT sequence,operation,request_id FROM mutation_requests ORDER BY sequence")?
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?.to_string(),
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let sequence = connection.query_row(
        "SELECT COALESCE(MAX(sequence),0) FROM snapshot_outbox",
        [],
        |r| r.get(0),
    )?;
    validate_snapshot(Validation {
        records: &rows.records,
        cost_rows: &rows.cost_rows,
        versions: &rows.versions,
        heads: &rows.heads,
        recipe_versions: &recipe.versions,
        recipe_lines: &recipe.lines,
        holdings: &holdings,
        prices: &prices,
        material_versions: &materials,
        business_records: &business.records,
        revisions: &business.revisions,
        confirmations: &business.confirmations,
        corrections: &business.corrections,
        other_confirmation_ids: &others.iter().map(String::as_str).collect::<Vec<_>>(),
        mutations: &mutations
            .iter()
            .map(|(s, op, request)| (s.as_str(), op.as_str(), request.as_str()))
            .collect::<Vec<_>>(),
        sequence,
    })?;
    Ok(())
}
