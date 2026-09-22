//! Point-of-use source and candidate binding for routed load evidence.
//!
//! Checkout identity alone does not prove that the measured fixture, migration binary, or k6
//! script still correspond to the checked-out head: an earlier step can mutate tracked source or
//! replace a generated executable after checkout verification. The routed measurement therefore
//! re-verifies HEAD, rejects index and working-tree drift from HEAD, rebuilds the Rust fixture,
//! cleans the package's release artifacts, and rebuilds the migration binary before it starts
//! measured traffic.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const VERIFY_HEAD: &str = "test \"$(git rev-parse HEAD)\" = \"$EXPECTED_SHA\"";
const VERIFY_SOURCE: &str = "git diff --exit-code HEAD -- Cargo.toml Cargo.lock src tests/load/load_origin.rs tests/load/pg_erd_gateway_smoke.js";
const REBUILD_ORIGIN: &str = "rustc --edition 2021 -D warnings -C opt-level=3 -C debuginfo=0 --out-dir /tmp tests/load/load_origin.rs";
const CLEAN_RELEASE: &str = "cargo clean -p cwl-pingora-gateway --release";
const REBUILD_CANDIDATE: &str =
    "cargo build --release --locked --bin cwl-pingora-pg-erd-migration";
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

fn ordered_once(lines: &[&str], needle: &str, after: Option<usize>) -> Option<usize> {
    let matches: Vec<_> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| (*line == needle).then_some(index))
        .collect();
    if matches.len() != 1 {
        return None;
    }
    let index = matches[0];
    after.is_none_or(|previous| previous < index).then_some(index)
}

fn routed_point_of_use_is_bound(source: &str) -> bool {
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

    let Some(head) = ordered_once(&lines, VERIFY_HEAD, None) else {
        return false;
    };
    let Some(source) = ordered_once(&lines, VERIFY_SOURCE, Some(head)) else {
        return false;
    };
    let Some(origin) = ordered_once(&lines, REBUILD_ORIGIN, Some(source)) else {
        return false;
    };
    let Some(clean) = ordered_once(&lines, CLEAN_RELEASE, Some(origin)) else {
        return false;
    };
    let Some(candidate) = ordered_once(&lines, REBUILD_CANDIDATE, Some(clean)) else {
        return false;
    };
    ordered_once(&lines, START_CANDIDATE, Some(candidate)).is_some()
}

#[test]
fn live_routed_measurement_rebuilds_from_clean_exact_head() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        routed_point_of_use_is_bound(&source),
        "routed evidence must re-check tracked source and freshly rebuild its Rust fixture and candidate before measured traffic"
    );
}

#[test]
fn checkout_only_is_not_point_of_use_source_binding() {
    let source = r#"
env:
  EXPECTED_SHA: deadbeef
jobs:
  load-contract:
    steps:
      - name: Checkout exact revision
        run: test \"$(git rev-parse HEAD)\" = \"$EXPECTED_SHA\"
      - name: Mutate measured script after checkout
        run: printf 'tampered' > tests/load/pg_erd_gateway_smoke.js
      - name: Run routed pg-erd loopback traffic
        run: |
          target/release/cwl-pingora-pg-erd-migration --config /tmp/pg-erd-load.yaml >/tmp/pingora-pg-erd-load.log 2>&1 &
"#;
    assert!(!routed_point_of_use_is_bound(source));
}

#[test]
fn index_relative_diff_is_not_exact_head_binding() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          test "$(git rev-parse HEAD)" = "$EXPECTED_SHA"
          git diff --exit-code -- Cargo.toml Cargo.lock src tests/load/load_origin.rs tests/load/pg_erd_gateway_smoke.js
          rustc --edition 2021 -D warnings -C opt-level=3 -C debuginfo=0 --out-dir /tmp tests/load/load_origin.rs
          cargo clean -p cwl-pingora-gateway --release
          cargo build --release --locked --bin cwl-pingora-pg-erd-migration
          target/release/cwl-pingora-pg-erd-migration --config /tmp/pg-erd-load.yaml >/tmp/pingora-pg-erd-load.log 2>&1 &
"#;
    assert!(
        !routed_point_of_use_is_bound(source),
        "working-tree-to-index diff can miss a staged mutation and must not substitute for HEAD binding"
    );
}

#[test]
fn clean_source_without_fresh_candidate_rebuild_is_not_enough() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          test "$(git rev-parse HEAD)" = "$EXPECTED_SHA"
          git diff --exit-code HEAD -- Cargo.toml Cargo.lock src tests/load/load_origin.rs tests/load/pg_erd_gateway_smoke.js
          rustc --edition 2021 -D warnings -C opt-level=3 -C debuginfo=0 --out-dir /tmp tests/load/load_origin.rs
          target/release/cwl-pingora-pg-erd-migration --config /tmp/pg-erd-load.yaml >/tmp/pingora-pg-erd-load.log 2>&1 &
"#;
    assert!(!routed_point_of_use_is_bound(source));
}

#[test]
fn stale_fixture_with_fresh_candidate_is_not_enough() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          test "$(git rev-parse HEAD)" = "$EXPECTED_SHA"
          git diff --exit-code HEAD -- Cargo.toml Cargo.lock src tests/load/load_origin.rs tests/load/pg_erd_gateway_smoke.js
          cargo clean -p cwl-pingora-gateway --release
          cargo build --release --locked --bin cwl-pingora-pg-erd-migration
          target/release/cwl-pingora-pg-erd-migration --config /tmp/pg-erd-load.yaml >/tmp/pingora-pg-erd-load.log 2>&1 &
"#;
    assert!(!routed_point_of_use_is_bound(source));
}
