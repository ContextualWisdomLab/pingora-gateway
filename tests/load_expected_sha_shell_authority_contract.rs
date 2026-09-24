//! Fail-closed contract for routed-load source identity inside the evidence shell.
//!
//! `EXPECTED_SHA` is workflow-owned pull-request identity. Once Bash starts the routed evidence
//! step, an ordinary shell assignment can otherwise rebind that inherited value before the
//! point-of-use HEAD check and make a different checkout self-consistent. The routed shell must
//! therefore mark the inherited value readonly before any other evidence-bearing command runs.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const STRICT_MODE: &str = "set -euo pipefail";
const LOCK_EXPECTED_SHA: &str = "readonly EXPECTED_SHA";
const VERIFY_HEAD: &str =
    "test \"$(git --no-replace-objects rev-parse HEAD)\" = \"$EXPECTED_SHA\"";

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

fn routed_source_identity_is_immutable(source: &str) -> bool {
    let Ok(document) = serde_yaml::from_str::<Value>(source) else {
        return false;
    };
    let Some(job) = document.get("jobs").and_then(|jobs| jobs.get(LOAD_JOB)) else {
        return false;
    };
    let Some(steps) = job.get("steps").and_then(Value::as_sequence) else {
        return false;
    };
    let Some(routed) = unique_named_step(steps, ROUTED_STEP) else {
        return false;
    };
    let Some(run) = routed.get("run").and_then(Value::as_str) else {
        return false;
    };
    let lines = active_lines(run);

    if lines.first().copied() != Some(STRICT_MODE)
        || lines.get(1).copied() != Some(LOCK_EXPECTED_SHA)
        || lines.iter().filter(|line| **line == LOCK_EXPECTED_SHA).count() != 1
    {
        return false;
    }

    let verify_positions: Vec<_> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| (*line == VERIFY_HEAD).then_some(index))
        .collect();
    verify_positions.len() == 1 && verify_positions[0] > 1
}

#[test]
fn live_routed_shell_locks_workflow_owned_expected_sha_before_other_commands() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        routed_source_identity_is_immutable(&source),
        "routed evidence must make the workflow-owned EXPECTED_SHA readonly before other shell commands"
    );
}

#[test]
fn local_expected_sha_rebinding_before_lock_must_not_claim_exact_head_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          set -euo pipefail
          EXPECTED_SHA="$(git --no-replace-objects rev-parse HEAD)"
          readonly EXPECTED_SHA
          test "$(git --no-replace-objects rev-parse HEAD)" = "$EXPECTED_SHA"
"#;

    assert!(
        !routed_source_identity_is_immutable(source),
        "locking EXPECTED_SHA after a local rebind preserves the attacker-selected identity"
    );
}

#[test]
fn canonical_early_readonly_binding_is_admitted() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          set -euo pipefail
          readonly EXPECTED_SHA
          readonly PATH=/usr/bin:/bin
          test "$(git --no-replace-objects rev-parse HEAD)" = "$EXPECTED_SHA"
"#;

    assert!(routed_source_identity_is_immutable(source));
}
