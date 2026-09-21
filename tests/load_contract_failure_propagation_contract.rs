//! Fail-closed contract for routed-load failure propagation in GitHub Actions.
//!
//! Routed k6 thresholds are release evidence only when the measured step, summary-presence gate,
//! and their enclosing job propagate failures. A green workflow must not be manufactured by
//! `continue-on-error`, skip conditions, duplicate named decoys, non-executing shell overrides,
//! or archived/dead shell text around this evidence path.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const LOAD_JOB_IF: &str = "github.event_name != 'pull_request' || github.event.pull_request.draft == false";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const ROUTED_K6_TAIL: &str = "PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \\\n  k6 run --quiet tests/load/pg_erd_gateway_smoke.js";
const SUMMARY_STEP: &str = "Require routed pg-erd latency summary";
const SUMMARY_RUN: &str = "test -s k6-pg-erd-summary.json";

fn failure_propagates(node: &Value) -> bool {
    match node.get("continue-on-error") {
        None | Some(Value::Bool(false)) => true,
        Some(_) => false,
    }
}

fn run_shell_executes_normally(step: &Value) -> bool {
    match step.get("shell") {
        None => true,
        Some(Value::String(shell)) => shell == "bash",
        Some(_) => false,
    }
}

fn has_run_shell_default(node: &Value) -> bool {
    node.get("defaults")
        .and_then(|defaults| defaults.get("run"))
        .and_then(|run| run.get("shell"))
        .is_some()
}

fn unique_named_step<'a>(steps: &'a [Value], name: &str) -> Option<&'a Value> {
    let mut matches = steps
        .iter()
        .filter(|step| step.get("name").and_then(Value::as_str) == Some(name));
    let step = matches.next()?;
    matches.next().is_none().then_some(step)
}

fn routed_k6_is_terminal_command(run: &str) -> bool {
    run.trim_end().ends_with(ROUTED_K6_TAIL)
}

fn routed_load_failure_propagates(source: &str) -> bool {
    let Ok(document) = serde_yaml::from_str::<Value>(source) else {
        return false;
    };
    if has_run_shell_default(&document) {
        return false;
    }

    let Some(job) = document.get("jobs").and_then(|jobs| jobs.get(LOAD_JOB)) else {
        return false;
    };
    if job.get("if").and_then(Value::as_str) != Some(LOAD_JOB_IF)
        || !failure_propagates(job)
        || has_run_shell_default(job)
    {
        return false;
    }

    let Some(steps) = job.get("steps").and_then(Value::as_sequence) else {
        return false;
    };
    let Some(routed_step) = unique_named_step(steps, ROUTED_STEP) else {
        return false;
    };
    if routed_step.get("if").is_some()
        || !failure_propagates(routed_step)
        || !run_shell_executes_normally(routed_step)
    {
        return false;
    }
    let Some(run) = routed_step.get("run").and_then(Value::as_str) else {
        return false;
    };
    if !routed_k6_is_terminal_command(run) {
        return false;
    }

    let Some(summary_step) = unique_named_step(steps, SUMMARY_STEP) else {
        return false;
    };
    if summary_step.get("if").is_some()
        || !failure_propagates(summary_step)
        || !run_shell_executes_normally(summary_step)
    {
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
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
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
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
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
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
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
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
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
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
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
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
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

#[test]
fn inherited_custom_shell_must_not_claim_routed_release_evidence() {
    let source = format!(
        r#"
jobs:
  load-contract:
    if: {LOAD_JOB_IF}
    defaults:
      run:
        shell: bash -n {{0}}
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            k6 run --quiet tests/load/pg_erd_gateway_smoke.js
      - name: Require routed pg-erd latency summary
        run: test -s k6-pg-erd-summary.json
"#
    );

    assert!(
        !routed_load_failure_propagates(&source),
        "job-level shell defaults must not silently turn the summary gate into non-executing evidence"
    );
}

#[test]
fn routed_k6_inside_false_branch_must_not_claim_release_evidence() {
    let source = format!(
        r#"
jobs:
  load-contract:
    if: {LOAD_JOB_IF}
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          set -euo pipefail
          if false; then
            PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
              k6 run --quiet tests/load/pg_erd_gateway_smoke.js
          fi
          printf '{{}}' > k6-pg-erd-summary.json
      - name: Require routed pg-erd latency summary
        run: test -s k6-pg-erd-summary.json
"#
    );

    assert!(
        !routed_load_failure_propagates(&source),
        "canonical k6 text inside dead shell control-flow is not executed routed-load evidence"
    );
}

#[test]
fn canonical_routed_k6_tail_is_admitted() {
    let source = format!(
        r#"
jobs:
  load-contract:
    if: {LOAD_JOB_IF}
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          set -euo pipefail
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            k6 run --quiet tests/load/pg_erd_gateway_smoke.js
      - name: Require routed pg-erd latency summary
        run: test -s k6-pg-erd-summary.json
"#
    );

    assert!(
        routed_load_failure_propagates(&source),
        "the canonical routed k6 command must remain the terminal command of the failure-propagating step"
    );
}
