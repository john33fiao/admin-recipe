#![recursion_limit = "256"]

use chrono::NaiveDate;
use serde_json::json;
use sunchang_hansang::{
    catalog::MaterialVersion,
    correction::*,
    holding::{HoldingAmounts, HoldingBasisKind, HoldingBasisRef},
    receipt::{DraftReceiptV1, ReceiptSourceDraft, ReceiptStockEffectDraft},
    stocktake::{
        ConsumptionAllocation, NoAllocationReason, StocktakeCalculation,
        StocktakeObservationRecord, StocktakeRowEffect,
    },
};

fn id(n: u128) -> String {
    uuid::Uuid::from_u128(n).to_string()
}
const TIME: &str = "2026-01-01T00:00:00Z";
fn reference(kind: CorrectionSubjectKind, n: u128, seq: i64) -> CorrectionEventRef {
    CorrectionEventRef {
        kind,
        original_id: id(n),
        original_confirmation_id: id(n + 100),
        confirmation_id: id(n + 100),
        version_id: id(n),
        business_record_id: id(n + 200),
        logical_seq: seq.to_string(),
        logical_position: 0,
        original_recorded_at: TIME.into(),
        recorded_at: TIME.into(),
        seq: seq.to_string(),
    }
}
fn receipt(quantity: &str, amount: &str) -> DraftReceiptV1 {
    DraftReceiptV1 {
        source: ReceiptSourceDraft::Direct {
            material_id: Some(id(1)),
            material_version_id: Some(id(2)),
        },
        quantity: quantity.into(),
        unit: "g".into(),
        amount: amount.into(),
        receipt_date: "2020-02-29".into(),
        note: String::new(),
        stock_effect: ReceiptStockEffectDraft::Unreflected {},
    }
}
fn price(
    amount: &str,
    quantity: &str,
    entity: CorrectionRefKind,
    n: u128,
    seq: i64,
) -> CorrectionPriceBasis {
    CorrectionPriceBasis {
        amount: amount.into(),
        quantity: quantity.into(),
        source: CorrectionPlanRef::Existing {
            entity,
            id: id(n),
            version_id: id(n),
        },
        logical_seq: seq.to_string(),
    }
}
fn initial_history() -> CorrectionHistory {
    let input = CorrectionReplacementDraft::Receipt {
        data: receipt("10", "100"),
    };
    CorrectionHistory {
        as_of_revision: 4,
        today: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        events: vec![CorrectionHistoryEvent {
            reference: reference(CorrectionSubjectKind::Receipt, 10, 2),
            original_input: input.clone(),
            effective_input: Some(input),
            effective_value: None,
            observations: vec![],
        }],
        materials: vec![MaterialVersion {
            id: id(2),
            material_id: id(1),
            name: "가상 정정 품목".into(),
            unit: "g".into(),
            canonical_unit: "g".into(),
            dimension: "mass".into(),
            kind: "raw".into(),
            description: String::new(),
            archived: false,
            recorded_at: TIME.into(),
            seq: Some("1".into()),
            source: "catalog".into(),
        }],
        prices: vec![],
        orders: vec![],
        closures: vec![],
        sales: vec![],
        sale_versions: vec![],
        existing_inclusions: vec![],
        actual_materials: vec![CorrectionMaterialSnapshot {
            material_id: id(1),
            state: CorrectionHoldingState::Known {
                amounts: HoldingAmounts {
                    quantity: "10".into(),
                    value: Some("100".into()),
                    source_id: Some(id(999)),
                },
                projection_seq: Some("2".into()),
                last_valid_price: Some(price("100", "10", CorrectionRefKind::Receipt, 10, 2)),
                observation_boundary: None,
                initial_boundary: Some(CorrectionInitialBoundary {
                    reference: CorrectionPlanRef::Existing {
                        entity: CorrectionRefKind::Receipt,
                        id: id(10),
                        version_id: id(10),
                    },
                    logical_seq: "2".into(),
                }),
            },
        }],
        actual_allocations: vec![],
    }
}
fn draft(action: CorrectionActionDraft) -> DraftCorrectionV1 {
    DraftCorrectionV1 {
        expected_target_version_id: Some(id(10)),
        reason: "가상 오입력 수정".into(),
        action,
        observation_reviews: vec![],
        linked_receipt_changes: vec![],
    }
}
fn with_observation(mut h: CorrectionHistory) -> CorrectionHistory {
    let basis = HoldingBasisRef {
        kind: HoldingBasisKind::Receipt,
        id: Some(id(10)),
        seq: Some("2".into()),
    };
    let allocation = ConsumptionAllocation {
        basis: basis.clone(),
        end_seq: "4".into(),
        as_of_revision: "4".into(),
        total_quantity: "4".into(),
        total_value: "40".into(),
        candidates: vec![],
        unallocated_quantity: "4".into(),
        unallocated_value: "40".into(),
        reason: Some(NoAllocationReason::NoSales),
    };
    let effect = StocktakeRowEffect {
        position: 0,
        material_id: id(1),
        material_version_id: id(2),
        material_name: "가상 정정 품목".into(),
        unit: "g".into(),
        input_quantity: "6".into(),
        input_unit: "g".into(),
        measured_quantity: "6".into(),
        before: Some(HoldingAmounts {
            quantity: "10".into(),
            value: Some("100".into()),
            source_id: Some(id(10)),
        }),
        after: HoldingAmounts {
            quantity: "6".into(),
            value: Some("60".into()),
            source_id: None,
        },
        basis_before: Some(basis),
        observation_before: None,
        calculation: StocktakeCalculation::Settlement {
            consumption_quantity: "4".into(),
            consumption_value: "40".into(),
            excess_quantity: "0".into(),
            excess_value: "0".into(),
            valuation: None,
            allocation: Box::new(allocation.clone()),
        },
    };
    let row = StocktakeObservationRecord {
        id: id(31),
        version_id: id(32),
        previous_version_id: None,
        stocktake_id: id(30),
        previous_observation_version_id: None,
        consumption_version_id: Some(id(33)),
        effect,
        recorded_at: TIME.into(),
        seq: "4".into(),
    };
    let input = CorrectionReplacementDraft::Stocktake {
        data: DraftStocktakeCorrectionV1 {
            rows: vec![StocktakeCorrectionRowDraft {
                observation_id: Some(id(31)),
                expected_version_id: Some(id(32)),
                action: ObservationCorrectionActionDraft::Replace {
                    quantity: "6".into(),
                    unit: "g".into(),
                    initial_value: None,
                    excess: None,
                },
            }],
        },
    };
    h.events.push(CorrectionHistoryEvent {
        reference: reference(CorrectionSubjectKind::Stocktake, 30, 4),
        original_input: input.clone(),
        effective_input: Some(input),
        effective_value: None,
        observations: vec![row],
    });
    h.actual_allocations.push(CorrectionAllocationSnapshot {
        observation_id: id(31),
        consumption_version_id: id(33),
        allocation,
    });
    h.actual_materials[0].state = CorrectionHoldingState::Known {
        amounts: HoldingAmounts {
            quantity: "6".into(),
            value: Some("60".into()),
            source_id: Some(id(998)),
        },
        projection_seq: Some("4".into()),
        last_valid_price: Some(price("60", "6", CorrectionRefKind::Observation, 32, 4)),
        observation_boundary: Some(CorrectionObservationBoundary {
            reference: CorrectionPlanRef::Existing {
                entity: CorrectionRefKind::Observation,
                id: id(31),
                version_id: id(32),
            },
            stocktake_id: id(30),
            logical_seq: "4".into(),
            observed_at: TIME.into(),
            quantity: "6".into(),
        }),
        initial_boundary: Some(CorrectionInitialBoundary {
            reference: CorrectionPlanRef::Existing {
                entity: CorrectionRefKind::Receipt,
                id: id(10),
                version_id: id(10),
            },
            logical_seq: "2".into(),
        }),
    };
    h
}

