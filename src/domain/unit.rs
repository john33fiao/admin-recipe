use super::{DomainError, Quantity};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unit {
    canonical: String,
    dimension: &'static str,
    multiplier: i64,
}

impl Unit {
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        let text = text.trim();
        if text.is_empty() || text.chars().count() > 20 || text.chars().any(char::is_control) {
            return Err(DomainError(
                "관리 단위는 제어문자 없이 1~20자로 입력하세요".into(),
            ));
        }
        let (canonical, dimension, multiplier) = match text {
            "g" => ("g", "mass", 1),
            "kg" => ("g", "mass", 1000),
            "mL" => ("mL", "volume", 1),
            "L" => ("mL", "volume", 1000),
            custom => (custom, "custom", 1),
        };
        Ok(Self {
            canonical: canonical.into(),
            dimension,
            multiplier,
        })
    }

    pub fn canonical(&self) -> &str {
        &self.canonical
    }
    pub fn dimension(&self) -> &str {
        self.dimension
    }
    pub fn same_dimension(&self, other: &Self) -> bool {
        self.dimension == other.dimension && self.canonical == other.canonical
    }
    pub fn to_canonical(&self, quantity: Quantity) -> Result<Quantity, DomainError> {
        let micros = quantity
            .micros()
            .checked_mul(self.multiplier)
            .ok_or_else(|| DomainError("단위 환산 결과가 허용 범위를 넘습니다".into()))?;
        Quantity::from_micros(micros)
    }
}
