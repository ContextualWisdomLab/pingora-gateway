//! Fail-closed contract for inherited Git pathspec authority in routed evidence.
//!
//! Git supports `GIT_LITERAL_PATHSPECS`, `GIT_GLOB_PATHSPECS`, `GIT_NOGLOB_PATHSPECS`, and
//! `GIT_ICASE_PATHSPECS` as process-wide pathspec interpretation controls. The routed evidence step
//! uses path-limited `ls-tree`, `ls-files`, and `diff` commands to prove the exact source surface that
//! is rebuilt and measured. Inherited runner pathspec policy must therefore be removed before Git is
//! used as a provenance oracle; otherwise the same command text can select a different path set.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const REQUIRED_SANITIZER: &str =
    "unset GIT_LITERAL_PATHSPECS GIT_GLOB_PATHSPECS GIT_NOGLOB_PATHSPECS GIT_ICASE_PATHSPECS";
const FIRST_GIT_PROOF: &str =
    "test \"$(git --no-replace-objects rev-parse HEAD)\" = \"$EXPECTED_SHA\"";

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

fn git_pathspec_environment_is_sanitized_before_provenance(source: &str) -> bool {
    let Some(run) = routed_run(source) else {
        return false;
    };
    let lines: Vec<_> = run
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    let Some(proof_index) = lines.iter().position(|line| *line == FIRST_GIT_PROOF) else {
        return false;
    };
    let matches: Vec<_> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| (*line == REQUIRED_SANITIZER).then_some(index))
        .collect();
    matches.len() == 1 && matches[0] < proof_index
}

#[test]
fn live_routed_evidence_sanitizes_git_pathspec_environment() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        git_pathspec_environment_is_sanitized_before_provenance(&source),
        "routed exact-source provenance must neutralize inherited Git pathspec interpretation authority before Git is used as an oracle"
    );
}

#[test]
fn missing_git_pathspec_sanitizer_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {FIRST_GIT_PROOF}\n"
    );
    assert!(!git_pathspec_environment_is_sanitized_before_provenance(&source));
}

#[test]
fn partial_git_pathspec_sanitizer_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          unset GIT_LITERAL_PATHSPECS GIT_GLOB_PATHSPECS GIT_NOGLOB_PATHSPECS\n          {FIRST_GIT_PROOF}\n"
    );
    assert!(!git_pathspec_environment_is_sanitized_before_provenance(&source));
}

#[test]
fn sanitizer_after_git_proof_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {FIRST_GIT_PROOF}\n          {REQUIRED_SANITIZER}\n"
    );
    assert!(!git_pathspec_environment_is_sanitized_before_provenance(&source));
}

#[test]
fn exact_git_pathspec_sanitizer_before_proof_is_admitted() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {REQUIRED_SANITIZER}\n          {FIRST_GIT_PROOF}\n"
    );
    assert!(git_pathspec_environment_is_sanitized_before_provenance(&source));
}