#[test]
fn correction_input_is_closed_and_preserves_blank_values() {
    let value = json!({"expected_target_version_id":null,"reason":"\r\n","action":{"kind":"replace","replacement":{"kind":"sale_count","data":{"count":""}}},"observation_reviews":[],"linked_receipt_changes":[]});
    let mut draft: DraftCorrectionV1 = serde_json::from_value(value.clone()).unwrap();
    normalize_correction_draft(&mut draft).unwrap();
    assert_eq!(draft.reason, "\n");
    let mut bad = value;
    bad["action"] = json!({"kind":"void","quantity":"0"});
    assert!(serde_json::from_value::<DraftCorrectionV1>(bad).is_err());
    assert!(serde_json::from_value::<DraftCorrectionV1>(json!({"expected_target_version_id":null,"reason":"","action":{"kind":"replace","replacement":{"kind":"sale_count","data":{"count":0}}},"observation_reviews":[],"linked_receipt_changes":[]})).is_err());
}

#[test]
fn review_lists_are_not_truncated_or_capped_at_one_batch() {
    let mut d = draft(CorrectionActionDraft::Void {});
    d.observation_reviews = (0..121)
        .map(|_| ObservationReviewDraft {
            observation_id: None,
            expected_version_id: None,
            initial_value: None,
            excess: None,
        })
        .collect();
    d.linked_receipt_changes = (0..21)
        .map(|_| LinkedReceiptChangeDraft {
            receipt_id: None,
            expected_version_id: None,
            action: LinkedReceiptActionDraft::Void {},
        })
        .collect();
    normalize_correction_draft(&mut d).unwrap();
    assert_eq!(d.observation_reviews.len(), 121);
    assert_eq!(d.linked_receipt_changes.len(), 21);
}

