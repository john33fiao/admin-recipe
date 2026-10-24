use sunchang_hansang::domain::allocate_largest_remainder;

#[test]
fn production_percentages_preserve_total_value_including_zero_value_outputs() {
    assert_eq!(
        allocate_largest_remainder(30_000_000_000, &[100_000_000, 0]).unwrap(),
        [30_000_000_000, 0]
    );
    assert_eq!(
        allocate_largest_remainder(30_000_000_000, &[70_000_000, 30_000_000]).unwrap(),
        [21_000_000_000, 9_000_000_000]
    );
    assert_eq!(allocate_largest_remainder(0, &[100, 0]).unwrap(), [0, 0]);
}

#[test]
fn smallest_units_follow_remainders_then_original_row_order() {
    assert_eq!(
        allocate_largest_remainder(1, &[1, 1, 1]).unwrap(),
        [1, 0, 0]
    );
    assert_eq!(
        allocate_largest_remainder(2, &[1, 1, 1]).unwrap(),
        [1, 1, 0]
    );
    assert_eq!(
        allocate_largest_remainder(5, &[1, 2, 3]).unwrap(),
        [1, 2, 2]
    );
    assert_eq!(
        allocate_largest_remainder(5, &[3, 2, 1]).unwrap(),
        [2, 2, 1]
    );
    assert_eq!(
        allocate_largest_remainder(5, &[0, 0, 9]).unwrap(),
        [0, 0, 5]
    );
}

#[test]
fn repeated_material_rows_share_the_single_material_cost_without_remainder_loss() {
    assert_eq!(
        allocate_largest_remainder(10_000_000_001, &[1_000_000_000, 2_000_000_000]).unwrap(),
        [3_333_333_334, 6_666_666_667]
    );
    assert_eq!(
        allocate_largest_remainder(9_000_000_000_000_000_000, &[1, 2]).unwrap(),
        [3_000_000_000_000_000_000, 6_000_000_000_000_000_000]
    );
}

#[test]
fn wide_intermediates_allow_weights_whose_sum_exceeds_i64() {
    assert_eq!(
        allocate_largest_remainder(i64::MAX, &[i64::MAX, i64::MAX]).unwrap(),
        [4_611_686_018_427_387_904, 4_611_686_018_427_387_903]
    );
    assert_eq!(
        allocate_largest_remainder(i64::MAX, &[i64::MAX; 3]).unwrap(),
        [
            3_074_457_345_618_258_603,
            3_074_457_345_618_258_602,
            3_074_457_345_618_258_602
        ]
    );
}

#[test]
fn invalid_totals_and_nonpositive_total_weight_have_no_allocation() {
    for (total, weights) in [
        (-1, vec![1]),
        (1, vec![]),
        (0, vec![]),
        (0, vec![0, 0]),
        (1, vec![0, 0]),
        (1, vec![1, -1]),
    ] {
        assert!(allocate_largest_remainder(total, &weights).is_err());
    }
}

#[test]
fn shares_obey_exact_quota_bounds_and_do_not_mutate_inputs() {
    for weights in [
        vec![1, 1, 1],
        vec![0, 7, 3],
        vec![2, 5, 17, 0, 4],
        vec![i64::MAX, 1],
    ] {
        let original = weights.clone();
        let sum: i128 = weights.iter().map(|weight| i128::from(*weight)).sum();
        for total in [0, 1, 2, 11, 1_000_001, i64::MAX] {
            let allocated = allocate_largest_remainder(total, &weights).unwrap();
            assert_eq!(allocated.len(), weights.len());
            assert_eq!(
                allocated
                    .iter()
                    .map(|value| i128::from(*value))
                    .sum::<i128>(),
                i128::from(total)
            );
            for (&share, &weight) in allocated.iter().zip(&weights) {
                let exact_numerator = i128::from(total) * i128::from(weight);
                let lower = exact_numerator / sum;
                let upper = lower + i128::from(exact_numerator % sum != 0);
                assert!((lower..=upper).contains(&i128::from(share)));
            }
            assert_eq!(weights, original);
        }
    }
}
