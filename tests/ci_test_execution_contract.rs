use std::fs;

use serde_yaml::Value;

const EXPECTED_COMPILE_AND_TEST_RUN: &str = "cargo test --all-targets --locked --no-fail-fast";

/// Proves the canonical CI test step executes every test binary and cannot hide a failing target.
#[test]
fn ci_runs_every_test_binary_before_failing_the_gate() {
    let workflow_source =
        fs::read_to_string(".github/workflows/ci.yml").expect("CI workflow should be readable");
    let workflow: Value =
        serde_yaml::from_str(&workflow_source).expect("CI workflow should parse as YAML");
    let jobs = workflow
        .get("jobs")
        .and_then(Value::as_mapping)
        .expect("CI workflow should define jobs");

    let compile_and_test_steps = jobs
        .values()
        .filter_map(Value::as_mapping)
        .filter_map(|job| job.get("steps"))
        .filter_map(Value::as_sequence)
        .flatten()
        .filter_map(Value::as_mapping)
        .filter(|step| step.get("name").and_then(Value::as_str) == Some("Compile and test"))
        .collect::<Vec<_>>();

    assert_eq!(
        compile_and_test_steps.len(),
        1,
        "CI should have one canonical compile-and-test step"
    );
    let step = compile_and_test_steps[0];
    assert_eq!(
        step.get("run").and_then(Value::as_str),
        Some(EXPECTED_COMPILE_AND_TEST_RUN),
        "CI must execute all test targets without target-level fail-fast"
    );
    assert!(
        step.get("if").is_none(),
        "canonical compile-and-test execution must not be conditional"
    );
    assert!(
        matches!(step.get("continue-on-error"), None | Some(Value::Bool(false))),
        "canonical compile-and-test failures must fail the CI job"
    );
}
