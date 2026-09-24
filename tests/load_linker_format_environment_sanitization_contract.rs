//! Fail-closed contract for inherited GNU ld format/emulation authority.
//!
//! `GNUTARGET` can select the default BFD input format and `LDEMULATION` can select the default
//! linker emulation. Routed candidate evidence must remove both inherited values before the exact
//! release rebuild so runner state cannot change the linker script or object-format interpretation.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const REQUIRED_SANITIZER: &str = "unset GNUTARGET LDEMULATION";
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

fn linker_format_environment_is_sanitized_before_rebuild(source: &str) -> bool {
    let Some(run) = routed_run(source) else {
        return false;
    };
    let lines: Vec<_> = run
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    let Some(clean_index) = lines.iter().position(|line| *line == CARGO_CLEAN) else {
        return false;
    };
    let Some(build_index) = lines.iter().position(|line| *line == CARGO_BUILD) else {
        return false;
    };
    if clean_index >= build_index {
        return false;
    }

    let matches: Vec<_> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| (*line == REQUIRED_SANITIZER).then_some(index))
        .collect();
    matches.len() == 1 && matches[0] < clean_index
}

#[test]
fn live_routed_rebuild_sanitizes_linker_format_environment() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        linker_format_environment_is_sanitized_before_rebuild(&source),
        "routed evidence must unset inherited GNU ld format/emulation authority before the exact rebuild"
    );
}

#[test]
fn missing_linker_format_sanitizer_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n"
    );
    assert!(!linker_format_environment_is_sanitized_before_rebuild(&source));
}

#[test]
fn partial_linker_format_sanitizer_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          unset GNUTARGET\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n"
    );
    assert!(!linker_format_environment_is_sanitized_before_rebuild(&source));
}

#[test]
fn linker_format_sanitizer_after_build_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n          {REQUIRED_SANITIZER}\n"
    );
    assert!(!linker_format_environment_is_sanitized_before_rebuild(&source));
}

#[test]
fn exact_linker_format_sanitizer_before_clean_rebuild_is_admitted() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {REQUIRED_SANITIZER}\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n"
    );
    assert!(linker_format_environment_is_sanitized_before_rebuild(&source));
}
