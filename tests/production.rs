use serde_json::json;
use sunchang_hansang::{
    domain::{Money, Quantity, UnitPrice},
    holding::{HoldingBasisKind, HoldingBasisRef, HoldingInput, HoldingObservation},
    production::{
        AllocationPercent, DraftProductionInput, DraftProductionOutput, DraftProductionV1,
        ProductionCheck, ProductionConflictReason, ProductionMaterialContext, ProductionPreview,
        normalize_production_draft, project_production,
    },
};
use uuid::Uuid;

fn id(n: u128) -> String {
    Uuid::from_u128(n).to_string()
}
fn known(quantity: &str, value: Option<&str>) -> HoldingInput {
    HoldingInput {
        basis: Some(HoldingBasisRef {
            kind: HoldingBasisKind::Receipt,
            id: Some(id(100)),
            seq: Some("10".into()),
        }),
        observation: None,
        quantity: Some(Quantity::parse(quantity).unwrap()),
        value: value.map(|v| Money::parse(v).unwrap()),
        source_id: Some(id(101)),
        last_valid: None,
    }
}
fn context(n: u128, holding: HoldingInput) -> ProductionMaterialContext {
    ProductionMaterialContext {
        material_id: id(n),
        material_version_id: id(n + 10),
        material_name: format!("가상 품목 {n}"),
        unit: "g".into(),
        holding,
    }
}
fn input(n: u128, quantity: &str, unit: &str) -> DraftProductionInput {
    DraftProductionInput {
        material_id: Some(id(n)),
        material_version_id: Some(id(n + 10)),
        quantity: quantity.into(),
        unit: unit.into(),
    }
}
fn output(n: u128, quantity: &str, percent: &str) -> DraftProductionOutput {
    DraftProductionOutput {
        material_id: Some(id(n)),
        material_version_id: Some(id(n + 10)),
        quantity: quantity.into(),
        unit: "g".into(),
        allocation_percent: percent.into(),
    }
}
fn data(
    inputs: Vec<DraftProductionInput>,
    outputs: Vec<DraftProductionOutput>,
) -> DraftProductionV1 {
    DraftProductionV1 {
        inputs,
        outputs,
        note: "가상 생산 순수 검증".into(),
    }
}
fn ready(data: &DraftProductionV1, contexts: &[ProductionMaterialContext]) -> ProductionPreview {
    match project_production(data, contexts).unwrap() {
        ProductionCheck::Ready(result) => result,
        other => panic!("expected ready: {other:?}"),
    }
}

#[test]
fn measured_production_carries_all_value_and_preserves_original_quantities() {
    let draft = data(vec![input(1, "3", "kg")], vec![output(2, "2000", "100")]);
    let contexts = [
        context(1, known("3000", Some("30000"))),
        context(2, HoldingInput::default()),
    ];
    let result = ready(&draft, &contexts);
    assert_eq!(result.total_input_value, "30000");
    assert_eq!(result.inputs[0].quantity, "3000");
    assert_eq!(result.inputs[0].input_quantity, "3");
    assert_eq!(result.inputs[0].input_unit, "kg");
    assert_eq!(result.inputs[0].unit, "g");
    assert_eq!(result.outputs[0].value, "30000");
    assert_eq!(result.materials[0].after.quantity, "0");
    assert_eq!(result.materials[0].after.value.as_deref(), Some("0"));
    assert_eq!(
        result.materials[0]
            .before
            .as_ref()
            .unwrap()
            .source_id
            .as_deref(),
        Some(id(101).as_str())
    );
    assert!(result.materials[1].before.is_none());
    assert!(result.materials[1].basis_before.is_none());
    assert!(
        result
            .materials
            .iter()
            .all(|m| m.after.source_id.is_none() && m.observation.is_none())
    );
    assert_eq!(contexts[0].holding.quantity.unwrap().decimal(), "3000");
}

#[test]
fn user_percentages_and_zero_value_results_preserve_the_exact_total() {
    let contexts = [
        context(1, known("3000", Some("30000"))),
        context(2, HoldingInput::default()),
        context(3, HoldingInput::default()),
    ];
    for (first, second, first_value, second_value) in
        [("100", "0", "30000", "0"), ("70", "30", "21000", "9000")]
    {
        let result = ready(
            &data(
                vec![input(1, "3", "kg")],
                vec![output(2, "2000", first), output(3, "1000", second)],
            ),
            &contexts,
        );
        assert_eq!(result.outputs[0].value, first_value);
        assert_eq!(result.outputs[1].value, second_value);
        assert_eq!(result.total_input_value, "30000");
    }
}

