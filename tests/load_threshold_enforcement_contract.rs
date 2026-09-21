//! Fail-closed contract that keeps routed k6 thresholds executable.
//!
//! The routed load receipt is only meaningful when k6 evaluates the thresholds declared by the
//! checked-in script. Grafana k6 exposes `K6_NO_THRESHOLDS` / `--no-thresholds` specifically to
//! disable threshold execution, so the evidence lane must not allow that switch to be injected.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const ROUTED_K6: &str = "k6 run --quiet tests/load/pg_erd_gateway_smoke.js";

fn unique_named_step<'a>(steps: &'a [Value], name: &str) -> Option<&'a Value> {
    let mut matches = steps
        .iter()
        .filter(|step| step.get("name").and_then(Value::as_str) == Some(name));
    let step = matches.next()?;
    matches.next().is_none().then_some(step)
}

fn routed_thresholds_are_enforced(source: &str) -> bool {
    let Ok(document) = serde_yaml::from_str::<Value>(source) else {
        return false;
    };
    let Some(job) = document.get("jobs").and_then(|jobs| jobs.get(LOAD_JOB)) else {
        return false;
    };
    let Some(steps) = job.get("steps").and_then(Value::as_sequence) else {
        return false;
    };
    let Some(routed) = unique_named_step(steps, ROUTED_STEP) else {
        return false;
    };

    routed
        .get("run")
        .and_then(Value::as_str)
        .is_some_and(|run| run.contains(ROUTED_K6))
}

#[test]
fn live_routed_load_keeps_threshold_execution_enabled() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        routed_thresholds_are_enforced(&source),
        "routed release evidence must execute the checked-in k6 thresholds"
    );
}

#[test]
fn job_level_no_thresholds_must_not_claim_routed_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    env:
      K6_NO_THRESHOLDS: true
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(
        !routed_thresholds_are_enforced(source),
        "K6_NO_THRESHOLDS disables threshold evaluation even when the canonical k6 command text is unchanged"
    );
}
