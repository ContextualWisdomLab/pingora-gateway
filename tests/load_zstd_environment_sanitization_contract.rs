//! Fail-closed contract for inherited `zstd-sys` package-selection authority.
//!
//! The exact dependency graph contains `zstd-sys` 2.0.16+zstd.1.5.7. Its build script switches
//! from the bundled C source path to system `pkg-config` discovery when
//! `ZSTD_SYS_USE_PKG_CONFIG` is present. The routed rebuild therefore removes that process-level
//! authority before rebuilding the measured candidate.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const CARGO_CLEAN: &str = "cargo clean --release";
const CARGO_BUILD: &str = "cargo build --release --locked --bin cwl-pingora-pg-erd-migration";
const CANONICAL_SANITIZE: &str = "unset ZSTD_SYS_USE_PKG_CONFIG";

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

fn zstd_environment_is_sanitized_before_rebuild(source: &str) -> bool {
    let Some(run) = routed_run(source) else {
        return false;
    };
    let Some(sanitize_index) = run.find(CANONICAL_SANITIZE) else {
        return false;
    };
    if run[sanitize_index + CANONICAL_SANITIZE.len()..].contains(CANONICAL_SANITIZE) {
        return false;
    }
    let Some(clean_index) = run.find(CARGO_CLEAN) else {
        return false;
    };
    let Some(build_index) = run.find(CARGO_BUILD) else {
        return false;
    };
    sanitize_index < clean_index && clean_index < build_index
}

#[test]
fn live_routed_rebuild_neutralizes_inherited_zstd_package_selection() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        zstd_environment_is_sanitized_before_rebuild(&source),
        "routed evidence must clear inherited zstd-sys pkg-config package-selection authority before rebuilding the measured candidate"
    );
}

#[test]
fn clean_rebuild_without_zstd_sanitization_is_rejected() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          cargo clean --release
          cargo build --release --locked --bin cwl-pingora-pg-erd-migration
"#;
    assert!(!zstd_environment_is_sanitized_before_rebuild(source));
}

#[test]
fn zstd_sanitization_after_build_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n          {CANONICAL_SANITIZE}\n"
    );
    assert!(!zstd_environment_is_sanitized_before_rebuild(&source));
}

#[test]
fn canonical_zstd_sanitization_before_clean_rebuild_is_admitted() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {CANONICAL_SANITIZE}\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n"
    );
    assert!(zstd_environment_is_sanitized_before_rebuild(&source));
}
