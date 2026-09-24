//! Fail-closed contract for Git worktree identity in routed exact-head evidence.
//!
//! Workflow-level working-directory controls are not the only authority over Git's worktree.
//! Repository-local configuration can set `core.worktree`, and linked-worktree metadata can point
//! Git at a different filesystem root while `HEAD` still resolves to the expected commit. Because
//! routed provenance compares Git objects with files Cargo and k6 read from the checkout, the live
//! step must prove Git's own top-level worktree is the GitHub workspace before the first source
//! provenance check.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const WORKTREE_PROOF: &str =
    "test \"$(git --no-replace-objects rev-parse --show-toplevel)\" = \"$GITHUB_WORKSPACE\"";
const FIRST_GIT_PROOF: &str =
    "test \"$(git --no-replace-objects rev-parse HEAD)\" = \"$EXPECTED_SHA\"";
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

fn git_worktree_is_proven_before_source_provenance(source: &str) -> bool {
    let Some(run) = routed_run(source) else {
        return false;
    };
    let lines: Vec<_> = run
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();

    let proofs: Vec<_> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| (*line == WORKTREE_PROOF).then_some(index))
        .collect();
    let Some(first_git_index) = lines.iter().position(|line| *line == FIRST_GIT_PROOF) else {
        return false;
    };
    let Some(clean_index) = lines.iter().position(|line| *line == CARGO_CLEAN) else {
        return false;
    };

    proofs.len() == 1 && proofs[0] < first_git_index && first_git_index < clean_index
}

#[test]
fn live_routed_evidence_proves_git_worktree_identity() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        git_worktree_is_proven_before_source_provenance(&source),
        "routed evidence must prove Git's top-level worktree equals GITHUB_WORKSPACE before exact-source provenance checks"
    );
}

#[test]
fn head_identity_without_worktree_identity_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {FIRST_GIT_PROOF}\n          {CARGO_CLEAN}\n"
    );
    assert!(!git_worktree_is_proven_before_source_provenance(&source));
}

#[test]
fn worktree_proof_after_head_proof_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {FIRST_GIT_PROOF}\n          {WORKTREE_PROOF}\n          {CARGO_CLEAN}\n"
    );
    assert!(!git_worktree_is_proven_before_source_provenance(&source));
}

#[test]
fn worktree_proof_before_head_proof_is_admitted() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {WORKTREE_PROOF}\n          {FIRST_GIT_PROOF}\n          {CARGO_CLEAN}\n"
    );
    assert!(git_worktree_is_proven_before_source_provenance(&source));
}
