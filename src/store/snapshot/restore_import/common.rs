use crate::{
    domain::{Money, Quantity},
    store::StoreError,
};
use serde::Serialize;

pub(super) fn bad(error: impl std::fmt::Display) -> StoreError {
    StoreError::Validation(format!("복원 행을 확인할 수 없습니다: {error}"))
}
pub(super) fn seq(value: &str) -> Result<i64, StoreError> {
    let n = value.parse::<i64>().map_err(bad)?;
    if n <= 0 || n.to_string() != value {
        return Err(bad("순번 오류"));
    }
    Ok(n)
}
pub(super) fn nonnegative(value: &str) -> Result<i64, StoreError> {
    let n = value.parse::<i64>().map_err(bad)?;
    if n < 0 || n.to_string() != value {
        return Err(bad("정수 오류"));
    }
    Ok(n)
}
pub(super) fn q(value: &str) -> Result<i64, StoreError> {
    let n = Quantity::parse(value).map_err(bad)?;
    if n.decimal() != value {
        return Err(bad("수량이 정규 숫자가 아닙니다"));
    }
    Ok(n.micros())
}
pub(super) fn money(value: &str) -> Result<i64, StoreError> {
    let n = Money::parse(value).map_err(bad)?;
    if n.decimal() != value {
        return Err(bad("금액이 정규 숫자가 아닙니다"));
    }
    Ok(n.micros())
}
pub(super) fn tag(value: &impl Serialize) -> Result<String, StoreError> {
    serde_json::to_value(value)
        .map_err(bad)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| bad("태그 오류"))
}
pub(super) fn json(value: &impl Serialize) -> Result<String, StoreError> {
    serde_json::to_string(value).map_err(bad)
}
