//! Fail-closed contract for deterministic vendored OpenSSL build timestamps.
//!
//! OpenSSL 3.6.3 generates `crypto/buildinf.h` from `util/mkbuildinf.pl`; that generator embeds
//! `SOURCE_DATE_EPOCH` when present and otherwise embeds the wall clock. Routed release evidence
//! therefore binds `SOURCE_DATE_EPOCH` to the exact Git commit timestamp before the clean rebuild,
//! replacing inherited runner authority and removing wall-clock drift from the measured candidate.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const CANONICAL_PIN: &str =
    "declare -rx SOURCE_DATE_EPOCH=\"$(git --no-replace-objects show -s --format=%ct \"$EXPECTED_SHA\")\"";
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

fn source_date_epoch_is_bound_to_exact_head_before_rebuild(source: &str) -> bool {
    let Some(run) = routed_run(source) else {
        return false;
    };
    let lines: Vec<_> = run
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    let pins: Vec<_> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| (*line == CANONICAL_PIN).then_some(index))
        .collect();
    let Some(clean_index) = lines.iter().position(|line| *line == CARGO_CLEAN) else {
        return false;
    };
    let Some(build_index) = lines.iter().position(|line| *line == CARGO_BUILD) else {
        return false;
    };

    pins.len() == 1 && pins[0] < clean_index && clean_index < build_index
}

#[test]
fn live_routed_rebuild_binds_source_date_epoch_to_exact_head() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        source_date_epoch_is_bound_to_exact_head_before_rebuild(&source),
        "routed evidence must export SOURCE_DATE_EPOCH from the exact Git commit before rebuilding vendored OpenSSL"
    );
}

#[test]
fn inherited_or_wall_clock_source_date_without_exact_head_pin_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n"
    );
    assert!(!source_date_epoch_is_bound_to_exact_head_before_rebuild(&source));
}

#[test]
fn source_date_pin_after_build_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n          {CANONICAL_PIN}\n"
    );
    assert!(!source_date_epoch_is_bound_to_exact_head_before_rebuild(&source));
}

#[test]
fn merely_unsetting_source_date_epoch_is_not_reproducible() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          unset SOURCE_DATE_EPOCH\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n"
    );
    assert!(!source_date_epoch_is_bound_to_exact_head_before_rebuild(&source));
}

#[test]
fn canonical_exact_head_source_date_pin_before_clean_rebuild_is_admitted() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {CANONICAL_PIN}\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n"
    );
    assert!(source_date_epoch_is_bound_to_exact_head_before_rebuild(&source));
}
