//! Fail-closed contract for Git replacement-object authority in routed evidence.
//!
//! `refs/replace/*` can transparently substitute objects for ordinary Git commands while the
//! branch ref itself still names the reviewed commit. Point-of-use source binding therefore uses
//! `git --no-replace-objects` for every Git command in the routed measurement step.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const SAFE_GIT_PREFIX: &str = "git --no-replace-objects ";

fn routed_git_commands_ignore_replace_refs(source: &str) -> bool {
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

    let mut routed_steps = steps.iter().filter(|step| {
        step.get("name").and_then(Value::as_str) == Some(ROUTED_STEP)
    });
    let Some(routed) = routed_steps.next() else {
        return false;
    };
    if routed_steps.next().is_some() {
        return false;
    }
    let Some(run) = routed.get("run").and_then(Value::as_str) else {
        return false;
    };

    let mut saw_git = false;
    for (offset, _) in run.match_indices("git ") {
        saw_git = true;
        if !run[offset..].starts_with(SAFE_GIT_PREFIX) {
            return false;
        }
    }
    saw_git
}

#[test]
fn live_routed_git_commands_ignore_replacement_refs() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        routed_git_commands_ignore_replace_refs(&source),
        "routed exact-head checks must disable refs/replace object substitution"
    );
}

#[test]
fn ordinary_git_object_resolution_must_not_claim_exact_head_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          test "$(git rev-parse HEAD)" = "$EXPECTED_SHA"
          git diff --exit-code HEAD -- Cargo.toml src
"#;

    assert!(
        !routed_git_commands_ignore_replace_refs(source),
        "ordinary Git commands honor refs/replace and can read substituted commit/tree/blob objects"
    );
}

#[test]
fn explicit_no_replace_object_resolution_is_admitted() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          test "$(git --no-replace-objects rev-parse HEAD)" = "$EXPECTED_SHA"
          git --no-replace-objects diff --exit-code HEAD -- Cargo.toml src
"#;

    assert!(routed_git_commands_ignore_replace_refs(source));
}