#[test]
fn receipt_quantity_replacement_replays_original_position() {
    let h = initial_history();
    let check = build_correction_plan(
        &h,
        &id(110),
        &draft(CorrectionActionDraft::Replace {
            replacement: CorrectionReplacementDraft::Receipt {
                data: receipt("8", "100"),
            },
        }),
    )
    .unwrap();
    assert!(check.issues.is_empty(), "{:?}", check.issues);
    assert_eq!(check.plan.materials[0].quantity_delta, "-2");
    assert_eq!(check.plan.materials[0].value_delta, "0");
    assert_eq!(check.plan.target.logical_seq, "2");
    assert_eq!(h.events[0].reference.seq, "2");
}

#[test]
fn void_only_source_is_not_established_and_not_known_zero() {
    let check = build_correction_plan(
        &initial_history(),
        &id(110),
        &draft(CorrectionActionDraft::Void {}),
    )
    .unwrap();
    assert!(check.issues.is_empty());
    assert!(matches!(
        check.plan.materials[0].after,
        CorrectionHoldingState::NotEstablished {}
    ));
    assert_eq!(check.plan.materials[0].value_delta, "-100");
}

#[test]
fn measured_quantity_stays_fixed_and_consumption_value_is_recomputed() {
    let h = with_observation(initial_history());
    let check = build_correction_plan(
        &h,
        &id(110),
        &draft(CorrectionActionDraft::Replace {
            replacement: CorrectionReplacementDraft::Receipt {
                data: receipt("8", "100"),
            },
        }),
    )
    .unwrap();
    assert!(check.issues.is_empty(), "{:?}", check.issues);
    let effect = check.plan.observations[0].after.as_ref().unwrap();
    assert_eq!(effect.measured_quantity, "6");
    assert_eq!(effect.after.value.as_deref(), Some("75"));
    let StocktakeCalculation::Settlement {
        consumption_quantity,
        consumption_value,
        ..
    } = &effect.calculation
    else {
        panic!()
    };
    assert_eq!(consumption_quantity, "2");
    assert_eq!(consumption_value, "25");
    assert_eq!(
        check.plan.allocations[0]
            .after
            .as_ref()
            .unwrap()
            .total_value,
        "25"
    );
}

#[test]
fn void_source_keeps_later_measurement_and_requires_initial_value_review() {
    let check = build_correction_plan(
        &with_observation(initial_history()),
        &id(110),
        &draft(CorrectionActionDraft::Void {}),
    )
    .unwrap();
    assert!(!check.issues.is_empty());
    assert!(matches!(
        check.plan.review_requests.first(),
        Some(CorrectionReviewRequest::ObservationValuation {
            mode: CorrectionValuationMode::Initial,
            ..
        })
    ));
}

