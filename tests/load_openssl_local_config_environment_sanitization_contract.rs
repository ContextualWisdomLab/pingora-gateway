//! Fail-closed contract for inherited OpenSSL local build-configuration authority.
//!
//! The vendored OpenSSL `Configure` script reads `OPENSSL_LOCAL_CONFIG_DIR` and loads matching
//! configuration files and build templates from that directory. Because this can change native
//! build semantics without changing the Rust source or Cargo.lock, routed-load evidence must clear
//! the inherited variable before the point-of-use clean release rebuild.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const CANONICAL_SANITIZE: &str = "unset OPENSSL_LOCAL_CONFIG_DIR";
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

fn active_lines(run: &str) -> Vec<&str> {
    run.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

fn openssl_local_config_is_sanitized_before_rebuild(source: &str) -> bool {
    let Some(run) = routed_run(source) else {
        return false;
    };
    let lines = active_lines(&run);
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
fn live_routed_rebuild_clears_inherited_openssl_local_config_authority() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        openssl_local_config_is_sanitized_before_rebuild(&source),
        "routed evidence must clear inherited OPENSSL_LOCAL_CONFIG_DIR before rebuilding the measured candidate"
    );
}

#[test]
fn clean_rebuild_without_local_config_sanitization_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n"
    );
    assert!(!openssl_local_config_is_sanitized_before_rebuild(&source));
}

#[test]
fn local_config_sanitization_after_build_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n          {CANONICAL_SANITIZE}\n"
    );
    assert!(!openssl_local_config_is_sanitized_before_rebuild(&source));
}

#[test]
fn canonical_local_config_sanitization_before_clean_rebuild_is_admitted() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {CANONICAL_SANITIZE}\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n"
    );
    assert!(openssl_local_config_is_sanitized_before_rebuild(&source));
}
