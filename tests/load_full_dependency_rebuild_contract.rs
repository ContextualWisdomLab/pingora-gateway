//! Point-of-use release evidence must rebuild the complete release dependency graph.
//!
//! `cargo clean -p <package> --release` removes only the selected package's release artifacts.
//! The routed measurement runs after an earlier release build in the same job, so package-only
//! cleaning can reuse dependency and build-script artifacts produced before the hardened routed
//! trust root is established. Commercial latency evidence therefore requires a full release clean
//! immediately before the measured candidate build.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const VERIFY_CANDIDATE_SOURCE: &str = "git diff --exit-code HEAD -- Cargo.toml Cargo.lock src";
const FULL_RELEASE_CLEAN: &str = "cargo clean --release";
const BUILD_CANDIDATE: &str = "cargo build --release --locked --bin cwl-pingora-pg-erd-migration";
const START_CANDIDATE: &str = "target/release/cwl-pingora-pg-erd-migration --config /tmp/pg-erd-load.yaml >/tmp/pingora-pg-erd-load.log 2>&1 &";

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

fn routed_candidate_rebuilds_full_release_graph(source: &str) -> bool {
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

    let Some(source_check) = lines.iter().position(|line| *line == VERIFY_CANDIDATE_SOURCE) else {
        return false;
    };
    let Some(clean) = lines.iter().position(|line| *line == FULL_RELEASE_CLEAN) else {
        return false;
    };
    let Some(build) = lines.iter().position(|line| *line == BUILD_CANDIDATE) else {
        return false;
    };
    let Some(start) = lines.iter().position(|line| *line == START_CANDIDATE) else {
        return false;
    };

    clean == source_check + 1 && build == clean + 1 && start == build + 1
}

#[test]
fn live_routed_candidate_rebuilds_the_complete_release_graph() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        routed_candidate_rebuilds_full_release_graph(&source),
        "routed evidence must remove all release artifacts before rebuilding the measured candidate"
    );
}

#[test]
fn package_only_clean_must_not_claim_fresh_dependency_provenance() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          git diff --exit-code HEAD -- Cargo.toml Cargo.lock src
          cargo clean -p cwl-pingora-gateway --release
          cargo build --release --locked --bin cwl-pingora-pg-erd-migration
          target/release/cwl-pingora-pg-erd-migration --config /tmp/pg-erd-load.yaml >/tmp/pingora-pg-erd-load.log 2>&1 &
"#;

    assert!(
        !routed_candidate_rebuilds_full_release_graph(source),
        "package-only clean can reuse release dependencies built before the routed trust root"
    );
}

#[test]
fn full_release_clean_is_admitted() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          git diff --exit-code HEAD -- Cargo.toml Cargo.lock src
          cargo clean --release
          cargo build --release --locked --bin cwl-pingora-pg-erd-migration
          target/release/cwl-pingora-pg-erd-migration --config /tmp/pg-erd-load.yaml >/tmp/pingora-pg-erd-load.log 2>&1 &
"#;

    assert!(routed_candidate_rebuilds_full_release_graph(source));
}