#[test]
fn stale_head_is_not_automatically_retargeted() {
    let mut d = draft(CorrectionActionDraft::Void {});
    d.expected_target_version_id = Some(id(777));
    let check = build_correction_plan(&initial_history(), &id(110), &d).unwrap();
    assert_eq!(check.issues[0].code, "stale_reference");
    assert!(check.plan.events.is_empty());
}

#[test]
fn base_mismatch_is_corruption_not_a_user_preview_issue() {
    let mut h = initial_history();
    if let CorrectionHoldingState::Known { amounts, .. } = &mut h.actual_materials[0].state {
        amounts.value = Some("99".into());
    }
    assert!(matches!(
        build_correction_plan(&h, &id(110), &draft(CorrectionActionDraft::Void {})),
        Err(CorrectionError::InvalidHistory(_))
    ));
}

#[test]
fn current_materialized_sequence_does_not_move_observation_boundary() {
    let mut h = with_observation(initial_history());
    h.as_of_revision = 30;
    if let CorrectionHoldingState::Known { projection_seq, .. } = &mut h.actual_materials[0].state {
        *projection_seq = Some("30".into());
    }
    let check = build_correction_plan(
        &h,
        &id(110),
        &draft(CorrectionActionDraft::Replace {
            replacement: CorrectionReplacementDraft::Receipt {
                data: receipt("8", "100"),
            },
        }),
    )
    .unwrap();
    assert!(check.issues.is_empty(), "{:?}", check.issues);
    let CorrectionHoldingState::Known {
        observation_boundary,
        ..
    } = &check.plan.materials[0].after
    else {
        panic!()
    };
    assert_eq!(observation_boundary.as_ref().unwrap().logical_seq, "4");
}