#[test]
fn duplicate_inputs_round_the_group_once_then_allocate_back_to_rows() {
    let contexts = [
        context(1, known("3", Some("0.000001"))),
        context(2, HoldingInput::default()),
    ];
    let result = ready(
        &data(
            vec![input(1, "1", "g"), input(1, "1", "g")],
            vec![output(2, "2", "100")],
        ),
        &contexts,
    );
    assert_eq!(result.total_input_value, "0.000001");
    assert_eq!(
        result
            .inputs
            .iter()
            .map(|r| r.value.as_str())
            .collect::<Vec<_>>(),
        ["0.000001", "0"]
    );
    assert_eq!(result.inputs[0].position, "0");
    assert_eq!(result.inputs[1].position, "1");
    assert_eq!(result.materials.len(), 2);
    assert_eq!(result.materials[0].after.quantity, "1");
    assert_eq!(result.materials[0].after.value.as_deref(), Some("0"));
    let full = ready(
        &data(
            vec![input(1, "1", "g"), input(1, "2", "g")],
            vec![output(2, "3", "100")],
        ),
        &contexts,
    );
    assert_eq!(
        full.inputs
            .iter()
            .map(|r| r.value.as_str())
            .collect::<Vec<_>>(),
        ["0", "0.000001"]
    );
    assert_eq!(full.materials[0].after.value.as_deref(), Some("0"));
}

#[test]
fn output_remainders_are_exact_and_stable_without_future_ids() {
    let contexts = [
        context(1, known("1", Some("0.000001"))),
        context(2, HoldingInput::default()),
        context(3, HoldingInput::default()),
        context(4, HoldingInput::default()),
    ];
    let result = ready(
        &data(
            vec![input(1, "1", "g")],
            vec![
                output(2, "1", "33.333333"),
                output(3, "1", "33.333333"),
                output(4, "1", "33.333334"),
            ],
        ),
        &contexts,
    );
    assert_eq!(
        result
            .outputs
            .iter()
            .map(|r| r.value.as_str())
            .collect::<Vec<_>>(),
        ["0", "0", "0.000001"]
    );
    let repeated_contexts = &contexts[..3];
    let draft = data(
        vec![input(1, "1", "g")],
        vec![output(2, "1", "50"), output(3, "1", "50")],
    );
    let first = ready(&draft, repeated_contexts);
    assert_eq!(
        first
            .outputs
            .iter()
            .map(|r| r.value.as_str())
            .collect::<Vec<_>>(),
        ["0.000001", "0"]
    );
    assert_eq!(
        serde_json::to_value(&first).unwrap(),
        serde_json::to_value(ready(&draft, repeated_contexts)).unwrap()
    );
}

#[test]
fn same_material_inputs_and_outputs_use_before_and_create_one_effect() {
    let contexts = [context(1, known("1000", Some("10000")))];
    let draft = data(
        vec![input(1, "0.4", "kg"), input(1, "0.6", "kg")],
        vec![output(1, "250", "20"), output(1, "750", "80")],
    );
    let result = ready(&draft, &contexts);
    assert_eq!(result.materials.len(), 1);
    assert_eq!(result.materials[0].input_quantity, "1000");
    assert_eq!(result.materials[0].output_quantity, "1000");
    assert_eq!(result.materials[0].after.quantity, "1000");
    assert_eq!(result.materials[0].after.value.as_deref(), Some("10000"));
    for inputs in [
        vec![input(1, "1.2", "kg")],
        vec![input(1, "0.6", "kg"), input(1, "0.6", "kg")],
    ] {
        let blocked = data(inputs, vec![output(1, "2000", "100")]);
        let ProductionCheck::NeedsReconciliation(issues) =
            project_production(&blocked, &contexts).unwrap()
        else {
            panic!("output must not fund input")
        };
        assert_eq!(
            issues[0].reason,
            ProductionConflictReason::InsufficientQuantity
        );
        assert_eq!(issues[0].requested_quantity, "1200");
        assert_eq!(issues[0].available_quantity.as_deref(), Some("1000"));
        assert_eq!(issues[0].field, "inputs.0.quantity");
    }
}

