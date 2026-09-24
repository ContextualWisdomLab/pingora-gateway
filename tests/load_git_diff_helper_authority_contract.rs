//! Fail-closed contract for Git diff helper authority in routed exact-source evidence.
//!
//! The routed evidence step uses `git diff --exit-code` as one part of its exact-source proof.
//! Git permits external diff helpers through environment/configuration and textconv drivers through
//! attributes/configuration. Those helpers are unnecessary for byte-for-byte source provenance and
//! can execute code or substitute comparison semantics. Every routed provenance diff therefore has
//! to disable both external diff and textconv explicitly at the command boundary.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const CANONICAL_DIFF_PREFIX: &str =
    "git --no-replace-objects diff --no-ext-diff --no-textconv --exit-code HEAD --";

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

fn routed_provenance_diffs_are_internal(source: &str) -> bool {
    let Some(run) = routed_run(source) else {
        return false;
    };
    let diffs: Vec<_> = run
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("git --no-replace-objects diff "))
        .collect();

    diffs.len() == 3
        && diffs
            .iter()
            .all(|line| line.starts_with(CANONICAL_DIFF_PREFIX))
}

#[test]
fn live_routed_evidence_disables_git_diff_helpers() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        routed_provenance_diffs_are_internal(&source),
        "routed exact-source provenance must disable external diff and textconv helpers on every git diff"
    );
}

#[test]
fn inherited_external_diff_surface_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          git --no-replace-objects diff --exit-code HEAD -- Cargo.toml\n          git --no-replace-objects diff --exit-code HEAD -- src\n          git --no-replace-objects diff --exit-code HEAD -- tests/load/pg_erd_gateway_smoke.js\n"
    );
    assert!(!routed_provenance_diffs_are_internal(&source));
}

#[test]
fn missing_textconv_disable_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          git --no-replace-objects diff --no-ext-diff --exit-code HEAD -- Cargo.toml\n          git --no-replace-objects diff --no-ext-diff --exit-code HEAD -- src\n          git --no-replace-objects diff --no-ext-diff --exit-code HEAD -- tests/load/pg_erd_gateway_smoke.js\n"
    );
    assert!(!routed_provenance_diffs_are_internal(&source));
}

#[test]
fn all_internal_diff_commands_are_admitted() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {CANONICAL_DIFF_PREFIX} Cargo.toml\n          {CANONICAL_DIFF_PREFIX} src\n          {CANONICAL_DIFF_PREFIX} tests/load/pg_erd_gateway_smoke.js\n"
    );
    assert!(routed_provenance_diffs_are_internal(&source));
}
