//! Fail-closed contract for inherited Cargo jobserver/make flag authority.
//!
//! Exact `openssl-src` forwards inherited `CARGO_MAKEFLAGS` into the `MAKEFLAGS` environment of
//! its non-Windows `make build_libs` child. Clearing `MAKEFLAGS` alone is therefore insufficient:
//! Cargo runner state can be reintroduced after the point-of-use sanitizer. Routed evidence clears
//! `CARGO_MAKEFLAGS` before the clean release rebuild so vendored OpenSSL make semantics stay owned.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const CANONICAL_SANITIZE: &str = "unset CARGO_MAKEFLAGS";
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

fn cargo_makeflags_are_sanitized_before_rebuild(source: &str) -> bool {
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
fn live_routed_rebuild_clears_inherited_cargo_makeflags() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        cargo_makeflags_are_sanitized_before_rebuild(&source),
        "routed evidence must clear inherited CARGO_MAKEFLAGS before openssl-src can forward it into make"
    );
}

#[test]
fn inherited_cargo_makeflags_without_sanitization_are_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n"
    );
    assert!(!cargo_makeflags_are_sanitized_before_rebuild(&source));
}

#[test]
fn cargo_makeflags_sanitization_after_build_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n          {CANONICAL_SANITIZE}\n"
    );
    assert!(!cargo_makeflags_are_sanitized_before_rebuild(&source));
}

#[test]
fn clearing_makeflags_without_cargo_makeflags_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          unset MAKEFLAGS MFLAGS GNUMAKEFLAGS MAKEFILES\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n"
    );
    assert!(!cargo_makeflags_are_sanitized_before_rebuild(&source));
}

#[test]
fn canonical_cargo_makeflags_sanitization_before_clean_rebuild_is_admitted() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {CANONICAL_SANITIZE}\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n"
    );
    assert!(cargo_makeflags_are_sanitized_before_rebuild(&source));
}
