//! Validate immutable correction evidence before trusting effective heads.
use super::*;
use crate::business::{BusinessKind, BusinessPayload, BusinessStatus};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

mod projections;

fn bad(message: &str) -> StoreError {
    unavailable(message)
}
fn identifier(value: &str) -> Result<(), StoreError> {
    if Uuid::parse_str(value).map_err(unavailable)?.to_string() != value {
        return Err(bad("정정 참조는 정규 UUID여야 합니다"));
    }
    Ok(())
}
fn number(value: &str, maximum: i64) -> Result<i64, StoreError> {
    let n = value.parse::<i64>().map_err(unavailable)?;
    if n <= 0 || n > maximum || n.to_string() != value {
        return Err(bad("정정 기록 순번이 올바르지 않습니다"));
    }
    Ok(n)
}
fn equal<T: Serialize>(left: &T, right: &T) -> Result<bool, StoreError> {
    Ok(serde_json::to_value(left).map_err(unavailable)?
        == serde_json::to_value(right).map_err(unavailable)?)
}
fn time(value: &str) -> Result<(), StoreError> {
    chrono::DateTime::parse_from_rfc3339(value).map_err(unavailable)?;
    Ok(())
}
fn new_id<'a>(ids: &mut BTreeSet<&'a str>, value: &'a str) -> Result<(), StoreError> {
    identifier(value)?;
    if !ids.insert(value) {
        return Err(bad("정정의 새 식별자가 중복됐습니다"));
    }
    Ok(())
}

