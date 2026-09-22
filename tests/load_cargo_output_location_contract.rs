//! Fail-closed contract for Cargo output-location authority in routed-load evidence.
//!
//! The routed candidate is started from the canonical host path `target/release/...`. Cargo also
//! accepts environment configuration that can redirect either the target directory or target
//! triple. The evidence contract rejects those declarative redirects so a successful point-of-use
//! build cannot silently land somewhere else while an older canonical-path binary is measured.

use serde_yaml::{Mapping, Value};
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const FORBIDDEN_OUTPUT_ENV_KEYS: &[&str] = &[
    "CARGO_TARGET_DIR",
    "CARGO_BUILD_TARGET_DIR",
    "CARGO_BUILD_TARGET",
];

fn mapping_has_output_redirect(mapping: Option<&Mapping>) -> bool {
    mapping.is_some_and(|mapping| {
        FORBIDDEN_OUTPUT_ENV_KEYS
            .iter()
            .any(|key| mapping.contains_key(Value::String((*key).to_owned())))
    })
}

fn node_has_output_redirect(node: &Value) -> bool {
    mapping_has_output_redirect(node.get("env").and_then(Value::as_mapping))
}

fn routed_candidate_output_location_is_canonical(source: &str) -> bool {
    let Ok(document) = serde_yaml::from_str::<Value>(source) else {
        return false;
    };
    if node_has_output_redirect(&document) {
        return false;
    }

    let Some(job) = document.get("jobs").and_then(|jobs| jobs.get(LOAD_JOB)) else {
        return false;
    };
    if node_has_output_redirect(job) {
        return false;
    }

    let Some(steps) = job.get("steps").and_then(Value::as_sequence) else {
        return false;
    };
    let mut routed_steps = steps
        .iter()
        .filter(|step| step.get("name").and_then(Value::as_str) == Some(ROUTED_STEP));
    let Some(routed) = routed_steps.next() else {
        return false;
    };
    if routed_steps.next().is_some() || node_has_output_redirect(routed) {
        return false;
    }

    true
}

#[test]
fn live_routed_candidate_has_no_declarative_output_redirect() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        routed_candidate_output_location_is_canonical(&source),
        "routed evidence must keep Cargo output on the canonical host target path"
    );
}

#[test]
fn cargo_target_dir_override_is_rejected() {
    let source = r#"
env:
  CARGO_TARGET_DIR: /tmp/redirected-target
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: cargo build --release --locked --bin cwl-pingora-pg-erd-migration
"#;
    assert!(!routed_candidate_output_location_is_canonical(source));
}

#[test]
fn cargo_build_target_dir_alias_is_rejected() {
    let source = r#"
jobs:
  load-contract:
    env:
      CARGO_BUILD_TARGET_DIR: /tmp/redirected-target
    steps:
      - name: Run routed pg-erd loopback traffic
        run: cargo build --release --locked --bin cwl-pingora-pg-erd-migration
"#;
    assert!(!routed_candidate_output_location_is_canonical(source));
}

#[test]
fn cargo_build_target_triple_override_is_rejected() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        env:
          CARGO_BUILD_TARGET: x86_64-unknown-linux-musl
        run: cargo build --release --locked --bin cwl-pingora-pg-erd-migration
"#;
    assert!(!routed_candidate_output_location_is_canonical(source));
}

#[test]
fn process_local_target_dir_redirect_is_rejected() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          CARGO_TARGET_DIR=/tmp/redirected-target cargo clean -p cwl-pingora-gateway --release
          CARGO_TARGET_DIR=/tmp/redirected-target cargo build --release --locked --bin cwl-pingora-pg-erd-migration
          target/release/cwl-pingora-pg-erd-migration --config /tmp/pg-erd-load.yaml
"#;
    assert!(!routed_candidate_output_location_is_canonical(source));
}
