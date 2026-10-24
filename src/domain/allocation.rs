use super::DomainError;

/// Allocate an exact nonnegative total without losing its smallest units.
/// The caller orders weights by the original sequence, row position and ID;
/// equal remainders retain that order. This works for both money and quantity.
pub fn allocate_largest_remainder(total: i64, weights: &[i64]) -> Result<Vec<i64>, DomainError> {
    let invalid = || DomainError("배분 총량과 가중치를 확인해 주세요".into());
    if total < 0 || weights.iter().any(|weight| *weight < 0) {
        return Err(invalid());
    }
    let denominator = weights.iter().try_fold(0_i128, |sum, weight| {
        sum.checked_add(i128::from(*weight)).ok_or_else(invalid)
    })?;
    if denominator <= 0 {
        return Err(invalid());
    }
    let mut shares = Vec::with_capacity(weights.len());
    let mut remainders = Vec::with_capacity(weights.len());
    let mut assigned = 0_i128;
    for (position, weight) in weights.iter().enumerate() {
        let numerator = i128::from(total)
            .checked_mul(i128::from(*weight))
            .ok_or_else(invalid)?;
        let floor = numerator / denominator;
        assigned = assigned.checked_add(floor).ok_or_else(invalid)?;
        shares.push(i64::try_from(floor).map_err(|_| invalid())?);
        remainders.push((position, numerator % denominator));
    }
    let remaining = usize::try_from(i128::from(total) - assigned).map_err(|_| invalid())?;
    if remaining > shares.len() {
        return Err(invalid());
    }
    remainders.sort_by(|(left_position, left), (right_position, right)| {
        right.cmp(left).then(left_position.cmp(right_position))
    });
    for &(position, _) in remainders.iter().take(remaining) {
        shares[position] = shares[position].checked_add(1).ok_or_else(invalid)?;
    }
    Ok(shares)
}
