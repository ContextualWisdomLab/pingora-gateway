//! Fail-closed contract for routed-load failure propagation in GitHub Actions.
//!
//! Routed k6 thresholds are release evidence only when the measured step and its enclosing job
//! propagate a non-zero k6 exit status. A green workflow must not be manufactured by
//! `continue-on-error` around the evidence-bearing step or job.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const ROUTED_K6_LINE: &str = "k6 run --quiet tests/load/pg_erd_gateway_smoke.js";

fn failure_propagates(node: &Value) -> bool {
    match node.get("continue-on-error") {
        None | Some(Value::Bool(false)) => true,
        Some(_) => false,
    }
}

fn routed_load_failure_propagates(source: &str) -> bool {
    let Ok(document) = serde_yaml::from_str::<Value>(source) else {
        return false;
    };
    let Some(job) = document.get("jobs").and_then(|jobs| jobs.get(LOAD_JOB)) else {
        return false;
    };
    if !failure_propagates(job) {
        return false;
    }

    let Some(steps) = job.get("steps").and_then(Value::as_sequence) else {
        return false;
    };
    let Some(step) = steps.iter().find(|step| {
        step.get("name").and_then(Value::as_str) == Some(ROUTED_STEP)
    }) else {
        return false;
    };
    if step.get("if").is_some() || !failure_propagates(step) {
        return false;
    }

    let Some(run) = step.get("run").and_then(Value::as_str) else {
        return false;
    };

    run.lines().any(|line| line.trim() == ROUTED_K6_LINE)
}

#[test]
fn live_workflow_propagates_routed_load_failure() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        routed_load_failure_propagates(&source),
        "routed k6 threshold failures must fail the load-contract job"
    );
}

#[test]
fn step_continue_on_error_must_not_mask_routed_load_failure() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        continue-on-error: true
        run: |
          k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(
        !routed_load_failure_propagates(source),
        "an evidence-bearing step with continue-on-error=true must not earn GREEN"
    );
}

#[test]
fn job_continue_on_error_must_not_mask_routed_load_failure() {
    let source = r#"
jobs:
  load-contract:
    continue-on-error: true
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(
        !routed_load_failure_propagates(source),
        "a load-contract job with continue-on-error=true must not earn GREEN"
    );
}

#[test]
fn conditional_routed_step_must_not_claim_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        if: ${{ false }}
        run: |
          k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(
        !routed_load_failure_propagates(source),
        "a conditional evidence-bearing step must not claim unconditional release evidence"
    );
}
