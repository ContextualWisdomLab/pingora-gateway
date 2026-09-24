//! Fail-closed contract for inherited OpenSSL `KERNEL_BITS` architecture authority in routed-load evidence.
//!
//! OpenSSL 3.6.3 documents `KERNEL_BITS` as an environment input that may select 32- or 64-bit
//! architecture when Configure cannot infer it unambiguously. The vendored `openssl-src` build
//! launches Configure with inherited process environment, so routed evidence clears `KERNEL_BITS`
//! before the point-of-use clean release rebuild rather than allowing runner state to influence
//! native architecture selection.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const CANONICAL_SANITIZE: &str = "unset KERNEL_BITS";
const CARGO_CLEAN: &str = "cargo clean --release";
const CARGO_BUILD: &str = "cargo build --release --locked --bin cwl-pingora-pg-erd-migration";

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

fn openssl_kernel_bits_environment_is_sanitized_before_rebuild(source: &str) -> bool {
    let Some(run) = routed_run(source) else {
        return false;
    };
    let lines: Vec<_> = run
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    let sanitizers: Vec<_> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| (*line == CANONICAL_SANITIZE).then_some(index))
        .collect();
    let Some(clean_index) = lines.iter().position(|line| *line == CARGO_CLEAN) else {
        return false;
    };
    let Some(build_index) = lines.iter().position(|line| *line == CARGO_BUILD) else {
        return false;
    };

    sanitizers.len() == 1 && sanitizers[0] < clean_index && clean_index < build_index
}

#[test]
fn live_routed_rebuild_clears_inherited_openssl_kernel_bits_authority() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        openssl_kernel_bits_environment_is_sanitized_before_rebuild(&source),
        "routed evidence must clear inherited OpenSSL KERNEL_BITS architecture authority before rebuilding vendored OpenSSL"
    );
}

#[test]
fn inherited_kernel_bits_without_sanitization_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n"
    );
    assert!(!openssl_kernel_bits_environment_is_sanitized_before_rebuild(&source));
}

#[test]
fn kernel_bits_sanitization_after_build_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n          {CANONICAL_SANITIZE}\n"
    );
    assert!(!openssl_kernel_bits_environment_is_sanitized_before_rebuild(&source));
}

#[test]
fn canonical_kernel_bits_sanitization_before_clean_rebuild_is_admitted() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {CANONICAL_SANITIZE}\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n"
    );
    assert!(openssl_kernel_bits_environment_is_sanitized_before_rebuild(&source));
}
