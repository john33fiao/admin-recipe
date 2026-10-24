//! Pure numeric contracts. No database, HTTP, or file access belongs here.
mod number;
mod price;
mod unit;

pub use number::{Money, Quantity, round_ratio};
pub use price::{HoldingBasis, ResolvedPrice, UnitPrice, resolve_price};
pub use unit::Unit;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DomainError(pub String);

impl std::fmt::Display for DomainError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for DomainError {}
