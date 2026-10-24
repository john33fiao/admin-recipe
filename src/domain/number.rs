use super::DomainError;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};

const SCALE: i64 = 1_000_000;

fn parse_decimal(text: &str, maximum: i64) -> Result<i64, DomainError> {
    let text = text.trim();
    let (whole, fraction) = match text.split_once('.') {
        Some((whole, fraction)) if !fraction.is_empty() => (whole, fraction),
        Some(_) => return Err(DomainError("소수점 뒤의 숫자를 입력하세요".into())),
        None => (text, ""),
    };
    if whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || fraction.len() > 6
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(DomainError(
            "0 이상의 십진수와 소수 최대 6자리로 입력하세요".into(),
        ));
    }
    let whole: i64 = whole
        .parse()
        .map_err(|_| DomainError("입력한 수가 허용 범위를 넘습니다".into()))?;
    let fraction: i64 = if fraction.is_empty() {
        0
    } else {
        fraction.parse::<i64>().unwrap() * 10_i64.pow(6 - fraction.len() as u32)
    };
    let micros = whole
        .checked_mul(SCALE)
        .and_then(|value| value.checked_add(fraction))
        .filter(|value| *value <= maximum)
        .ok_or_else(|| DomainError("입력한 수가 허용 범위를 넘습니다".into()))?;
    Ok(micros)
}

fn decimal(micros: i64) -> String {
    let whole = micros / SCALE;
    let fraction = micros % SCALE;
    if fraction == 0 {
        whole.to_string()
    } else {
        format!("{whole}.{}", format!("{fraction:06}").trim_end_matches('0'))
    }
}

macro_rules! fixed_decimal {
    ($name:ident, $maximum:expr) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $name(i64);

        impl $name {
            pub fn parse(text: &str) -> Result<Self, DomainError> {
                parse_decimal(text, $maximum).map(Self)
            }
            pub fn from_micros(value: i64) -> Result<Self, DomainError> {
                if !(0..=$maximum).contains(&value) {
                    return Err(DomainError("계산 결과가 허용 범위를 벗어났습니다".into()));
                }
                Ok(Self(value))
            }
            pub fn micros(self) -> i64 {
                self.0
            }
            pub fn decimal(self) -> String {
                decimal(self.0)
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(&self.decimal())
            }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let value = String::deserialize(deserializer)?;
                Self::parse(&value).map_err(D::Error::custom)
            }
        }
    };
}

fixed_decimal!(Quantity, 9_000_000_000_000_000);
fixed_decimal!(Money, 9_000_000_000_000_000_000);

/// Round an exact ratio once, with ties away from zero. Intermediate overflow
/// and invalid denominators are errors rather than saturation or float fallback.
pub fn round_ratio(numerator: i128, denominator: i128) -> Result<i128, DomainError> {
    if denominator <= 0 {
        return Err(DomainError("계산의 기준 수량은 0보다 커야 합니다".into()));
    }
    let magnitude = numerator
        .checked_abs()
        .ok_or_else(|| DomainError("계산 중간값이 허용 범위를 넘습니다".into()))?;
    let quotient = magnitude / denominator;
    let remainder = magnitude % denominator;
    // Avoid doubling a remainder near i128::MAX.
    let threshold = denominator / 2 + denominator % 2;
    let rounded = if remainder >= threshold {
        quotient
            .checked_add(1)
            .ok_or_else(|| DomainError("반올림 결과가 허용 범위를 넘습니다".into()))?
    } else {
        quotient
    };
    Ok(if numerator < 0 { -rounded } else { rounded })
}