fn late_history(with_later_observation: bool) -> CorrectionHistory {
    let mut h = with_observation(initial_history());
    h.as_of_revision = 8;
    let mut first = h.events[1].clone();
    first.reference = reference(CorrectionSubjectKind::Stocktake, 40, 3);
    let row = &mut first.observations[0];
    row.id = id(41);
    row.version_id = id(42);
    row.stocktake_id = id(40);
    row.consumption_version_id = Some(id(43));
    row.seq = "3".into();
    row.effect.measured_quantity = "10".into();
    row.effect.input_quantity = "10".into();
    row.effect.after.quantity = "10".into();
    row.effect.after.value = Some("100".into());
    if let StocktakeCalculation::Settlement {
        consumption_quantity,
        consumption_value,
        allocation,
        ..
    } = &mut row.effect.calculation
    {
        *consumption_quantity = "0".into();
        *consumption_value = "0".into();
        allocation.end_seq = "3".into();
        allocation.total_quantity = "0".into();
        allocation.total_value = "0".into();
        allocation.unallocated_quantity = "0".into();
        allocation.unallocated_value = "0".into();
    }
    let input = CorrectionReplacementDraft::Stocktake {
        data: DraftStocktakeCorrectionV1 {
            rows: vec![StocktakeCorrectionRowDraft {
                observation_id: Some(id(41)),
                expected_version_id: Some(id(42)),
                action: ObservationCorrectionActionDraft::Replace {
                    quantity: "10".into(),
                    unit: "g".into(),
                    initial_value: None,
                    excess: None,
                },
            }],
        },
    };
    first.original_input = input.clone();
    first.effective_input = Some(input);
    h.events[1].observations[0].previous_observation_version_id = Some(id(42));
    let second = &mut h.events[1].observations[0].effect;
    second.basis_before = Some(HoldingBasisRef {
        kind: HoldingBasisKind::Observation,
        id: Some(id(42)),
        seq: Some("3".into()),
    });
    second.observation_before = Some(sunchang_hansang::holding::HoldingObservation {
        id: id(42),
        quantity: "10".into(),
        recorded_at: TIME.into(),
        seq: "3".into(),
    });
    if let StocktakeCalculation::Settlement { allocation, .. } = &mut second.calculation {
        allocation.basis = second.basis_before.clone().unwrap();
        h.actual_allocations[0].allocation = allocation.as_ref().clone();
    }
    if let StocktakeCalculation::Settlement { allocation, .. } =
        &first.observations[0].effect.calculation
    {
        h.actual_allocations.push(CorrectionAllocationSnapshot {
            observation_id: id(41),
            consumption_version_id: id(43),
            allocation: allocation.as_ref().clone(),
        });
    }
    h.events.insert(1, first);
    let mut r = receipt("3", "60");
    r.stock_effect=serde_json::from_value(json!({"kind":"reconciled","selections":[{"observation_version_id":id(42),"source":{"kind":"all_available"},"included_quantity":"3"}]})).unwrap();
    let input = CorrectionReplacementDraft::Receipt { data: r.clone() };
    h.events.push(CorrectionHistoryEvent {
        reference: reference(CorrectionSubjectKind::Receipt, 90, 5),
        original_input: input.clone(),
        effective_input: Some(input),
        effective_value: None,
        observations: vec![],
    });
    let reconciliation: sunchang_hansang::reconciliation::ReceiptReconciliation=serde_json::from_value(json!({"id":id(91),"receipt_id":id(90),"confirmation_id":id(190),"business_record_id":id(290),"recorded_at":TIME,"seq":"5","selection_results":[],"material_results":[],"effect":{
        "material_id":id(1),"material_version_id":id(2),"material_name":"가상 정정 품목","quantity":"3","amount":"60","unit":"g","input_quantity":"3","input_unit":"g","receipt_date":"2020-02-29","included_quantity":"3","unreflected_quantity":"0","included_amount":"60","unreflected_amount":"0","booked_value":"30","value_difference":"30","separate_adjustment":"12","materials":[],"trace":[],"order_before":null,"order_after":null,
        "selections":[{"position":0,"stocktake_id":id(40),"observation_id":id(41),"observation_version_id":id(42),"source":{"kind":"all_available"},"included_quantity":"3","unit":"g","booked_value":"30","actual_included_value":"60","value_difference":"30","portions":[{"position":0,"source_key":{"kind":"receipt_inflow","id":id(10),"position":0},"source_version_id":id(10),"included_share":{"numerator":"3","denominator":"1","unit":"g"},"reserved_source_share":{"numerator":"3","denominator":"1","unit":"g"},"booked_value":"30","actual_included_value":"60"}]}]
    }})).unwrap();
    h.existing_inclusions.push(CorrectionEffectiveInclusion {
        receipt_id: id(90),
        effective_confirmation_id: id(190),
        reconciliation,
    });
    if let CorrectionHoldingState::Known {
        amounts,
        last_valid_price,
        ..
    } = &mut h.actual_materials[0].state
    {
        amounts.value = Some("78".into());
        *last_valid_price = Some(price("78", "6", CorrectionRefKind::Receipt, 90, 5));
    }
    if with_later_observation {
        let mut later = h.events[2].clone();
        later.reference = reference(CorrectionSubjectKind::Stocktake, 50, 6);
        let o = &mut later.observations[0];
        o.id = id(51);
        o.version_id = id(52);
        o.stocktake_id = id(50);
        o.previous_observation_version_id = Some(id(32));
        o.consumption_version_id = Some(id(53));
        o.seq = "6".into();
        o.effect.input_quantity = "5".into();
        o.effect.measured_quantity = "5".into();
        o.effect.before = Some(HoldingAmounts {
            quantity: "6".into(),
            value: Some("78".into()),
            source_id: Some(id(90)),
        });
        o.effect.after.quantity = "5".into();
        o.effect.after.value = Some("65".into());
        o.effect.basis_before = Some(HoldingBasisRef {
            kind: HoldingBasisKind::Observation,
            id: Some(id(32)),
            seq: Some("4".into()),
        });
        o.effect.observation_before = Some(sunchang_hansang::holding::HoldingObservation {
            id: id(32),
            quantity: "6".into(),
            recorded_at: TIME.into(),
            seq: "4".into(),
        });
        if let StocktakeCalculation::Settlement {
            consumption_quantity,
            consumption_value,
            allocation,
            ..
        } = &mut o.effect.calculation
        {
            *consumption_quantity = "1".into();
            *consumption_value = "13".into();
            allocation.basis = o.effect.basis_before.clone().unwrap();
            allocation.end_seq = "6".into();
            allocation.total_quantity = "1".into();
            allocation.total_value = "13".into();
            allocation.unallocated_quantity = "1".into();
            allocation.unallocated_value = "13".into();
            h.actual_allocations.push(CorrectionAllocationSnapshot {
                observation_id: id(51),
                consumption_version_id: id(53),
                allocation: allocation.as_ref().clone(),
            });
        }
        let input = CorrectionReplacementDraft::Stocktake {
            data: DraftStocktakeCorrectionV1 {
                rows: vec![StocktakeCorrectionRowDraft {
                    observation_id: Some(id(51)),
                    expected_version_id: Some(id(52)),
                    action: ObservationCorrectionActionDraft::Replace {
                        quantity: "5".into(),
                        unit: "g".into(),
                        initial_value: None,
                        excess: None,
                    },
                }],
            },
        };
        later.original_input = input.clone();
        later.effective_input = Some(input);
        h.events.push(later);
        if let CorrectionHoldingState::Known {
            amounts,
            last_valid_price,
            observation_boundary,
            ..
        } = &mut h.actual_materials[0].state
        {
            amounts.quantity = "5".into();
            amounts.value = Some("65".into());
            *last_valid_price = Some(price("65", "5", CorrectionRefKind::Observation, 52, 6));
            *observation_boundary = Some(CorrectionObservationBoundary {
                reference: CorrectionPlanRef::Existing {
                    entity: CorrectionRefKind::Observation,
                    id: id(51),
                    version_id: id(52),
                },
                stocktake_id: id(50),
                logical_seq: "6".into(),
                observed_at: TIME.into(),
                quantity: "5".into(),
            });
        }
    }
    h
}

