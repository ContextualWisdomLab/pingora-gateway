//! Fail-closed contract for k6 telemetry during routed loopback evidence.
//!
//! k6 sends an anonymous usage report by default. The routed benchmark is intended to exercise
//! only the local Rust origins and the candidate gateway; unrelated outbound telemetry adds an
//! external network side effect and avoidable timing/noise authority to that evidence path. The
//! checksum-proven k6 invocation therefore disables usage reporting explicitly at the CLI layer.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const CANONICAL_K6: &str = "/usr/local/bin/k6 run --no-usage-report --config /tmp/cwl-k6-routed/config.json --quiet tests/load/pg_erd_gateway_smoke.js";
const TELEMETRY_ENABLED_K6: &str = "/usr/local/bin/k6 run --config /tmp/cwl-k6-routed/config.json --quiet tests/load/pg_erd_gateway_smoke.js";

fn routed_run(source: &str) -> Option<String> {
    let document = serde_yaml::from_str::<Value>(source).ok()?;
    let steps = document
        .get("jobs")?
        .get(LOAD_JOB)?
        .get("steps")?
        .as_sequence()?;
    let mut matches = steps
        .iter()
        .filter(|step| step.get("name").and_then(Value::as_str) == Some(ROUTED_STEP));
    let run = matches.next()?.get("run")?.as_str()?.to_owned();
    matches.next().is_none().then_some(run)
}

fn routed_k6_disables_usage_reporting(source: &str) -> bool {
    let Some(run) = routed_run(source) else {
        return false;
    };
    let Some(command_index) = run.find(CANONICAL_K6) else {
        return false;
    };
    !run[command_index + CANONICAL_K6.len()..].contains(CANONICAL_K6)
        && !run.contains(TELEMETRY_ENABLED_K6)
}

#[test]
fn live_routed_load_disables_k6_usage_reporting() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        routed_k6_disables_usage_reporting(&source),
        "routed evidence must not perform k6's default external usage-report request"
    );
}

#[test]
fn default_usage_reporting_is_rejected() {
    let source = format!(
        "jobs:\n  {LOAD_JOB}:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {TELEMETRY_ENABLED_K6}\n"
    );
    assert!(!routed_k6_disables_usage_reporting(&source));
}

#[test]
fn environment_only_disable_is_rejected() {
    let source = format!(
        "jobs:\n  {LOAD_JOB}:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          K6_NO_USAGE_REPORT=true {TELEMETRY_ENABLED_K6}\n"
    );
    assert!(
        !routed_k6_disables_usage_reporting(&source),
        "the routed step clears inherited K6_* state; telemetry authority must be explicit on the admitted CLI"
    );
}

#[test]
fn canonical_no_usage_report_invocation_is_admitted() {
    let source = format!(
        "jobs:\n  {LOAD_JOB}:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {CANONICAL_K6}\n"
    );
    assert!(routed_k6_disables_usage_reporting(&source));
}