#[test]
fn unknown_quantity_unknown_value_and_known_zero_remain_distinct() {
    let draft = data(vec![input(1, "1", "g")], vec![output(2, "1", "100")]);
    for (holding, reason, available) in [
        (
            HoldingInput::default(),
            ProductionConflictReason::UnknownQuantity,
            None,
        ),
        (
            known("2", None),
            ProductionConflictReason::UnknownValue,
            Some("2"),
        ),
        (
            known("0", Some("0")),
            ProductionConflictReason::InsufficientQuantity,
            Some("0"),
        ),
    ] {
        let ProductionCheck::NeedsReconciliation(issues) = project_production(
            &draft,
            &[context(1, holding), context(2, HoldingInput::default())],
        )
        .unwrap() else {
            panic!("must require a basis")
        };
        assert_eq!(issues[0].reason, reason);
        assert_eq!(issues[0].available_quantity.as_deref(), available);
        assert!(!issues[0].message().is_empty());
    }
    let result = project_production(
        &draft,
        &[
            context(1, known("2", Some("1"))),
            context(2, known("1", None)),
        ],
    )
    .unwrap();
    let ProductionCheck::NeedsReconciliation(issues) = result else {
        panic!("unknown output value must not become zero")
    };
    assert_eq!(issues[0].field, "outputs.0.quantity");
    assert_eq!(issues[0].reason, ProductionConflictReason::UnknownValue);
}

#[test]
fn all_zero_value_is_valid_and_does_not_restore_an_older_positive_price() {
    let mut current_zero_value = known("1", Some("0"));
    current_zero_value.last_valid = Some(
        UnitPrice::new(
            Money::parse("900").unwrap(),
            Quantity::parse("1").unwrap(),
            id(110),
        )
        .unwrap(),
    );
    let result = ready(
        &data(vec![input(1, "1", "g")], vec![output(2, "2", "100")]),
        &[
            context(1, current_zero_value),
            context(2, HoldingInput::default()),
        ],
    );
    assert_eq!(result.total_input_value, "0");
    assert_eq!(result.inputs[0].value, "0");
    assert_eq!(result.outputs[0].value, "0");
    assert_eq!(result.materials[0].after.quantity, "0");
    assert_eq!(
        result.materials[0]
            .before
            .as_ref()
            .unwrap()
            .value
            .as_deref(),
        Some("0")
    );
    assert_eq!(result.materials[1].after.value.as_deref(), Some("0"));
}

#[test]
fn synthetic_reconciliation_preserves_actual_before_source_and_observation() {
    let draft = data(vec![input(1, "1.2", "kg")], vec![output(2, "800", "100")]);
    assert!(matches!(
        project_production(
            &draft,
            &[
                context(1, known("1000", Some("10000"))),
                context(2, HoldingInput::default())
            ]
        )
        .unwrap(),
        ProductionCheck::NeedsReconciliation(_)
    ));
    let mut corrected = known("1200", Some("12000"));
    corrected.basis = Some(HoldingBasisRef {
        kind: HoldingBasisKind::Observation,
        id: Some(id(900)),
        seq: Some("20".into()),
    });
    corrected.source_id = Some(id(901));
    corrected.observation = Some(HoldingObservation {
        id: id(900),
        quantity: "1200".into(),
        recorded_at: "2026-01-01T10:00:00Z".into(),
        seq: "20".into(),
    });
    let result = ready(
        &draft,
        &[
            context(1, corrected.clone()),
            context(2, HoldingInput::default()),
        ],
    );
    assert_eq!(
        result.materials[0].before.as_ref().unwrap().source_id,
        corrected.source_id
    );
    assert_eq!(result.materials[0].basis_before, corrected.basis);
    assert_eq!(result.materials[0].observation, corrected.observation);
    assert_eq!(result.materials[0].after.quantity, "0");
    assert_eq!(result.total_input_value, "12000");
    assert_eq!(draft.inputs[0].quantity, "1.2");
}

#[test]
fn different_dimensions_can_be_transformed_but_each_row_must_match_its_material() {
    let mut out = context(2, HoldingInput::default());
    out.unit = "mL".into();
    let contexts = [context(1, known("1000", Some("100"))), out];
    let mut draft = data(vec![input(1, "1", "kg")], vec![output(2, "2", "100")]);
    draft.outputs[0].unit = "L".into();
    assert_eq!(ready(&draft, &contexts).outputs[0].quantity, "2000");
    draft.inputs[0].unit = "mL".into();
    assert!(project_production(&draft, &contexts).is_err());
}

