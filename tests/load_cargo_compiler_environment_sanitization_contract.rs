//! Fail-closed contract for inherited Cargo/rustc compiler authority in routed-load evidence.
//!
//! Cargo accepts compiler, wrapper, and flag authority from process environment variables in
//! addition to repository configuration. A clean target directory and a pinned `PATH` do not
//! neutralize inherited `RUSTC*`, `RUSTFLAGS`, or their `CARGO_BUILD_*` aliases. The routed load
//! step therefore clears those inherited authorities before the point-of-use release rebuild.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const CANONICAL_SANITIZE: &str = "unset RUSTC RUSTC_WRAPPER RUSTC_WORKSPACE_WRAPPER RUSTFLAGS CARGO_ENCODED_RUSTFLAGS CARGO_BUILD_RUSTC CARGO_BUILD_RUSTC_WRAPPER CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER CARGO_BUILD_RUSTFLAGS CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER";
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

fn compiler_environment_is_sanitized_before_rebuild(source: &str) -> bool {
    let Some(run) = routed_run(source) else {
        return false;
    };
    let lines = active_lines(&run);
    let sanitize_positions: Vec<_> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| (*line == CANONICAL_SANITIZE).then_some(index))
        .collect();
    if sanitize_positions.len() != 1 {
        return false;
    }
    let Some(clean_index) = lines.iter().position(|line| *line == CARGO_CLEAN) else {
        return false;
    };
    let Some(build_index) = lines.iter().position(|line| *line == CARGO_BUILD) else {
        return false;
    };
    sanitize_positions[0] < clean_index && clean_index < build_index
}

#[test]
fn live_routed_rebuild_clears_inherited_compiler_authority() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        compiler_environment_is_sanitized_before_rebuild(&source),
        "routed evidence must clear inherited Cargo/rustc compiler, wrapper, flag, and target-linker authority before rebuilding the measured candidate"
    );
}

#[test]
fn clean_rebuild_without_environment_sanitization_is_rejected() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          cargo clean --release
          cargo build --release --locked --bin cwl-pingora-pg-erd-migration
"#;
    assert!(!compiler_environment_is_sanitized_before_rebuild(source));
}

#[test]
fn sanitization_after_build_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n          {CANONICAL_SANITIZE}\n"
    );
    assert!(!compiler_environment_is_sanitized_before_rebuild(&source));
}

#[test]
fn canonical_sanitization_before_clean_rebuild_is_admitted() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {CANONICAL_SANITIZE}\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n"
    );
    assert!(compiler_environment_is_sanitized_before_rebuild(&source));
}
