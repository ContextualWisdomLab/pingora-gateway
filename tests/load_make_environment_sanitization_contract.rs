//! Fail-closed contract for inherited GNU make control authority in routed-load evidence.
//!
//! Vendored OpenSSL is built by `openssl-src` through `make depend` and `make build_libs`.
//! GNU make interprets `MAKEFLAGS`, legacy `MFLAGS`, `GNUMAKEFLAGS`, and `MAKEFILES` from the
//! inherited environment; in particular `MAKEFILES` can load additional makefiles before the
//! normal build and option variables can alter make semantics. Routed evidence therefore clears
//! these process inputs before the point-of-use clean release rebuild.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const CANONICAL_SANITIZE: &str = "unset MAKEFLAGS MFLAGS GNUMAKEFLAGS MAKEFILES";
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

fn make_environment_is_sanitized_before_rebuild(source: &str) -> bool {
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
fn live_routed_rebuild_clears_inherited_make_control_authority() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        make_environment_is_sanitized_before_rebuild(&source),
        "routed evidence must clear inherited GNU make control/file-injection authority before rebuilding vendored OpenSSL"
    );
}

#[test]
fn makefiles_injection_without_sanitization_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n"
    );
    assert!(!make_environment_is_sanitized_before_rebuild(&source));
}

#[test]
fn make_sanitization_after_build_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n          {CANONICAL_SANITIZE}\n"
    );
    assert!(!make_environment_is_sanitized_before_rebuild(&source));
}

#[test]
fn canonical_make_sanitization_before_clean_rebuild_is_admitted() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {CANONICAL_SANITIZE}\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n"
    );
    assert!(make_environment_is_sanitized_before_rebuild(&source));
}
