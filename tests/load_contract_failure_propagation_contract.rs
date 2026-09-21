//! Fail-closed contract for routed-load failure propagation in GitHub Actions.
//!
//! Routed k6 thresholds are release evidence only when the measured step, summary-presence gate,
//! and their enclosing job propagate failures. A green workflow must not be manufactured by
//! `continue-on-error`, skip conditions, duplicate named decoys, or non-executing shell overrides
//! around this evidence path.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const LOAD_JOB_IF: &str = "github.event_name != 'pull_request' || github.event.pull_request.draft == false";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const ROUTED_K6_LINE: &str = "k6 run --quiet tests/load/pg_erd_gateway_smoke.js";
const SUMMARY_STEP: &str = "Require routed pg-erd latency summary";
const SUMMARY_RUN: &str = "test -s k6-pg-erd-summary.json";

fn failure_propagates(node: &Value) -> bool {
    match node.get("continue-on-error") {
        None | Some(Value::Bool(false)) => true,
        Some(_) => false,
    }
}

fn unique_named_step<'a>(steps: &'a [Value], name: &str) -> Option<&'a Value> {
    let mut matches = steps
        .iter()
        .filter(|step| step.get("name").and_then(Value::as_str) == Some(name));
    let step = matches.next()?;
    matches.next().is_none().then_some(step)
}

fn routed_load_failure_propagates(source: &str) -> bool {
    let Ok(document) = serde_yaml::from_str::<Value>(source) else {
        return false;
    };
    let Some(job) = document.get("jobs").and_then(|jobs| jobs.get(LOAD_JOB)) else {
        return false;
    };
    if job.get("if").and_then(Value::as_str) != Some(LOAD_JOB_IF) || !failure_propagates(job) {
        return false;
    }

    let Some(steps) = job.get("steps").and_then(Value::as_sequence) else {
        return false;
    };
    let Some(routed_step) = unique_named_step(steps, ROUTED_STEP) else {
        return false;
    };
    if routed_step.get("if").is_some() || !failure_propagates(routed_step) {
        return false;
    }
    let Some(run) = routed_step.get("run").and_then(Value::as_str) else {
        return false;
    };
    if !run.lines().any(|line| line.trim() == ROUTED_K6_LINE) {
        return false;
    }

    let Some(summary_step) = unique_named_step(steps, SUMMARY_STEP) else {
        return false;
    };
    if summary_step.get("if").is_some() || !failure_propagates(summary_step) {
        return false;
    }

    summary_step
        .get("run")
        .and_then(Value::as_str)
        .is_some_and(|run| run.trim() == SUMMARY_RUN)
}

#[test]
fn live_workflow_propagates_routed_load_failure() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        routed_load_failure_propagates(&source),
        "routed k6 threshold and summary-presence failures must fail the admitted load-contract job"
    );
}

#[test]
fn step_continue_on_error_must_not_mask_routed_load_failure() {
    let source = format!(
        r#"
jobs:
  load-contract:
    if: {LOAD_JOB_IF}
    steps:
      - name: Run routed pg-erd loopback traffic
        continue-on-error: true
        run: |
          k6 run --quiet tests/load/pg_erd_gateway_smoke.js
      - name: Require routed pg-erd latency summary
        run: test -s k6-pg-erd-summary.json
"#
    );

    assert!(
        !routed_load_failure_propagates(&source),
        "an evidence-bearing step with continue-on-error=true must not earn GREEN"
    );
}

#[test]
fn job_continue_on_error_must_not_mask_routed_load_failure() {
    let source = format!(
        r#"
jobs:
  load-contract:
    if: {LOAD_JOB_IF}
    continue-on-error: true
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          k6 run --quiet tests/load/pg_erd_gateway_smoke.js
      - name: Require routed pg-erd latency summary
        run: test -s k6-pg-erd-summary.json
"#
    );

    assert!(
        !routed_load_failure_propagates(&source),
        "a load-contract job with continue-on-error=true must not earn GREEN"
    );
}

#[test]
fn conditional_routed_step_must_not_claim_release_evidence() {
    let source = format!(
        r#"
jobs:
  load-contract:
    if: {LOAD_JOB_IF}
    steps:
      - name: Run routed pg-erd loopback traffic
        if: ${{{{ false }}}}
        run: |
          k6 run --quiet tests/load/pg_erd_gateway_smoke.js
      - name: Require routed pg-erd latency summary
        run: test -s k6-pg-erd-summary.json
"#
    );

    assert!(
        !routed_load_failure_propagates(&source),
        "a conditional evidence-bearing step must not claim unconditional release evidence"
    );
}

#[test]
fn skipped_load_job_must_not_claim_routed_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    if: ${{ false }}
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          k6 run --quiet tests/load/pg_erd_gateway_smoke.js
      - name: Require routed pg-erd latency summary
        run: test -s k6-pg-erd-summary.json
"#;

    assert!(
        !routed_load_failure_propagates(source),
        "a skipped load-contract job must not claim routed release evidence"
    );
}

#[test]
fn summary_gate_continue_on_error_must_not_claim_routed_release_evidence() {
    let source = format!(
        r#"
jobs:
  load-contract:
    if: {LOAD_JOB_IF}
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          k6 run --quiet tests/load/pg_erd_gateway_smoke.js
      - name: Require routed pg-erd latency summary
        continue-on-error: true
        run: test -s k6-pg-erd-summary.json
"#
    );

    assert!(
        !routed_load_failure_propagates(&source),
        "a masked summary-presence failure must not retain routed release evidence"
    );
}

#[test]
fn non_executing_shell_must_not_claim_routed_release_evidence() {
    let source = format!(
        r#"
jobs:
  load-contract:
    if: {LOAD_JOB_IF}
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          k6 run --quiet tests/load/pg_erd_gateway_smoke.js
      - name: Require routed pg-erd latency summary
        shell: bash -n {{0}}
        run: test -s k6-pg-erd-summary.json
"#
    );

    assert!(
        !routed_load_failure_propagates(&source),
        "syntax-check-only or otherwise custom shell overrides must not manufacture routed release evidence"
    );
}