fn validate_identities(s: &Validation<'_>) -> Result<(), StoreError> {
    if s.records.is_empty()
        && s.event_versions.is_empty()
        && s.projection_versions.is_empty()
        && s.price_bases.is_empty()
    {
        return Ok(());
    }
    // New correction UUIDs must not reuse preserved domain identities. Stable
    // IDs may recur as references, never as new records or version IDs.
    let mut ids = s
        .inclusion_links
        .iter()
        .map(|r| r.id.as_str())
        .chain(s.inclusion_link_versions.iter().map(|r| r.id.as_str()))
        .chain(s.inclusion_reservations.iter().map(|r| r.id.as_str()))
        .collect::<BTreeSet<_>>();
    ids.extend(s.business_records.iter().map(|r| r.id.as_str()));
    ids.extend(s.revisions.iter().map(|r| r.id.as_str()));
    ids.extend(s.confirmations.iter().map(|r| r.id.as_str()));
    ids.extend(s.materials.iter().map(|r| r.id.as_str()));
    ids.extend(s.prices.iter().map(|r| r.id.as_str()));
    ids.extend(
        s.orders
            .iter()
            .filter(|r| r.correction_projection_id.is_none())
            .map(|r| r.id.as_str()),
    );
    ids.extend(s.receipts.iter().map(|r| r.id.as_str()));
    ids.extend(s.productions.iter().map(|r| r.id.as_str()));
    ids.extend(s.sales.iter().map(|r| r.id.as_str()));
    ids.extend(
        s.sale_versions
            .iter()
            .filter(|r| {
                !s.event_versions
                    .iter()
                    .any(|e| e.sale_version_id.as_ref() == Some(&r.id))
            })
            .map(|r| r.id.as_str()),
    );
    ids.extend(s.stocktake_batches.iter().map(|r| r.id.as_str()));
    ids.extend(s.observations.iter().map(|r| r.id.as_str()));
    ids.extend(
        s.observation_versions
            .iter()
            .filter(|r| r.correction_projection_id.is_none())
            .map(|r| r.id.as_str()),
    );
    ids.extend(
        s.consumption_versions
            .iter()
            .filter(|r| r.correction_projection_id.is_none())
            .map(|r| r.id.as_str()),
    );
    ids.extend(
        s.holdings
            .iter()
            .filter(|r| r.correction_projection_id.is_none())
            .map(|r| r.id.as_str()),
    );
    ids.extend(s.wastes.iter().map(|r| r.id.as_str()));
    ids.extend(s.purchase_returns.iter().map(|r| r.id.as_str()));
    for r in s
        .orders
        .iter()
        .filter(|r| r.correction_projection_id.is_some())
    {
        new_id(&mut ids, &r.id)?;
    }
    for r in s.sale_versions.iter().filter(|r| {
        s.event_versions
            .iter()
            .any(|e| e.sale_version_id.as_ref() == Some(&r.id))
    }) {
        new_id(&mut ids, &r.id)?;
    }
    for r in s
        .observation_versions
        .iter()
        .filter(|r| r.correction_projection_id.is_some())
    {
        new_id(&mut ids, &r.id)?;
    }
    for r in s
        .consumption_versions
        .iter()
        .filter(|r| r.correction_projection_id.is_some())
    {
        new_id(&mut ids, &r.id)?;
    }
    for r in s
        .holdings
        .iter()
        .filter(|r| r.correction_projection_id.is_some())
    {
        new_id(&mut ids, &r.id)?;
    }
    for r in s.records {
        new_id(&mut ids, &r.id)?;
    }
    for v in s.event_versions {
        new_id(&mut ids, &v.id)?;
    }
    for v in s.projection_versions {
        new_id(&mut ids, &v.id)?;
    }
    for p in s.price_bases {
        new_id(&mut ids, &p.id)?;
    }
    let originals = s
        .inclusion_links
        .iter()
        .map(|l| l.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut links = BTreeSet::new();
    for v in s.projection_versions {
        if let Projection::Inclusion { links: rows, .. } = &v.projection {
            for row in rows {
                new_id(&mut ids, &row.id)?;
                new_id(&mut ids, &row.reservation_id)?;
                identifier(&row.link_id)?;
                if links.insert(row.link_id.as_str()) && !originals.contains(row.link_id.as_str()) {
                    new_id(&mut ids, &row.link_id)?;
                }
            }
        }
    }
    Ok(())
}

fn validate_event_versions(
    s: &Validation<'_>,
    r: &CorrectionRecord,
    history: &CorrectionHistory,
) -> Result<(), StoreError> {
    let rows = s
        .event_versions
        .iter()
        .filter(|v| v.correction_id == r.id)
        .collect::<Vec<_>>();
    if rows.len() != r.plan.events.len() || rows.is_empty() {
        return Err(bad("정정 사건 개정과 계산 계획의 행 수가 다릅니다"));
    }
    let mut seen = BTreeSet::new();
    for row in rows {
        let reference = &row.reference;
        let key = (kind_text(reference.kind), reference.original_id.as_str());
        if !seen.insert(key) {
            return Err(bad("같은 원사건의 정정 개정이 중복됐습니다"));
        }
        let planned = r
            .plan
            .events
            .iter()
            .find(|p| {
                p.reference.kind == reference.kind
                    && p.reference.original_id == reference.original_id
            })
            .ok_or_else(|| bad("정정 계획에 없는 사건 개정입니다"))?;
        let old = history
            .events
            .iter()
            .find(|e| {
                e.reference.kind == reference.kind
                    && e.reference.original_id == reference.original_id
            })
            .ok_or_else(|| bad("정정 원사건이 없습니다"))?;
        for id in [
            &reference.original_id,
            &reference.original_confirmation_id,
            &reference.confirmation_id,
            &reference.version_id,
            &reference.business_record_id,
        ] {
            identifier(id)?;
        }
        if row.id != reference.version_id
            || reference.confirmation_id != r.confirmation_id
            || reference.business_record_id != r.business_record_id
            || reference.seq != r.seq
            || reference.recorded_at != r.recorded_at
            || reference.original_confirmation_id != planned.reference.original_confirmation_id
            || reference.logical_seq != planned.reference.logical_seq
            || reference.logical_position != planned.reference.logical_position
            || reference.original_recorded_at != planned.reference.original_recorded_at
            || !equal(&row.after, &planned.after)?
            || !equal(&planned.reference, &old.reference)?
        {
            return Err(bad(
                "정정 사건의 원래 위치·새 기록 시점·효과가 일치하지 않습니다",
            ));
        }
        let earlier = s
            .event_versions
            .iter()
            .filter(|v| {
                v.reference.kind == reference.kind
                    && v.reference.original_id == reference.original_id
                    && v.reference
                        .seq
                        .parse::<i64>()
                        .is_ok_and(|n| n < r.seq.parse::<i64>().unwrap_or(0))
            })
            .max_by_key(|v| v.reference.seq.parse::<i64>().unwrap_or(0));
        match earlier {
            Some(previous)
                if row.previous_version_id.as_deref() == Some(previous.id.as_str())
                    && row.previous_original_version_id.is_none() => {}
            None if row.previous_version_id.is_none()
                && row.previous_original_version_id.as_deref()
                    == Some(old.reference.version_id.as_str()) => {}
            _ => return Err(bad("정정 사건 개정의 직전 연결이 올바르지 않습니다")),
        }
        match (&row.after, &row.sale_version_id) {
            (Some(CorrectionEventValue::SaleCount { input, result }), Some(id)) => {
                identifier(id)?;
                let actual = s
                    .sale_versions
                    .iter()
                    .find(|v| v.id == *id)
                    .ok_or_else(|| bad("정정 판매의 실제 개정이 없습니다"))?;
                let previous = s
                    .sale_versions
                    .iter()
                    .filter(|v| {
                        v.sale_id == reference.original_id
                            && v.seq
                                .parse::<i64>()
                                .is_ok_and(|n| n < r.seq.parse::<i64>().unwrap_or(0))
                    })
                    .max_by_key(|v| v.seq.parse::<i64>().unwrap_or(0))
                    .ok_or_else(|| bad("정정 판매의 직전 개정이 없습니다"))?;
                if actual.id == row.id
                    || actual.sale_id != reference.original_id
                    || actual.previous_version_id.as_deref() != Some(previous.id.as_str())
                    || actual.confirmation_id != r.confirmation_id
                    || actual.business_record_id != r.business_record_id
                    || actual.seq != r.seq
                    || actual.recorded_at != r.recorded_at
                    || actual.count != input.count
                    || actual.count != result.count
                    || actual.amount != result.amount
                    || actual.note != r.reason
                {
                    return Err(bad(
                        "정정 판매 실제 버전의 소속·개수·금액·직전 연결이 다릅니다",
                    ));
                }
            }
            (Some(CorrectionEventValue::SaleCount { .. }), None) | (_, Some(_)) => {
                return Err(bad("정정 사건과 실제 판매 버전의 종류가 다릅니다"));
            }
            (_, None) => {}
        }
    }
    if !seen
        .iter()
        .any(|(kind, id)| *kind == kind_text(r.plan.target.kind) && *id == r.original_id)
    {
        return Err(bad("정정 계획에 주 대상 사건이 없습니다"));
    }
    Ok(())
}

fn validate_heads(s: &Validation<'_>) -> Result<(), StoreError> {
    let mut events = BTreeMap::<(&str, &str), &EventVersionRow>::new();
    for row in s.event_versions {
        if !s.records.iter().any(|r| r.id == row.correction_id) {
            return Err(bad("정정 사건의 소유 기록이 없습니다"));
        }
        let key = (
            kind_text(row.reference.kind),
            row.reference.original_id.as_str(),
        );
        let seq = number(&row.reference.seq, s.sequence)?;
        if events
            .get(&key)
            .is_none_or(|old| old.reference.seq.parse::<i64>().unwrap_or(0) < seq)
        {
            events.insert(key, row);
        }
    }
    if events.len() != s.event_heads.len() {
        return Err(bad("정정 사건 head 수가 다릅니다"));
    }
    let mut seen = BTreeSet::new();
    for head in s.event_heads {
        let key = (kind_text(head.kind), head.original_id.as_str());
        if !seen.insert(key)
            || events
                .get(&key)
                .is_none_or(|v| v.id != head.current_version_id)
        {
            return Err(bad("정정 사건 head가 최신 유효 버전을 가리키지 않습니다"));
        }
    }
    let mut projections = BTreeMap::<(&str, &str), &ProjectionVersionRow>::new();
    let mut ordered = s.projection_versions.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|v| v.seq.parse::<i64>().unwrap_or(0));
    for row in ordered {
        let parent = s
            .records
            .iter()
            .find(|r| r.id == row.correction_id)
            .ok_or_else(|| bad("정정 투영의 소유 기록이 없습니다"))?;
        let key = (row.projection.kind(), row.projection.subject_id());
        identifier(key.1)?;
        number(&row.seq, s.sequence)?;
        if row.seq != parent.seq
            || row.recorded_at != parent.recorded_at
            || row.previous_version_id.as_deref() != projections.get(&key).map(|p| p.id.as_str())
            || projections.get(&key).is_some_and(|p| p.seq == row.seq)
        {
            return Err(bad("정정 투영의 소유·시점·직전 연결이 일치하지 않습니다"));
        }
        projections.insert(key, row);
    }
    if projections.len() != s.projection_heads.len() {
        return Err(bad("정정 투영 head 수가 다릅니다"));
    }
    let mut seen = BTreeSet::new();
    for head in s.projection_heads {
        let key = (head.kind.text(), head.subject_id.as_str());
        if !seen.insert(key)
            || projections
                .get(&key)
                .is_none_or(|v| v.id != head.current_version_id)
        {
            return Err(bad("정정 투영 head가 최신 유효 버전을 가리키지 않습니다"));
        }
    }
    for price in s.price_bases {
        let record = s
            .records
            .iter()
            .find(|r| r.id == price.correction_id)
            .ok_or_else(|| bad("정정 단가의 소유 기록이 없습니다"))?;
        if price.seq != record.seq || price.recorded_at != record.recorded_at {
            return Err(bad("정정 단가의 기록 시점이 다릅니다"));
        }
        if !s.projection_versions.iter().any(|p| {
            p.correction_id == price.correction_id
                && matches!(&p.projection,
            Projection::Material { price_basis_id:Some(id), .. }
            | Projection::Observation { price_basis_id:Some(id), .. } if id == &price.id)
        }) {
            return Err(bad("사용하지 않는 정정 단가 근거가 있습니다"));
        }
    }
    Ok(())
}

