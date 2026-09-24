//! Fail-closed contract for k6's implicit on-disk configuration authority.
//!
//! k6 loads an on-disk configuration file before script options unless an explicit configuration
//! path is supplied. Script-level workload fields only override overlapping keys, so an inherited
//! user config can still change other request/runtime behavior and make routed latency evidence
//! depend on mutable runner state. The measured run therefore supplies a known-empty JSON config
//! created in the routed evidence step and passes it explicitly to the checksum-proven k6 binary.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const EMPTY_CONFIG: &str = "printf '%s\\n' '{}' > /tmp/cwl-k6-routed/config.json";
const ROUTED_K6: &str = "/usr/local/bin/k6 run --config /tmp/cwl-k6-routed/config.json --quiet tests/load/pg_erd_gateway_smoke.js";
const IMPLICIT_CONFIG_K6: &str = "/usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js";

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

fn routed_k6_uses_known_empty_explicit_config(source: &str) -> bool {
    let Some(run) = routed_run(source) else {
        return false;
    };
    let Some(config_index) = run.find(EMPTY_CONFIG) else {
        return false;
    };
    if run[config_index + EMPTY_CONFIG.len()..].contains(EMPTY_CONFIG) {
        return false;
    }
    let Some(k6_index) = run.find(ROUTED_K6) else {
        return false;
    };
    if run[k6_index + ROUTED_K6.len()..].contains(ROUTED_K6) {
        return false;
    }
    config_index < k6_index && !run.contains(IMPLICIT_CONFIG_K6)
}

#[test]
fn live_routed_load_uses_known_empty_explicit_k6_config() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        routed_k6_uses_known_empty_explicit_config(&source),
        "routed evidence must create a known-empty k6 config and pass it explicitly before measurement"
    );
}

#[test]
fn implicit_runner_config_is_rejected() {
    let source = format!(
        "jobs:\n  {LOAD_JOB}:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {IMPLICIT_CONFIG_K6}\n"
    );
    assert!(!routed_k6_uses_known_empty_explicit_config(&source));
}

#[test]
fn explicit_config_created_after_measurement_is_rejected() {
    let source = format!(
        "jobs:\n  {LOAD_JOB}:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {ROUTED_K6}\n          {EMPTY_CONFIG}\n"
    );
    assert!(!routed_k6_uses_known_empty_explicit_config(&source));
}

#[test]
fn nonempty_unreviewed_config_is_rejected() {
    let source = format!(
        "jobs:\n  {LOAD_JOB}:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          printf '%s\\n' '{{\"noConnectionReuse\":true}}' > /tmp/cwl-k6-routed/config.json\n          {ROUTED_K6}\n"
    );
    assert!(!routed_k6_uses_known_empty_explicit_config(&source));
}

#[test]
fn canonical_explicit_empty_config_is_admitted() {
    let source = format!(
        "jobs:\n  {LOAD_JOB}:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {EMPTY_CONFIG}\n          {ROUTED_K6}\n"
    );
    assert!(routed_k6_uses_known_empty_explicit_config(&source));
}