#[test]
fn late_value_adjustment_does_not_rewrite_prior_consumption() {
    let h = late_history(false);
    let check = build_correction_plan(
        &h,
        &id(110),
        &draft(CorrectionActionDraft::Replace {
            replacement: CorrectionReplacementDraft::Receipt {
                data: receipt("10", "100"),
            },
        }),
    )
    .unwrap();
    assert!(check.issues.is_empty(), "{:?}", check.issues);
    assert!(
        check.plan.allocations.is_empty(),
        "{:?}",
        check.plan.allocations
    );
    assert!(check.plan.materials.is_empty());
    assert!(check.plan.separate_adjustments.is_empty());
}

#[test]
fn observation_after_late_receipt_uses_known_value_without_second_separate_charge() {
    let h = late_history(true);
    let check = build_correction_plan(
        &h,
        &id(110),
        &draft(CorrectionActionDraft::Replace {
            replacement: CorrectionReplacementDraft::Receipt {
                data: receipt("10", "100"),
            },
        }),
    )
    .unwrap();
    assert!(check.issues.is_empty(), "{:?}", check.issues);
    assert!(
        check.plan.allocations.is_empty(),
        "{:?}",
        check.plan.allocations
    );
    assert!(check.plan.materials.is_empty());
    assert!(check.plan.separate_adjustments.is_empty());
}

#[test]
fn physical_price_correction_keeps_frozen_inclusion_and_updates_known_timeline() {
    let h = late_history(true);
    let check = build_correction_plan(
        &h,
        &id(110),
        &draft(CorrectionActionDraft::Replace {
            replacement: CorrectionReplacementDraft::Receipt {
                data: receipt("10", "200"),
            },
        }),
    )
    .unwrap();
    assert!(check.issues.is_empty(), "{:?}", check.issues);
    let at4 = check
        .plan
        .observations
        .iter()
        .find(|o| o.logical_seq == "4")
        .unwrap()
        .after
        .as_ref()
        .unwrap();
    let at6 = check
        .plan
        .observations
        .iter()
        .find(|o| o.logical_seq == "6")
        .unwrap()
        .after
        .as_ref()
        .unwrap();
    let StocktakeCalculation::Settlement {
        consumption_value, ..
    } = &at4.calculation
    else {
        panic!()
    };
    assert_eq!(consumption_value, "80");
    let StocktakeCalculation::Settlement {
        consumption_value, ..
    } = &at6.calculation
    else {
        panic!()
    };
    assert_eq!(consumption_value, "23");
    let CorrectionHoldingState::Known { amounts, .. } = &check.plan.materials[0].after else {
        panic!()
    };
    assert_eq!(amounts.quantity, "5");
    assert_eq!(amounts.value.as_deref(), Some("115"));
    assert!(check.plan.separate_adjustments.is_empty());
}

