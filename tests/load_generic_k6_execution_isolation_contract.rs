//! Fail-closed contract for the generic loopback k6 execution substrate.
//!
//! The generic load preflight runs in the same required `load-contract` job as routed pg-erd
//! evidence. k6 environment variables outrank checked-in script options, its default disk config can
//! supply non-overlapping runtime options, and anonymous usage reporting introduces an unrelated
//! external network side effect. The preflight therefore clears every inherited `K6_*` name,
//! supplies a known-empty explicit config, and disables usage reporting before measurement.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const GENERIC_STEP: &str = "Exercise concurrent loopback traffic contract";
const EMPTY_CONFIG: &str = "printf '%s\\n' '{}' > /tmp/cwl-k6-generic/config.json";
const K6_COMMAND: &str = "/usr/local/bin/k6 run --no-usage-report --config /tmp/cwl-k6-generic/config.json --quiet tests/load/gateway_smoke.js";
const SANITIZER_START: &str = "while IFS='=' read -r k6_env _; do";
const SANITIZER_MATCH: &str = "K6_*)";
const SANITIZER_UNSET: &str = "unset \"$k6_env\"";

fn generic_run(source: &str) -> Option<String> {
    let document = serde_yaml::from_str::<Value>(source).ok()?;
    let steps = document
        .get("jobs")?
        .get(LOAD_JOB)?
        .get("steps")?
        .as_sequence()?;
    let mut matches = steps
        .iter()
        .filter(|step| step.get("name").and_then(Value::as_str) == Some(GENERIC_STEP));
    let run = matches.next()?.get("run")?.as_str()?.to_owned();
    matches.next().is_none().then_some(run)
}

fn generic_k6_is_isolated(source: &str) -> bool {
    let Some(run) = generic_run(source) else {
        return false;
    };
    let Some(sanitize_index) = run.find(SANITIZER_START) else {
        return false;
    };
    let Some(config_index) = run.find(EMPTY_CONFIG) else {
        return false;
    };
    let Some(k6_index) = run.find(K6_COMMAND) else {
        return false;
    };
    sanitize_index < config_index
        && config_index < k6_index
        && run.contains(SANITIZER_MATCH)
        && run.contains(SANITIZER_UNSET)
        && !run.contains(" k6 run --quiet tests/load/gateway_smoke.js")
}

#[test]
fn live_generic_loopback_k6_is_isolated() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        generic_k6_is_isolated(&source),
        "generic loopback evidence must neutralize inherited k6 authority and external telemetry"
    );
}

#[test]
fn inherited_k6_environment_without_cleanup_is_rejected() {
    let source = format!(
        "jobs:\n  {LOAD_JOB}:\n    steps:\n      - name: {GENERIC_STEP}\n        run: |\n          mkdir -p /tmp/cwl-k6-generic\n          {EMPTY_CONFIG}\n          GATEWAY_URL=http://127.0.0.1:18080 {K6_COMMAND}\n"
    );
    assert!(!generic_k6_is_isolated(&source));
}

#[test]
fn implicit_config_and_default_telemetry_are_rejected() {
    let source = format!(
        "jobs:\n  {LOAD_JOB}:\n    steps:\n      - name: {GENERIC_STEP}\n        run: |\n          {SANITIZER_START}\n            case \"$k6_env\" in {SANITIZER_MATCH} {SANITIZER_UNSET} ;; esac\n          done < <(/usr/bin/env)\n          GATEWAY_URL=http://127.0.0.1:18080 k6 run --quiet tests/load/gateway_smoke.js\n"
    );
    assert!(!generic_k6_is_isolated(&source));
}

#[test]
fn canonical_generic_k6_isolation_is_admitted() {
    let source = format!(
        "jobs:\n  {LOAD_JOB}:\n    steps:\n      - name: {GENERIC_STEP}\n        run: |\n          {SANITIZER_START}\n            case \"$k6_env\" in {SANITIZER_MATCH} {SANITIZER_UNSET} ;; esac\n          done < <(/usr/bin/env)\n          mkdir -p /tmp/cwl-k6-generic\n          {EMPTY_CONFIG}\n          GATEWAY_URL=http://127.0.0.1:18080 {K6_COMMAND}\n"
    );
    assert!(generic_k6_is_isolated(&source));
}
