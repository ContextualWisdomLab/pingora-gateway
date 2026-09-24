//! Fail-closed contract for inherited GNU linker runtime-search authority.
//!
//! GNU ld consumes `LD_RUN_PATH` when linking an ELF executable without an explicit `-rpath`.
//! An inherited runner value can therefore inject a runtime library search path into the measured
//! candidate without changing reviewed Rust source, Cargo.lock, or the admitted compiler/toolchain.
//! The routed release rebuild must clear that authority before either the clean or build command.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const SANITIZER: &str = "unset LD_RUN_PATH";
const CLEAN: &str = "cargo clean --release";
const BUILD: &str = "cargo build --release --locked --bin cwl-pingora-pg-erd-migration";

fn unique_named_step<'a>(steps: &'a [Value], name: &str) -> Option<&'a Value> {
    let mut matches = steps
        .iter()
        .filter(|step| step.get("name").and_then(Value::as_str) == Some(name));
    let step = matches.next()?;
    matches.next().is_none().then_some(step)
}

fn active_lines(run: &str) -> Vec<&str> {
    run.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

fn linker_runtime_path_is_sanitized_before_rebuild(run: &str) -> bool {
    let lines = active_lines(run);
    let sanitizer_positions: Vec<_> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| (*line == SANITIZER).then_some(index))
        .collect();
    if sanitizer_positions.len() != 1 {
        return false;
    }

    let Some(clean_index) = lines.iter().position(|line| *line == CLEAN) else {
        return false;
    };
    let Some(build_index) = lines.iter().position(|line| *line == BUILD) else {
        return false;
    };

    sanitizer_positions[0] < clean_index && sanitizer_positions[0] < build_index
}

fn live_linker_runtime_path_is_sanitized(source: &str) -> bool {
    let Ok(document) = serde_yaml::from_str::<Value>(source) else {
        return false;
    };
    let Some(steps) = document
        .get("jobs")
        .and_then(|jobs| jobs.get(LOAD_JOB))
        .and_then(|job| job.get("steps"))
        .and_then(Value::as_sequence)
    else {
        return false;
    };
    let Some(routed) = unique_named_step(steps, ROUTED_STEP) else {
        return false;
    };
    routed
        .get("run")
        .and_then(Value::as_str)
        .is_some_and(linker_runtime_path_is_sanitized_before_rebuild)
}

#[test]
fn live_routed_rebuild_clears_ld_run_path_before_linking() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        live_linker_runtime_path_is_sanitized(&source),
        "routed evidence must clear inherited LD_RUN_PATH before the exact release rebuild"
    );
}

#[test]
fn missing_ld_run_path_sanitizer_is_rejected() {
    let run = r#"
set -euo pipefail
cargo clean --release
cargo build --release --locked --bin cwl-pingora-pg-erd-migration
"#;
    assert!(!linker_runtime_path_is_sanitized_before_rebuild(run));
}

#[test]
fn late_ld_run_path_sanitizer_is_rejected() {
    let run = r#"
set -euo pipefail
cargo clean --release
cargo build --release --locked --bin cwl-pingora-pg-erd-migration
unset LD_RUN_PATH
"#;
    assert!(!linker_runtime_path_is_sanitized_before_rebuild(run));
}

#[test]
fn duplicate_ld_run_path_sanitizer_is_rejected() {
    let run = r#"
unset LD_RUN_PATH
unset LD_RUN_PATH
cargo clean --release
cargo build --release --locked --bin cwl-pingora-pg-erd-migration
"#;
    assert!(!linker_runtime_path_is_sanitized_before_rebuild(run));
}

#[test]
fn canonical_ld_run_path_sanitizer_is_admitted() {
    let run = r#"
unset LD_RUN_PATH
cargo clean --release
cargo build --release --locked --bin cwl-pingora-pg-erd-migration
"#;
    assert!(linker_runtime_path_is_sanitized_before_rebuild(run));
}