#[test]
fn negative_late_adjustment_keeps_later_valid_receipt_below_money_limit() {
    // All actual values fit Money: 9e12 -> consume 4/10 => 5.4e12;
    // late I=3,A=0 gives D=-2.7e12, current dV=-1.62e12 => 3.78e12;
    // a later ordinary receipt adds 4.5e12 => 8.28e12. A source-only
    // no-D pass would incorrectly add 5.4e12+4.5e12 and overflow.
    let mut h = late_history(false);
    if let CorrectionReplacementDraft::Receipt { data } = &mut h.events[0].original_input {
        data.amount = "9000000000000".into();
    }
    h.events[0].effective_input = Some(h.events[0].original_input.clone());
    for event in &mut h.events[1..3] {
        let effect = &mut event.observations[0].effect;
        effect.before.as_mut().unwrap().value = Some("9000000000000".into());
        effect.after.value = Some(
            if event.reference.logical_seq == "3" {
                "9000000000000"
            } else {
                "5400000000000"
            }
            .into(),
        );
        if let StocktakeCalculation::Settlement {
            consumption_value,
            allocation,
            ..
        } = &mut effect.calculation
        {
            let value = if event.reference.logical_seq == "3" {
                "0"
            } else {
                "3600000000000"
            };
            *consumption_value = value.into();
            allocation.total_value = value.into();
            allocation.unallocated_value = value.into();
        }
    }
    for allocation in &mut h.actual_allocations {
        if allocation.observation_id == id(31) {
            allocation.allocation.total_value = "3600000000000".into();
            allocation.allocation.unallocated_value = "3600000000000".into();
        }
    }
    if let CorrectionReplacementDraft::Receipt { data } = &mut h.events[3].original_input {
        data.amount = "0".into();
    }
    h.events[3].effective_input = Some(h.events[3].original_input.clone());
    let late = &mut h.existing_inclusions[0].reconciliation.effect;
    late.amount = "0".into();
    late.included_amount = "0".into();
    late.booked_value = "2700000000000".into();
    late.value_difference = "-2700000000000".into();
    late.separate_adjustment = "-1080000000000".into();
    let selection = &mut late.selections[0];
    selection.booked_value = "2700000000000".into();
    selection.actual_included_value = "0".into();
    selection.value_difference = "-2700000000000".into();
    selection.portions[0].booked_value = "2700000000000".into();
    selection.portions[0].actual_included_value = "0".into();
    let input = CorrectionReplacementDraft::Receipt {
        data: receipt("1", "4500000000000"),
    };
    h.events.push(CorrectionHistoryEvent {
        reference: reference(CorrectionSubjectKind::Receipt, 70, 6),
        original_input: input.clone(),
        effective_input: Some(input),
        effective_value: None,
        observations: vec![],
    });
    if let CorrectionHoldingState::Known {
        amounts,
        last_valid_price,
        ..
    } = &mut h.actual_materials[0].state
    {
        amounts.quantity = "7".into();
        amounts.value = Some("8280000000000".into());
        *last_valid_price = Some(price(
            "8280000000000",
            "7",
            CorrectionRefKind::Receipt,
            70,
            6,
        ));
    }
    let check = build_correction_plan(
        &h,
        &id(110),
        &draft(CorrectionActionDraft::Replace {
            replacement: CorrectionReplacementDraft::Receipt {
                data: receipt("10", "9000000000000"),
            },
        }),
    )
    .unwrap();
    assert!(check.issues.is_empty(), "{:?}", check.issues);
    assert!(check.plan.materials.is_empty());
}

