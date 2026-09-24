//! Fail-closed contract for inherited Git repository-location authority in routed evidence.
//!
//! Git documents repository-local environment variables that can redirect the repository, working
//! tree, index, object database, shared administrative directory, or namespace. The routed evidence
//! step uses Git to prove exact HEAD/path content before rebuilding, so those variables must be
//! removed before the first provenance check; otherwise inherited runner state could make the Git
//! proof inspect a different repository surface than the workspace Cargo builds.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const REQUIRED_SANITIZER: &str = "unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_OBJECT_DIRECTORY GIT_ALTERNATE_OBJECT_DIRECTORIES GIT_COMMON_DIR GIT_NAMESPACE";
const FIRST_GIT_PROOF: &str = "test \"$(git --no-replace-objects rev-parse HEAD)\" = \"$EXPECTED_SHA\"";
const CARGO_CLEAN: &str = "cargo clean --release";

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

fn git_repository_environment_is_sanitized_before_provenance(source: &str) -> bool {
    let Some(run) = routed_run(source) else {
        return false;
    };
    let lines: Vec<_> = run
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    let Some(git_index) = lines.iter().position(|line| *line == FIRST_GIT_PROOF) else {
        return false;
    };
    let Some(clean_index) = lines.iter().position(|line| *line == CARGO_CLEAN) else {
        return false;
    };
    let matches: Vec<_> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| (*line == REQUIRED_SANITIZER).then_some(index))
        .collect();
    matches.len() == 1 && matches[0] < git_index && git_index < clean_index
}

#[test]
fn live_routed_evidence_sanitizes_git_repository_environment() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        git_repository_environment_is_sanitized_before_provenance(&source),
        "routed evidence must remove inherited Git repository-location authority before exact-source provenance checks"
    );
}

#[test]
fn missing_git_repository_sanitizer_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {FIRST_GIT_PROOF}\n          {CARGO_CLEAN}\n"
    );
    assert!(!git_repository_environment_is_sanitized_before_provenance(&source));
}

#[test]
fn partial_git_repository_sanitizer_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE\n          {FIRST_GIT_PROOF}\n          {CARGO_CLEAN}\n"
    );
    assert!(!git_repository_environment_is_sanitized_before_provenance(&source));
}

#[test]
fn sanitizer_after_git_proof_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {FIRST_GIT_PROOF}\n          {REQUIRED_SANITIZER}\n          {CARGO_CLEAN}\n"
    );
    assert!(!git_repository_environment_is_sanitized_before_provenance(&source));
}

#[test]
fn exact_sanitizer_before_git_proof_is_admitted() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {REQUIRED_SANITIZER}\n          {FIRST_GIT_PROOF}\n          {CARGO_CLEAN}\n"
    );
    assert!(git_repository_environment_is_sanitized_before_provenance(&source));
}