#[test]
fn incomplete_invalid_ranges_and_percentages_never_create_partial_effects() {
    let contexts = [
        context(1, known("1000", Some("10000"))),
        context(2, HoldingInput::default()),
    ];
    let baseline = data(vec![input(1, "1", "g")], vec![output(2, "1", "100")]);
    for q in ["", "0", "-1", "1e2", "0.0000001", "9000000001"] {
        let mut draft = baseline.clone();
        draft.inputs[0].quantity = q.into();
        assert!(project_production(&draft, &contexts).is_err(), "{q}");
    }
    for percentage in ["", "0", "99", "101", "-1", "1e2", "100.0000001"] {
        let mut draft = baseline.clone();
        draft.outputs[0].allocation_percent = percentage.into();
        assert!(
            project_production(&draft, &contexts).is_err(),
            "{percentage}"
        );
    }
    let mut missing = baseline.clone();
    missing.inputs[0].material_id = None;
    assert!(project_production(&missing, &contexts).is_err());
    let mut missing = baseline.clone();
    missing.inputs.clear();
    assert!(project_production(&missing, &contexts).is_err());
    let mut missing = baseline.clone();
    missing.outputs.clear();
    assert!(project_production(&missing, &contexts).is_err());
    let mut stale = baseline.clone();
    stale.inputs[0].material_version_id = Some(id(999));
    assert!(project_production(&stale, &contexts).is_err());
    assert!(
        project_production(
            &baseline,
            &[
                contexts[0].clone(),
                contexts[0].clone(),
                contexts[1].clone()
            ]
        )
        .is_err()
    );
    let mut malformed = contexts.to_vec();
    malformed[0].holding.source_id = None;
    assert!(project_production(&baseline, &malformed).is_err());
}

#[test]
fn group_total_and_after_bounds_are_checked_without_float_arithmetic() {
    let max_q = "9000000000";
    let max_v = "9000000000000";
    let contexts = [
        context(1, known(max_q, Some(max_v))),
        context(2, HoldingInput::default()),
    ];
    let draft = data(vec![input(1, max_q, "g")], vec![output(2, "1", "100")]);
    assert_eq!(ready(&draft, &contexts).total_input_value, max_v);
    let overflow = data(
        vec![input(1, max_q, "g"), input(1, "0.000001", "g")],
        vec![output(2, "1", "100")],
    );
    assert!(project_production(&overflow, &contexts).is_err());
    let contexts = [
        context(1, known("1", Some(max_v))),
        context(2, known("1", Some("0.000001"))),
        context(3, HoldingInput::default()),
    ];
    let overflow = data(
        vec![input(1, "1", "g"), input(2, "1", "g")],
        vec![output(3, "1", "100")],
    );
    assert!(project_production(&overflow, &contexts).is_err());
    let contexts = [
        context(1, known("1", Some("1"))),
        context(2, known(max_q, Some("0"))),
    ];
    assert!(
        project_production(
            &data(vec![input(1, "1", "g")], vec![output(2, "0.000001", "100")]),
            &contexts
        )
        .is_err()
    );
}

#[test]
fn draft_normalization_is_strict_but_preserves_incomplete_user_text() {
    let mut draft: DraftProductionV1 = serde_json::from_value(json!({"inputs":[{"material_id":"ABCDEFAB-1234-4000-8000-123456789ABC","material_version_id":null,"quantity":" 1.","unit":""}],"outputs":[],"note":"가상\r\n메모\r끝"})).unwrap();
    normalize_production_draft(&mut draft).unwrap();
    assert_eq!(
        draft.inputs[0].material_id.as_deref(),
        Some("abcdefab-1234-4000-8000-123456789abc")
    );
    assert_eq!(draft.inputs[0].quantity, " 1.");
    assert_eq!(draft.note, "가상\n메모\n끝");
    let raw = serde_json::to_value(&draft).unwrap();
    for changed in [
        {
            let mut x = raw.clone();
            x["extra"] = json!(true);
            x
        },
        {
            let mut x = raw.clone();
            x["inputs"][0]["extra"] = json!(true);
            x
        },
        {
            let mut x = raw.clone();
            x["inputs"][0]["quantity"] = json!(1);
            x
        },
    ] {
        assert!(serde_json::from_value::<DraftProductionV1>(changed).is_err());
    }
    let mut too_many = draft.clone();
    too_many.inputs = vec![draft.inputs[0].clone(); 101];
    assert!(normalize_production_draft(&mut too_many).is_err());
    let mut too_long = draft.clone();
    too_long.note = "가".repeat(2001);
    assert!(normalize_production_draft(&mut too_long).is_err());
    let mut control = draft;
    control.inputs[0].quantity = "1\n2".into();
    assert!(normalize_production_draft(&mut control).is_err());
    assert_eq!(
        AllocationPercent::parse(" 70.000000 ").unwrap().decimal(),
        "70"
    );
    assert!(AllocationPercent::parse("100.000001").is_err());
}
