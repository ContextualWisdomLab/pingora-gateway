//! Raw-byte source binding for routed-load release evidence.
//!
//! Git attributes can attach a configured clean filter to a tracked path. `git diff HEAD` then
//! compares the filtered representation, so a hostile repository-local filter can normalize a
//! modified working-tree file back to the committed blob and manufacture a clean diff. The routed
//! evidence path therefore hashes every admitted working-tree input with `git hash-object
//! --no-filters` and compares it to the exact `HEAD:<path>` blob before the ordinary metadata/mode
//! diff. `--no-filters` is required so attributes and end-of-line conversion cannot redefine the
//! bytes that earn exact-head provenance.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";

const FULL_RAW: &str = "while IFS= read -r -d '' path; do test \"$(git hash-object --no-filters -- \"$path\")\" = \"$(git rev-parse \"HEAD:$path\")\"; done < <(git ls-tree -r --name-only -z HEAD -- Cargo.toml Cargo.lock src tests/load/load_origin.rs tests/load/pg_erd_gateway_smoke.js)";
const FULL_RESET: &str = "git ls-files -z -- Cargo.toml Cargo.lock src tests/load/load_origin.rs tests/load/pg_erd_gateway_smoke.js | git update-index --no-assume-unchanged --no-skip-worktree -z --stdin";
const CANDIDATE_RAW: &str = "while IFS= read -r -d '' path; do test \"$(git hash-object --no-filters -- \"$path\")\" = \"$(git rev-parse \"HEAD:$path\")\"; done < <(git ls-tree -r --name-only -z HEAD -- Cargo.toml Cargo.lock src)";
const CANDIDATE_RESET: &str = "git ls-files -z -- Cargo.toml Cargo.lock src | git update-index --no-assume-unchanged --no-skip-worktree -z --stdin";
const WORKLOAD_RAW: &str = "while IFS= read -r -d '' path; do test \"$(git hash-object --no-filters -- \"$path\")\" = \"$(git rev-parse \"HEAD:$path\")\"; done < <(git ls-tree -r --name-only -z HEAD -- tests/load/pg_erd_gateway_smoke.js)";
const WORKLOAD_RESET: &str = "git ls-files -z -- tests/load/pg_erd_gateway_smoke.js | git update-index --no-assume-unchanged --no-skip-worktree -z --stdin";

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
    if matches.len() == 1 {
        Some(matches[0])
    } else {
        None
    }
}

fn raw_check_precedes_reset(lines: &[&str], raw: &str, reset: &str) -> bool {
    let Some(raw_index) = unique_index(lines, raw) else {
        return false;
    };
    let Some(reset_index) = unique_index(lines, reset) else {
        return false;
    };
    reset_index == raw_index + 1
}

fn routed_raw_source_is_bound(source: &str) -> bool {
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

    raw_check_precedes_reset(&lines, FULL_RAW, FULL_RESET)
        && raw_check_precedes_reset(&lines, CANDIDATE_RAW, CANDIDATE_RESET)
        && raw_check_precedes_reset(&lines, WORKLOAD_RAW, WORKLOAD_RESET)
}

#[test]
fn live_routed_source_binding_includes_raw_blob_identity() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        routed_raw_source_is_bound(&source),
        "routed evidence must compare raw working-tree bytes to exact HEAD blobs before filtered Git diffs"
    );
}

#[test]
fn filtered_diff_without_raw_blob_check_must_not_claim_exact_source_evidence() {
    let source = format!(
        r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          {FULL_RESET}
          git diff --exit-code HEAD -- Cargo.toml Cargo.lock src tests/load/load_origin.rs tests/load/pg_erd_gateway_smoke.js
          {CANDIDATE_RESET}
          git diff --exit-code HEAD -- Cargo.toml Cargo.lock src
          {WORKLOAD_RESET}
          git diff --exit-code HEAD -- tests/load/pg_erd_gateway_smoke.js
"#
    );

    assert!(!routed_raw_source_is_bound(&source));
}

#[test]
fn canonical_raw_blob_checks_are_admitted() {
    let source = format!(
        r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          {FULL_RAW}
          {FULL_RESET}
          {CANDIDATE_RAW}
          {CANDIDATE_RESET}
          {WORKLOAD_RAW}
          {WORKLOAD_RESET}
"#
    );

    assert!(routed_raw_source_is_bound(&source));
}
