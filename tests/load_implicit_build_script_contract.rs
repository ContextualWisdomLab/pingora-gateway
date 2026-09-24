//! Fail-closed contract for Cargo's implicit package build script discovery.
//!
//! This branch has not yet inherited the foundation package-level `build = false` invariant. Cargo
//! can therefore discover an untracked repository-root `build.rs` even though every tracked
//! `Cargo.toml`, `Cargo.lock`, and `src/**` byte matches `HEAD`. Routed candidate evidence must not
//! execute such an input. The point-of-use lane rejects a root `build.rs` before the final raw
//! candidate-source binding. This CI guard is evidence containment, not a substitute for ordinary
//! ancestry integration of the foundation package invariant.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const REJECT_IMPLICIT_BUILD_SCRIPT: &str = "test ! -e build.rs";
const CANDIDATE_RAW: &str = "while IFS= read -r -d '' path; do test \"$(git hash-object --no-filters -- \"$path\")\" = \"$(git rev-parse \"HEAD:$path\")\"; done < <(git ls-tree -r --name-only -z HEAD -- Cargo.toml Cargo.lock src)";

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

fn routed_candidate_rejects_implicit_build_script(source: &str) -> bool {
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
    let Some(reject_index) = unique_index(&lines, REJECT_IMPLICIT_BUILD_SCRIPT) else {
        return false;
    };
    let Some(raw_index) = unique_index(&lines, CANDIDATE_RAW) else {
        return false;
    };
    raw_index == reject_index + 1
}

#[test]
fn live_routed_candidate_rejects_untracked_implicit_build_script() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        routed_candidate_rejects_implicit_build_script(&source),
        "pre-foundation routed evidence must reject a repository-root build.rs before candidate materialization"
    );
}

#[test]
fn tracked_source_checks_without_build_script_guard_are_not_enough() {
    let source = format!(
        r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          {CANDIDATE_RAW}
          git diff --exit-code HEAD -- Cargo.toml Cargo.lock src
          cargo build --release --locked --bin cwl-pingora-pg-erd-migration
"#
    );

    assert!(!routed_candidate_rejects_implicit_build_script(&source));
}

#[test]
fn guard_immediately_before_candidate_raw_binding_is_admitted() {
    let source = format!(
        r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          {REJECT_IMPLICIT_BUILD_SCRIPT}
          {CANDIDATE_RAW}
"#
    );

    assert!(routed_candidate_rejects_implicit_build_script(&source));
}
