use super::{DomainError, Money, Quantity};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnitPrice {
    amount: Money,
    quantity: Quantity,
    source_id: String,
}

impl UnitPrice {
    pub fn new(amount: Money, quantity: Quantity, source_id: String) -> Result<Self, DomainError> {
        if quantity.micros() == 0 {
            return Err(DomainError("가격의 기준 수량은 0보다 커야 합니다".into()));
        }
        Ok(Self {
            amount,
            quantity,
            source_id,
        })
    }
    pub fn amount(&self) -> Money {
        self.amount
    }
    pub fn quantity(&self) -> Quantity {
        self.quantity
    }
    pub fn source_id(&self) -> &str {
        &self.source_id
    }
}

#[derive(Clone, Debug)]
pub enum HoldingBasis {
    NotEstablished,
    Known {
        quantity: Quantity,
        value: Money,
        source_id: String,
    },
    ValueUnknown {
        quantity: Quantity,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedPrice {
    pub source: String,
    pub source_id: Option<String>,
    pub amount: Option<String>,
    pub quantity: Option<String>,
    pub reason: Option<String>,
}

impl ResolvedPrice {
    fn known(source: &str, price: UnitPrice) -> Self {
        Self {
            source: source.into(),
            source_id: Some(price.source_id),
            amount: Some(price.amount.decimal()),
            quantity: Some(price.quantity.decimal()),
            reason: None,
        }
    }
    fn missing(reason: &str) -> Self {
        Self {
            source: "missing".into(),
            source_id: None,
            amount: None,
            quantity: None,
            reason: Some(reason.into()),
        }
    }
}

pub fn resolve_price(
    holding: HoldingBasis,
    last_valid: Option<UnitPrice>,
    preregistered: Option<UnitPrice>,
) -> ResolvedPrice {
    match holding {
        HoldingBasis::Known {
            quantity,
            value,
            source_id,
        } if quantity.micros() > 0 => {
            return ResolvedPrice::known(
                "holding_average",
                UnitPrice {
                    amount: value,
                    quantity,
                    source_id,
                },
            );
        }
        HoldingBasis::Known { value, .. } if value.micros() != 0 => {
            return ResolvedPrice::missing("보유 수량과 가치의 근거를 확인해 주세요");
        }
        HoldingBasis::ValueUnknown { quantity } if quantity.micros() > 0 => {
            return ResolvedPrice::missing("보유 수량의 가치를 먼저 확인해 주세요");
        }
        HoldingBasis::NotEstablished => {
            return preregistered
                .map(|price| ResolvedPrice::known("preregistered", price))
                .unwrap_or_else(|| ResolvedPrice::missing("등록한 가격이 없습니다"));
        }
        _ => {}
    }
    last_valid
        .map(|price| ResolvedPrice::known("last_valid", price))
        .or_else(|| preregistered.map(|price| ResolvedPrice::known("preregistered", price)))
        .unwrap_or_else(|| ResolvedPrice::missing("사용할 수 있는 가격 이력이 없습니다"))
}