#[test]
fn initial_production_basis_retains_its_kind_in_observation_replay() {
    let mut h = with_observation(initial_history());
    let mut output = h.materials[0].clone();
    output.material_id = id(3);
    output.id = id(4);
    output.name = "가상 생산 결과".into();
    h.materials.push(output);
    let data=serde_json::from_value(json!({"inputs":[{"material_id":id(1),"material_version_id":id(2),"quantity":"10","unit":"g"}],"outputs":[{"material_id":id(3),"material_version_id":id(4),"quantity":"10","unit":"g","allocation_percent":"100"}],"note":"가상 생산 시작 기준"})).unwrap();
    let input = CorrectionReplacementDraft::Production { data };
    h.events.insert(
        1,
        CorrectionHistoryEvent {
            reference: reference(CorrectionSubjectKind::Production, 60, 3),
            original_input: input.clone(),
            effective_input: Some(input),
            effective_value: None,
            observations: vec![],
        },
    );
    let basis = HoldingBasisRef {
        kind: HoldingBasisKind::Production,
        id: Some(id(60)),
        seq: Some("3".into()),
    };
    let effect = &mut h.events[2].observations[0].effect;
    effect.material_id = id(3);
    effect.material_version_id = id(4);
    effect.material_name = "가상 생산 결과".into();
    effect.basis_before = Some(basis.clone());
    if let StocktakeCalculation::Settlement { allocation, .. } = &mut effect.calculation {
        allocation.basis = basis.clone();
    }
    h.actual_allocations[0].allocation.basis = basis;
    h.actual_materials[0].material_id = id(3);
    if let CorrectionHoldingState::Known {
        initial_boundary, ..
    } = &mut h.actual_materials[0].state
    {
        *initial_boundary = Some(CorrectionInitialBoundary {
            reference: CorrectionPlanRef::Existing {
                entity: CorrectionRefKind::Production,
                id: id(60),
                version_id: id(60),
            },
            logical_seq: "3".into(),
        });
    }
    let mut consumed = initial_history().actual_materials.remove(0);
    if let CorrectionHoldingState::Known { amounts, .. } = &mut consumed.state {
        amounts.quantity = "0".into();
        amounts.value = Some("0".into());
    }
    h.actual_materials.push(consumed);
    let check = build_correction_plan(
        &h,
        &id(110),
        &draft(CorrectionActionDraft::Replace {
            replacement: CorrectionReplacementDraft::Receipt {
                data: receipt("10", "100"),
            },
        }),
    )
    .unwrap();
    assert!(check.issues.is_empty(), "{:?}", check.issues);
    assert!(check.plan.allocations.is_empty());
    assert!(check.plan.materials.is_empty());
}

#[test]
fn initial_reconciled_inflow_basis_retains_its_kind_in_observation_replay() {
    let mut h = with_observation(initial_history());
    let mut data = receipt("10", "100");
    data.stock_effect = ReceiptStockEffectDraft::Reconciled { selections: vec![] };
    let input = CorrectionReplacementDraft::Receipt { data: data.clone() };
    h.events[0].original_input = input.clone();
    h.events[0].effective_input = Some(input);
    let mut record = late_history(false).existing_inclusions.remove(0);
    record.receipt_id = id(10);
    record.effective_confirmation_id = id(110);
    let r = &mut record.reconciliation;
    r.receipt_id = id(10);
    r.confirmation_id = id(110);
    r.business_record_id = id(210);
    r.seq = "2".into();
    let e = &mut r.effect;
    e.quantity = "10".into();
    e.amount = "100".into();
    e.input_quantity = "10".into();
    e.included_quantity = "0".into();
    e.unreflected_quantity = "10".into();
    e.included_amount = "0".into();
    e.unreflected_amount = "100".into();
    e.booked_value = "0".into();
    e.value_difference = "0".into();
    e.separate_adjustment = "0".into();
    e.selections.clear();
    h.existing_inclusions.push(record);
    let basis = HoldingBasisRef {
        kind: HoldingBasisKind::Reconciliation,
        id: Some(id(10)),
        seq: Some("2".into()),
    };
    let effect = &mut h.events[1].observations[0].effect;
    effect.basis_before = Some(basis.clone());
    if let StocktakeCalculation::Settlement { allocation, .. } = &mut effect.calculation {
        allocation.basis = basis.clone();
    }
    h.actual_allocations[0].allocation.basis = basis;
    let check = build_correction_plan(
        &h,
        &id(110),
        &draft(CorrectionActionDraft::Replace {
            replacement: CorrectionReplacementDraft::Receipt { data },
        }),
    )
    .unwrap();
    assert!(check.issues.is_empty(), "{:?}", check.issues);
    assert!(check.plan.allocations.is_empty());
    assert!(check.plan.materials.is_empty());
}
