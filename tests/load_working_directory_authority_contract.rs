//! Fail-closed contract for GitHub Actions working-directory authority in routed load evidence.
//!
//! Exact command text is not enough when GitHub can change the current directory before a `run`
//! step starts. A step-local `working-directory` can redirect Git/Cargo/source-relative checks away
//! from the exact checkout while leaving the reviewed shell text unchanged. This test-first version
//! covers explicit evidence-step overrides; job/workflow `defaults.run.working-directory` remains
//! the counterexample that the causal repair must close.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const VERIFY_STEP: &str = "Verify checkout identity";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const SUMMARY_STEP: &str = "Require routed pg-erd latency summary";

fn unique_named_step<'a>(steps: &'a [Value], name: &str) -> Option<&'a Value> {
    let mut matches = steps
        .iter()
        .filter(|step| step.get("name").and_then(Value::as_str) == Some(name));
    let step = matches.next()?;
    matches.next().is_none().then_some(step)
}

fn step_overrides_working_directory(step: &Value) -> bool {
    step.get("working-directory").is_some()
}

fn load_evidence_uses_checkout_working_directory(source: &str) -> bool {
    let Ok(document) = serde_yaml::from_str::<Value>(source) else {
        return false;
    };
    let Some(steps) = document
        .get("jobs")
        .and_then(|jobs| jobs.get(LOAD_JOB))
        .and_then(|job| job.get("steps"))
        .and_then(Value::as_sequence)
    else {
        return false;
    };

    [VERIFY_STEP, ROUTED_STEP, SUMMARY_STEP].iter().all(|name| {
        unique_named_step(steps, name).is_some_and(|step| !step_overrides_working_directory(step))
    })
}

#[test]
fn live_routed_load_uses_the_checkout_working_directory() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        load_evidence_uses_checkout_working_directory(&source),
        "exact-head evidence must keep Git, Cargo, k6 and summary checks in the checkout working directory"
    );
}

#[test]
fn routed_step_working_directory_override_must_not_claim_exact_head_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Verify checkout identity
        run: git rev-parse HEAD
      - name: Run routed pg-erd loopback traffic
        working-directory: /tmp/alternate-worktree
        run: cargo build --release --locked
      - name: Require routed pg-erd latency summary
        run: test -s k6-pg-erd-summary.json
"#;

    assert!(!load_evidence_uses_checkout_working_directory(source));
}

#[test]
fn job_default_working_directory_override_must_not_claim_exact_head_evidence() {
    let source = r#"
jobs:
  load-contract:
    defaults:
      run:
        working-directory: /tmp/alternate-worktree
    steps:
      - name: Verify checkout identity
        run: git rev-parse HEAD
      - name: Run routed pg-erd loopback traffic
        run: cargo build --release --locked
      - name: Require routed pg-erd latency summary
        run: test -s k6-pg-erd-summary.json
"#;

    assert!(
        !load_evidence_uses_checkout_working_directory(source),
        "job defaults can redirect every run step without adding a step-local working-directory key"
    );
}