pub(in crate::store) fn validate_snapshot(s: Validation<'_>) -> Result<(), StoreError> {
    validate_identities(&s)?;
    validate_heads(&s)?;
    let mut confirmations = BTreeSet::new();
    let mut sequences = BTreeSet::new();
    let mut previous = BTreeMap::<&str, &CorrectionRecord>::new();
    let mut records = s.records.iter().collect::<Vec<_>>();
    records.sort_by_key(|r| r.seq.parse::<i64>().unwrap_or(0));
    for record in records {
        let sequence = number(&record.seq, s.sequence)?;
        time(&record.recorded_at)?;
        for value in [
            &record.original_id,
            &record.target_confirmation_id,
            &record.confirmation_id,
            &record.business_record_id,
        ] {
            identifier(value)?;
        }
        if !confirmations.insert(record.confirmation_id.as_str())
            || !sequences.insert(sequence)
            || record.previous_version_id.as_deref()
                != previous
                    .get(record.original_id.as_str())
                    .map(|r| r.id.as_str())
        {
            return Err(bad("정정 원본·확정·직전 기록 연결이 일치하지 않습니다"));
        }
        let confirmation = s
            .confirmations
            .iter()
            .find(|c| c.id == record.confirmation_id)
            .ok_or_else(|| bad("정정 확정이 없습니다"))?;
        let business = s
            .business_records
            .iter()
            .find(|b| b.id == record.business_record_id)
            .ok_or_else(|| bad("정정 업무가 없습니다"))?;
        let revision = s
            .revisions
            .iter()
            .find(|v| v.id == confirmation.revision_id)
            .ok_or_else(|| bad("정정 확정 입력이 없습니다"))?;
        let BusinessPayload::Correction { version: 1, data } = &revision.payload else {
            return Err(bad("정정 확정의 입력 종류가 다릅니다"));
        };
        if confirmation.record_id != record.business_record_id
            || confirmation.seq != record.seq
            || confirmation.recorded_at != record.recorded_at
            || revision.record_id != business.id
            || revision.status != BusinessStatus::Confirmed
            || revision.seq != record.seq
            || revision.recorded_at != record.recorded_at
            || !equal(
                &crate::business::normalize_business_payload(revision.payload.clone())
                    .map_err(unavailable)?,
                &revision.payload,
            )?
            || business.kind != BusinessKind::Correction
            || business.corrects_confirmation_id.as_deref()
                != Some(record.target_confirmation_id.as_str())
            || record.reason != record.input.reason
            || !equal(data, &record.input)?
            || record.plan.target.original_id != record.original_id
            || record.plan.target.original_confirmation_id != record.target_confirmation_id
            || record.input.expected_target_version_id.as_deref()
                != Some(record.plan.target.version_id.as_str())
            || s.mutations
                .iter()
                .filter(|(seq, operation, request)| {
                    *seq == record.seq
                        && *operation == "business.confirm"
                        && *request == confirmation.request_id
                })
                .count()
                != 1
        {
            return Err(bad(
                "정정 입력·확정·계산 근거·요청 연결이 일치하지 않습니다",
            ));
        }
        identifier(&confirmation.request_id)?;
        // The original machine's local calendar is not inferred from UTC or
        // today's machine. Runtime confirmation checked future receipt dates;
        // historical replay checks calendar syntax and all persisted maths.
        let history = history::from_snapshot(&s, sequence - 1, NaiveDate::MAX)?;
        let expected =
            build_correction_plan(&history, &record.target_confirmation_id, &record.input)
                .map_err(unavailable)?;
        if !expected.issues.is_empty()
            || !expected.plan.review_requests.is_empty()
            || !equal(&expected.plan, &record.plan)?
        {
            return Err(bad(
                "저장된 정정 계획을 당시 유효 이력에서 재현할 수 없습니다",
            ));
        }
        validate_event_versions(&s, record, &history)?;
        projections::validate_projections(&s, record)?;
        previous.insert(&record.original_id, record);
    }
    for confirmation in s.confirmations {
        if s.business_records
            .iter()
            .any(|b| b.id == confirmation.record_id && b.kind == BusinessKind::Correction)
            && !confirmations.contains(confirmation.id.as_str())
        {
            return Err(bad("정정 확정에 대응하는 원장 기록이 없습니다"));
        }
    }
    if !s.records.is_empty() {
        let history = history::from_snapshot(&s, s.sequence, NaiveDate::MAX)?;
        effective_correction_replay(&history).map_err(unavailable)?;
    }
    Ok(())
}
