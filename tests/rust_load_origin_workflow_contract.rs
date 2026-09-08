//! Regression contract: measured gateway latency must not include a Python origin.
//!
//! The load fixture is inside the measured request path, so using Python here would
//! contaminate gateway latency attribution even though Python remains acceptable for
//! non-hot-path tooling when no practical Rust alternative exists.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";

/// Parses only the measured load job so unrelated repository tooling is not
/// prohibited from using Python where a Rust replacement would add no value.
fn load_steps() -> Vec<Value> {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    let document: Value =
        serde_yaml::from_str(&source).expect("CI workflow YAML should parse before validation");
    document
        .get("jobs")
        .and_then(|jobs| jobs.get(LOAD_JOB))
        .and_then(|job| job.get("steps"))
        .and_then(Value::as_sequence)
        .expect("load-contract job should define steps")
        .clone()
}

#[test]
fn measured_load_job_uses_only_the_bounded_rust_origin() {
    let steps = load_steps();
    let scripts = steps
        .iter()
        .filter_map(|step| step.get("run").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        scripts.contains("rustfmt --edition 2021 --check tests/load/load_origin.rs"),
        "load lane must format-check the Rust origin before measured traffic"
    );
    assert!(
        scripts.contains("rustc --edition 2021 -D warnings --test tests/load/load_origin.rs"),
        "load lane must run the Rust origin's direct fixture contract"
    );
    assert!(
        scripts.contains("rustc --edition 2021 -D warnings -C opt-level=3 -C debuginfo=0"),
        "measured origin must be optimized Rust rather than an interpreter fixture"
    );
    assert!(
        !scripts.to_ascii_lowercase().contains("python"),
        "Python must not sit anywhere inside the measured load job"
    );
    assert!(
        scripts.contains("/tmp/load_origin >/tmp/upstream-fixture.log 2>&1 &"),
        "generic measured traffic must use the bounded Rust origin"
    );
    assert!(
        scripts.contains(
            "UPSTREAM_PORT=18181 UPSTREAM_PAYLOAD=backend-ok /tmp/load_origin >/tmp/pg-erd-backend.log 2>&1 &"
        ),
        "pg-erd backend measured traffic must use the bounded Rust origin"
    );
    assert!(
        scripts.contains(
            "UPSTREAM_PORT=18183 UPSTREAM_PAYLOAD=frontend-ok /tmp/load_origin >/tmp/pg-erd-frontend.log 2>&1 &"
        ),
        "pg-erd frontend measured traffic must use the bounded Rust origin"
    );
}
