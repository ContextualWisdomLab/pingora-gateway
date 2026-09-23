//! Fail-closed Git index-state contract for routed-load source binding.
//!
//! `git diff HEAD -- <paths>` is only a trustworthy working-tree oracle when Git is required to
//! inspect those paths. `git update-index --assume-unchanged` and `--skip-worktree` can suppress
//! that inspection, allowing a modified fixture, candidate source, or k6 script to appear clean.
//! The routed evidence step therefore clears both index suppression bits immediately before every
//! source comparison that contributes to candidate or workload provenance.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";

const FULL_RESET: &str = "git ls-files -z -- Cargo.toml Cargo.lock src tests/load/load_origin.rs tests/load/pg_erd_gateway_smoke.js | git update-index --no-assume-unchanged --no-skip-worktree -z --stdin";
const FULL_DIFF: &str = "git diff --exit-code HEAD -- Cargo.toml Cargo.lock src tests/load/load_origin.rs tests/load/pg_erd_gateway_smoke.js";
const CANDIDATE_RESET: &str = "git ls-files -z -- Cargo.toml Cargo.lock src | git update-index --no-assume-unchanged --no-skip-worktree -z --stdin";
const CANDIDATE_DIFF: &str = "git diff --exit-code HEAD -- Cargo.toml Cargo.lock src";
const WORKLOAD_RESET: &str = "git ls-files -z -- tests/load/pg_erd_gateway_smoke.js | git update-index --no-assume-unchanged --no-skip-worktree -z --stdin";
const WORKLOAD_DIFF: &str = "git diff --exit-code HEAD -- tests/load/pg_erd_gateway_smoke.js";

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

fn unique_index(lines: &[&str], needle: &str) -> Option<usize> {
    let matches = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| (*line == needle).then_some(index))
        .collect::<Vec<_>>();
    (matches.len() == 1).then_some(matches[0])
}

fn reset_immediately_precedes_diff(lines: &[&str], reset: &str, diff: &str) -> bool {
    let Some(reset_index) = unique_index(lines, reset) else {
        return false;
    };
    let Some(diff_index) = unique_index(lines, diff) else {
        return false;
    };
    diff_index == reset_index + 1
}

fn routed_index_state_is_fail_closed(source: &str) -> bool {
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
    let Some(routed) = unique_named_step(steps, ROUTED_STEP) else {
        return false;
    };
    let Some(run) = routed.get("run").and_then(Value::as_str) else {
        return false;
    };
    let lines = active_lines(run);

    reset_immediately_precedes_diff(&lines, FULL_RESET, FULL_DIFF)
        && reset_immediately_precedes_diff(&lines, CANDIDATE_RESET, CANDIDATE_DIFF)
        && reset_immediately_precedes_diff(&lines, WORKLOAD_RESET, WORKLOAD_DIFF)
}

#[test]
fn live_routed_source_checks_clear_git_index_suppression_bits() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        routed_index_state_is_fail_closed(&source),
        "routed provenance checks must clear assume-unchanged and skip-worktree immediately before every source diff"
    );
}

#[test]
fn diff_without_index_state_reset_must_not_claim_exact_source_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          git diff --exit-code HEAD -- Cargo.toml Cargo.lock src tests/load/load_origin.rs tests/load/pg_erd_gateway_smoke.js
          git diff --exit-code HEAD -- Cargo.toml Cargo.lock src
          git diff --exit-code HEAD -- tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(!routed_index_state_is_fail_closed(source));
}

#[test]
fn assume_unchanged_can_hide_a_modified_candidate_input() {
    let source = format!(
        r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          git update-index --assume-unchanged src/lib.rs
          {FULL_DIFF}
          {CANDIDATE_DIFF}
          {WORKLOAD_DIFF}
"#
    );

    assert!(!routed_index_state_is_fail_closed(&source));
}

#[test]
fn canonical_index_reset_pairs_are_admitted() {
    let source = format!(
        r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          {FULL_RESET}
          {FULL_DIFF}
          {CANDIDATE_RESET}
          {CANDIDATE_DIFF}
          {WORKLOAD_RESET}
          {WORKLOAD_DIFF}
"#
    );

    assert!(routed_index_state_is_fail_closed(&source));
}
