//! Fail-closed routed-load contract for indirect environment mutation.
//!
//! Literal `K6_*` scans are insufficient when Bash reconstructs an option name at runtime and
//! exports it before invoking k6. This contract keeps the routed evidence shell free of such
//! environment-mutation primitives so threshold and workload options cannot be injected without a
//! literal `K6_*` token.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";

fn unique_named_step<'a>(steps: &'a [Value], name: &str) -> Option<&'a Value> {
    let mut matches = steps
        .iter()
        .filter(|step| step.get("name").and_then(Value::as_str) == Some(name));
    let step = matches.next()?;
    matches.next().is_none().then_some(step)
}

fn active_lines(run: &str) -> impl Iterator<Item = &str> {
    run.lines()
        .map(str::trim_start)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
}

fn routed_shell_environment_is_stable(run: &str) -> bool {
    active_lines(run).all(|line| !line.contains("K6_"))
}

fn live_routed_shell_environment_is_stable(source: &str) -> bool {
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
        .is_some_and(routed_shell_environment_is_stable)
}

#[test]
fn live_routed_shell_does_not_mutate_k6_environment_indirectly() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        live_routed_shell_environment_is_stable(&source),
        "routed evidence must not reconstruct and export k6 option names at runtime"
    );
}

#[test]
fn reconstructed_no_thresholds_export_must_not_claim_routed_release_evidence() {
    let run = r#"
set -euo pipefail
option_prefix=K6
option_name="${option_prefix}_NO_THRESHOLDS"
declare -x "${option_name}=true"
PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
  /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(
        !routed_shell_environment_is_stable(run),
        "a runtime-reconstructed K6_NO_THRESHOLDS export disables threshold evaluation without any literal K6_* token"
    );
}
